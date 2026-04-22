//! Async-signal-safe flag handler and sigaction-based installation helpers.
//!
//! Unix-only. The handler runs in an interrupt context and does nothing but
//! flip a single atomic flag in `super::SIGNAL_FLAGS`. All user-visible
//! operations (`trap`, `default`, `ignore`) route through the three helpers
//! here so signal-disposition changes serialize on a single mutex.

use std::mem::MaybeUninit;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

use super::{MAX_SIGNAL, SIGNAL_FLAGS};

/// Mutex to protect signal handler installation from concurrent access
static SIGNAL_MUTEX: Mutex<()> = Mutex::new(());

/// Signal handler that just sets the atomic flag
/// This is async-signal-safe: only uses atomic operations
extern "C" fn flag_signal_handler(sig: libc::c_int) {
    let sig_num = sig as usize;
    if sig_num < MAX_SIGNAL {
        SIGNAL_FLAGS[sig_num].store(true, Ordering::Release);
    }
}

/// Install a signal handler using sigaction (thread-safe)
///
/// Uses sigaction() instead of signal() for:
/// - Well-defined semantics across platforms
/// - Thread safety with strands
/// - SA_RESTART to automatically restart interrupted syscalls
pub(super) fn install_signal_handler(sig_num: libc::c_int) -> Result<(), std::io::Error> {
    let _guard = SIGNAL_MUTEX
        .lock()
        .expect("signal: mutex poisoned during handler installation");

    unsafe {
        let mut action: libc::sigaction = MaybeUninit::zeroed().assume_init();
        // Use sa_handler (not sa_sigaction) since we're not using SA_SIGINFO
        action.sa_sigaction = flag_signal_handler as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);

        let result = libc::sigaction(sig_num, &action, std::ptr::null_mut());
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Restore default signal handler using sigaction (thread-safe)
pub(super) fn restore_default_handler(sig_num: libc::c_int) -> Result<(), std::io::Error> {
    let _guard = SIGNAL_MUTEX
        .lock()
        .expect("signal: mutex poisoned during handler restoration");

    unsafe {
        let mut action: libc::sigaction = MaybeUninit::zeroed().assume_init();
        // Use SIG_DFL to restore default handler
        action.sa_sigaction = libc::SIG_DFL as libc::sighandler_t;
        action.sa_flags = 0;
        libc::sigemptyset(&mut action.sa_mask);

        let result = libc::sigaction(sig_num, &action, std::ptr::null_mut());
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Ignore a signal using sigaction (thread-safe)
pub(super) fn ignore_signal(sig_num: libc::c_int) -> Result<(), std::io::Error> {
    let _guard = SIGNAL_MUTEX
        .lock()
        .expect("signal: mutex poisoned during ignore");

    unsafe {
        let mut action: libc::sigaction = MaybeUninit::zeroed().assume_init();
        // Use SIG_IGN to ignore the signal
        action.sa_sigaction = libc::SIG_IGN as libc::sighandler_t;
        action.sa_flags = 0;
        libc::sigemptyset(&mut action.sa_mask);

        let result = libc::sigaction(sig_num, &action, std::ptr::null_mut());
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
