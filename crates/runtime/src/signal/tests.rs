use super::handlers::{ignore_signal, install_signal_handler, restore_default_handler};
use super::*;
use serial_test::serial;
use std::sync::atomic::Ordering;

#[test]
#[serial]
fn test_signal_flag_operations() {
    // Use index 3 (SIGQUIT) to avoid conflicts with actual signal tests
    // Tests use: SIGUSR1 (10 Linux, 30 macOS), SIGUSR2 (12 Linux, 31 macOS)
    // Index 3 is safe on both platforms
    const TEST_IDX: usize = 3;

    // Clear flag first (other tests might have set it)
    SIGNAL_FLAGS[TEST_IDX].store(false, Ordering::Release);

    // Now test that flag is false
    assert!(!SIGNAL_FLAGS[TEST_IDX].load(Ordering::Acquire));

    // Set flag manually (simulating signal receipt)
    SIGNAL_FLAGS[TEST_IDX].store(true, Ordering::Release);
    assert!(SIGNAL_FLAGS[TEST_IDX].load(Ordering::Acquire));

    // Swap should return old value and set new
    let was_set = SIGNAL_FLAGS[TEST_IDX].swap(false, Ordering::Acquire);
    assert!(was_set);
    assert!(!SIGNAL_FLAGS[TEST_IDX].load(Ordering::Acquire));

    // Second swap should return false
    let was_set = SIGNAL_FLAGS[TEST_IDX].swap(false, Ordering::Acquire);
    assert!(!was_set);

    // Clean up
    SIGNAL_FLAGS[TEST_IDX].store(false, Ordering::Release);
}

#[cfg(unix)]
#[test]
#[serial]
fn test_signal_handler_installation() {
    // Test that we can install a handler for SIGUSR1 (safe for testing)
    let result = install_signal_handler(libc::SIGUSR1);
    assert!(result.is_ok(), "Failed to install SIGUSR1 handler");

    // Test that we can restore the default handler
    let result = restore_default_handler(libc::SIGUSR1);
    assert!(result.is_ok(), "Failed to restore SIGUSR1 default handler");
}

#[cfg(unix)]
#[test]
#[serial]
fn test_signal_delivery() {
    // Install handler for SIGUSR1
    install_signal_handler(libc::SIGUSR1).expect("Failed to install handler");

    // Clear any pending flag
    SIGNAL_FLAGS[libc::SIGUSR1 as usize].store(false, Ordering::Release);

    // Send signal to self
    unsafe {
        libc::kill(libc::getpid(), libc::SIGUSR1);
    }

    // Give a tiny bit of time for signal delivery (should be immediate)
    std::thread::sleep(std::time::Duration::from_millis(1));

    // Check that the flag was set
    let received = SIGNAL_FLAGS[libc::SIGUSR1 as usize].swap(false, Ordering::Acquire);
    assert!(received, "Signal was not received");

    // Restore default handler
    restore_default_handler(libc::SIGUSR1).expect("Failed to restore handler");
}

#[cfg(unix)]
#[test]
#[serial]
fn test_invalid_signal_fails() {
    // SIGKILL and SIGSTOP cannot be caught
    let result = install_signal_handler(libc::SIGKILL);
    assert!(result.is_err(), "SIGKILL should not be catchable");

    let result = install_signal_handler(libc::SIGSTOP);
    assert!(result.is_err(), "SIGSTOP should not be catchable");
}

#[cfg(unix)]
#[test]
#[serial]
fn test_signal_ignore() {
    // Test that we can set a signal to be ignored
    let result = ignore_signal(libc::SIGUSR2);
    assert!(result.is_ok(), "Failed to ignore SIGUSR2");

    // Restore default
    let result = restore_default_handler(libc::SIGUSR2);
    assert!(result.is_ok(), "Failed to restore SIGUSR2 default handler");
}

#[cfg(unix)]
#[test]
#[serial]
fn test_multiple_signals_independent() {
    // Install handlers for two different signals
    install_signal_handler(libc::SIGUSR1).expect("Failed to install SIGUSR1");
    install_signal_handler(libc::SIGUSR2).expect("Failed to install SIGUSR2");

    // Clear flags
    SIGNAL_FLAGS[libc::SIGUSR1 as usize].store(false, Ordering::Release);
    SIGNAL_FLAGS[libc::SIGUSR2 as usize].store(false, Ordering::Release);

    // Send only SIGUSR1
    unsafe {
        libc::kill(libc::getpid(), libc::SIGUSR1);
    }
    std::thread::sleep(std::time::Duration::from_millis(1));

    // Only SIGUSR1 should be set
    assert!(SIGNAL_FLAGS[libc::SIGUSR1 as usize].load(Ordering::Acquire));
    assert!(!SIGNAL_FLAGS[libc::SIGUSR2 as usize].load(Ordering::Acquire));

    // Cleanup
    restore_default_handler(libc::SIGUSR1).expect("Failed to restore SIGUSR1");
    restore_default_handler(libc::SIGUSR2).expect("Failed to restore SIGUSR2");
}

#[cfg(unix)]
#[test]
fn test_signal_constants_valid() {
    // Verify signal constants are in valid range
    assert!(libc::SIGINT > 0 && (libc::SIGINT as usize) < MAX_SIGNAL);
    assert!(libc::SIGTERM > 0 && (libc::SIGTERM as usize) < MAX_SIGNAL);
    assert!(libc::SIGHUP > 0 && (libc::SIGHUP as usize) < MAX_SIGNAL);
    assert!(libc::SIGPIPE > 0 && (libc::SIGPIPE as usize) < MAX_SIGNAL);
    assert!(libc::SIGUSR1 > 0 && (libc::SIGUSR1 as usize) < MAX_SIGNAL);
    assert!(libc::SIGUSR2 > 0 && (libc::SIGUSR2 as usize) < MAX_SIGNAL);
}
