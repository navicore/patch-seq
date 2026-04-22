//! Scheduler lifecycle: init, run, shutdown, wait_all_strands.

use crate::stack::Stack;
use std::sync::atomic::Ordering;
use std::sync::{Once, OnceLock};
use std::time::{Duration, Instant};

use super::{ACTIVE_STRANDS, SHUTDOWN_CONDVAR, SHUTDOWN_MUTEX};

static SCHEDULER_INIT: Once = Once::new();
static SCHEDULER_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Default coroutine stack size: 128KB (0x20000 bytes)
/// Reduced from 1MB for better spawn performance (~16% faster in benchmarks).
/// Can be overridden via SEQ_STACK_SIZE environment variable.
pub(super) const DEFAULT_STACK_SIZE: usize = 0x20000;

/// Default coroutine pool capacity.
/// May reuses completed coroutine stacks from this pool to avoid allocations.
/// Default of 1000 is often too small for spawn-heavy workloads.
const DEFAULT_POOL_CAPACITY: usize = 10000;

/// Parse stack size from an optional string value.
/// Returns the parsed size, or DEFAULT_STACK_SIZE if the value is missing, zero, or invalid.
/// Prints a warning to stderr for invalid values.
pub(super) fn parse_stack_size(env_value: Option<String>) -> usize {
    match env_value {
        Some(val) => match val.parse::<usize>() {
            Ok(0) => {
                eprintln!(
                    "Warning: SEQ_STACK_SIZE=0 is invalid, using default {}",
                    DEFAULT_STACK_SIZE
                );
                DEFAULT_STACK_SIZE
            }
            Ok(size) => size,
            Err(_) => {
                eprintln!(
                    "Warning: SEQ_STACK_SIZE='{}' is not a valid number, using default {}",
                    val, DEFAULT_STACK_SIZE
                );
                DEFAULT_STACK_SIZE
            }
        },
        None => DEFAULT_STACK_SIZE,
    }
}

/// Get elapsed time since scheduler was initialized
pub fn scheduler_elapsed() -> Option<Duration> {
    SCHEDULER_START_TIME.get().map(|start| start.elapsed())
}

/// Initialize the scheduler.
///
/// # Safety
/// Safe to call multiple times (idempotent via Once).
/// Configures May coroutines with appropriate stack size for LLVM-generated code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_scheduler_init() {
    SCHEDULER_INIT.call_once(|| {
        // Configure stack size for coroutines
        // Default is 128KB, reduced from 1MB for better spawn performance.
        // Can be overridden via SEQ_STACK_SIZE environment variable (in bytes)
        // Example: SEQ_STACK_SIZE=2097152 for 2MB
        // Invalid values (non-numeric, zero) are warned and ignored.
        let stack_size = parse_stack_size(std::env::var("SEQ_STACK_SIZE").ok());

        // Configure coroutine pool capacity
        // May reuses coroutine stacks from this pool to reduce allocation overhead.
        // Default 10000 is 10x May's default (1000), better for spawn-heavy workloads.
        // Can be overridden via SEQ_POOL_CAPACITY environment variable.
        let pool_capacity = std::env::var("SEQ_POOL_CAPACITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_POOL_CAPACITY);

        may::config()
            .set_stack_size(stack_size)
            .set_pool_capacity(pool_capacity);

        // Record scheduler start time (for at-exit reporting)
        SCHEDULER_START_TIME.get_or_init(Instant::now);

        // Install SIGINT handler for Ctrl-C (unconditional - basic expected behavior)
        // Without this, tight loops won't respond to Ctrl-C because signals
        // are only delivered at syscall boundaries, and TCO loops may never syscall.
        #[cfg(unix)]
        {
            use std::sync::atomic::AtomicBool;
            static SIGINT_RECEIVED: AtomicBool = AtomicBool::new(false);

            extern "C" fn sigint_handler(_: libc::c_int) {
                // If we receive SIGINT twice, force exit (user is insistent)
                if SIGINT_RECEIVED.swap(true, Ordering::SeqCst) {
                    // Second SIGINT - exit immediately
                    unsafe { libc::_exit(130) }; // 128 + 2 (SIGINT)
                }
                // First SIGINT - exit cleanly
                std::process::exit(130);
            }

            unsafe {
                libc::signal(
                    libc::SIGINT,
                    sigint_handler as *const () as libc::sighandler_t,
                );
            }
        }

        // Install SIGQUIT handler for runtime diagnostics (kill -3)
        #[cfg(feature = "diagnostics")]
        crate::diagnostics::install_signal_handler();

        // Install watchdog timer (if enabled via SEQ_WATCHDOG_SECS)
        #[cfg(feature = "diagnostics")]
        crate::watchdog::install_watchdog();
    });
}

/// Run the scheduler and wait for all coroutines to complete
///
/// # Safety
/// Returns the final stack (always null for now since May handles all scheduling).
/// This function blocks until all spawned strands have completed.
///
/// Uses a condition variable for event-driven shutdown synchronization rather than
/// polling. The mutex is only held during the wait protocol, not during strand
/// execution, so there's no contention on the hot path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_scheduler_run() -> Stack {
    let mut guard = SHUTDOWN_MUTEX.lock().expect(
        "scheduler_run: shutdown mutex poisoned - strand panicked during shutdown synchronization",
    );

    // Wait for all strands to complete
    // The condition variable will be notified when the last strand exits
    while ACTIVE_STRANDS.load(Ordering::Acquire) > 0 {
        guard = SHUTDOWN_CONDVAR
            .wait(guard)
            .expect("scheduler_run: condvar wait failed - strand panicked during shutdown wait");
    }

    // All strands have completed
    std::ptr::null_mut()
}

/// Shutdown the scheduler
///
/// # Safety
/// Safe to call. May doesn't require explicit shutdown, so this is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_scheduler_shutdown() {
    // May doesn't require explicit shutdown
    // This function exists for API symmetry with init
}

/// Wait for all strands to complete
///
/// # Safety
/// Always safe to call. Blocks until all spawned strands have completed.
///
/// Uses event-driven synchronization via condition variable - no polling overhead.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_wait_all_strands() {
    let mut guard = SHUTDOWN_MUTEX.lock()
        .expect("wait_all_strands: shutdown mutex poisoned - strand panicked during shutdown synchronization");

    // Wait for all strands to complete
    // The condition variable will be notified when the last strand exits
    while ACTIVE_STRANDS.load(Ordering::Acquire) > 0 {
        guard = SHUTDOWN_CONDVAR
            .wait(guard)
            .expect("wait_all_strands: condvar wait failed - strand panicked during shutdown wait");
    }
}
