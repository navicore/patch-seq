//! Signal number constants exposed to Seq as `signal.SIGINT`, `signal.SIGTERM`,
//! etc. Unix builds return the real libc value; non-Unix builds return 0 so the
//! FFI shape stays the same.

use crate::stack::{Stack, push};
use crate::value::Value;

/// Get SIGINT constant (Ctrl+C interrupt)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigint(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGINT as i64)) }
}

/// Get SIGTERM constant (termination request)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigterm(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGTERM as i64)) }
}

/// Get SIGHUP constant (hangup)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sighup(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGHUP as i64)) }
}

/// Get SIGPIPE constant (broken pipe)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigpipe(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGPIPE as i64)) }
}

/// Get SIGUSR1 constant (user signal 1)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigusr1(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGUSR1 as i64)) }
}

/// Get SIGUSR2 constant (user signal 2)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigusr2(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGUSR2 as i64)) }
}

/// Get SIGCHLD constant (child status change)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigchld(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGCHLD as i64)) }
}

/// Get SIGALRM constant (alarm clock)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigalrm(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGALRM as i64)) }
}

/// Get SIGCONT constant (continue)
///
/// Stack effect: ( -- Int )
///
/// # Safety
/// Stack pointer must be valid.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigcont(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(libc::SIGCONT as i64)) }
}

// Non-Unix stubs for signal constants (return 0)
// Safety: Stack pointer must be valid for all functions below.

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigint(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigterm(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sighup(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigpipe(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigusr1(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigusr2(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigchld(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigalrm(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}

/// # Safety
/// Stack pointer must be valid.
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_signal_sigcont(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(0)) }
}
