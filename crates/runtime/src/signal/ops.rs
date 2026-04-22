//! Seq-callable signal operations: `signal.trap`, `signal.received?`,
//! `signal.pending?`, `signal.default`, `signal.ignore`, `signal.clear`.
//!
//! Unix implementations drive sigaction through `super::handlers`; non-Unix
//! builds ship stubs that preserve the FFI shape but no-op.

use crate::stack::{Stack, pop, push};
use crate::value::Value;

#[cfg(unix)]
use std::sync::atomic::Ordering;

#[cfg(unix)]
use super::{MAX_SIGNAL, SIGNAL_FLAGS};

/// Trap a signal: install handler that sets flag instead of default behavior
///
/// Stack effect: ( signal-num -- )
///
/// After trapping, the signal will set an internal flag instead of its default
/// action (which might be to terminate the process). Use `signal.received?` to
/// check and clear the flag.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_trap(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.trap: invalid signal number {}", n);
                }
                n as libc::c_int
            }
            _ => panic!(
                "signal.trap: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        // Install our flag-setting handler using sigaction
        if let Err(e) = super::handlers::install_signal_handler(sig_num) {
            panic!(
                "signal.trap: failed to install handler for signal {}: {}",
                sig_num, e
            );
        }
        stack
    }
}

/// Check if a signal was received and clear the flag
///
/// Stack effect: ( signal-num -- received? )
///
/// Returns true if the signal was received since the last check, false otherwise.
/// This atomically clears the flag, so the signal must be received again to return true.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_received(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.received?: invalid signal number {}", n);
                }
                n as usize
            }
            _ => panic!(
                "signal.received?: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        // Atomically swap the flag to false and return the old value
        let was_set = SIGNAL_FLAGS[sig_num].swap(false, Ordering::Acquire);
        push(stack, Value::Bool(was_set))
    }
}

/// Check if a signal is pending without clearing the flag
///
/// Stack effect: ( signal-num -- pending? )
///
/// Returns true if the signal was received, false otherwise.
/// Unlike `signal.received?`, this does NOT clear the flag.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_pending(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.pending?: invalid signal number {}", n);
                }
                n as usize
            }
            _ => panic!(
                "signal.pending?: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        let is_set = SIGNAL_FLAGS[sig_num].load(Ordering::Acquire);
        push(stack, Value::Bool(is_set))
    }
}

/// Restore the default handler for a signal
///
/// Stack effect: ( signal-num -- )
///
/// Restores the system default behavior for the signal.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_default(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.default: invalid signal number {}", n);
                }
                n as libc::c_int
            }
            _ => panic!(
                "signal.default: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        if let Err(e) = super::handlers::restore_default_handler(sig_num) {
            panic!(
                "signal.default: failed to restore default handler for signal {}: {}",
                sig_num, e
            );
        }
        stack
    }
}

/// Ignore a signal entirely
///
/// Stack effect: ( signal-num -- )
///
/// The signal will be ignored - it won't terminate the process or set any flag.
/// Useful for SIGPIPE in network servers.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_ignore(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.ignore: invalid signal number {}", n);
                }
                n as libc::c_int
            }
            _ => panic!(
                "signal.ignore: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        if let Err(e) = super::handlers::ignore_signal(sig_num) {
            panic!("signal.ignore: failed to ignore signal {}: {}", sig_num, e);
        }
        stack
    }
}

/// Clear the flag for a signal without checking it
///
/// Stack effect: ( signal-num -- )
///
/// Useful for resetting state without caring about the previous value.
///
/// # Safety
/// Stack must have an Int (signal number) on top
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_clear(stack: Stack) -> Stack {
    unsafe {
        let (stack, sig_val) = pop(stack);
        let sig_num = match sig_val {
            Value::Int(n) => {
                if n < 0 || n as usize >= MAX_SIGNAL {
                    panic!("signal.clear: invalid signal number {}", n);
                }
                n as usize
            }
            _ => panic!(
                "signal.clear: expected Int (signal number), got {:?}",
                sig_val
            ),
        };

        SIGNAL_FLAGS[sig_num].store(false, Ordering::Release);
        stack
    }
}

// Stub implementations for non-Unix platforms
// Safety: Stack pointer must be valid for all functions below.

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_trap(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    // No-op on non-Unix - signals not supported
    stack
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_default(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    stack
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_ignore(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    stack
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_received(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    // Always return false on non-Unix - signals not supported
    unsafe { push(stack, Value::Bool(false)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_pending(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    // Always return false on non-Unix - signals not supported
    unsafe { push(stack, Value::Bool(false)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_clear(stack: Stack) -> Stack {
    let (stack, _) = unsafe { pop(stack) };
    // No-op on non-Unix
    stack
}
