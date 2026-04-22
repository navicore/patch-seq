//! Weave-internal helpers for coroutine lifecycle.
//!
//! - `cleanup_strand`: decrement `ACTIVE_STRANDS` and signal shutdown when the
//!   last strand completes. Called by spawn (normal completion) and yield
//!   (unrecoverable error path).
//! - `block_forever`: block the current coroutine without panicking, used when
//!   an `extern "C"` weave function hits an unrecoverable error and cannot
//!   return or panic safely.

use may::sync::mpmc;

/// Block the current coroutine forever without panicking.
///
/// This is used when an unrecoverable error occurs in an extern "C" function.
/// We can't panic (UB across FFI) and we can't return (invalid state), so we
/// clean up and block forever. The coroutine is already marked as completed
/// via `cleanup_strand()`, so the program can still terminate normally.
///
/// # Safety
/// Must only be called from within a spawned coroutine, never from the main thread.
///
/// # Implementation
/// Uses May's coroutine-aware channel blocking. We keep the sender alive so that
/// `recv()` blocks the coroutine (not the OS thread) indefinitely. This is critical
/// because `std::thread::park()` would block the OS thread and starve all other
/// coroutines on that thread, potentially deadlocking the scheduler.
pub(super) fn block_forever() -> ! {
    // Create channel and keep sender alive to prevent recv() from returning Err
    let (tx, rx): (mpmc::Sender<()>, mpmc::Receiver<()>) = mpmc::channel();
    // Leak the sender so it's never dropped - this ensures recv() blocks forever
    // rather than returning Err(RecvError) when all senders are dropped.
    // This is an intentional memory leak - we're blocking forever anyway.
    std::mem::forget(tx);
    // Block forever using May's coroutine-aware recv()
    // This yields the coroutine to the scheduler rather than blocking the OS thread
    loop {
        let _ = rx.recv();
    }
}

/// Helper to clean up strand on exit
pub(super) fn cleanup_strand() {
    use crate::scheduler::{ACTIVE_STRANDS, SHUTDOWN_CONDVAR, SHUTDOWN_MUTEX, TOTAL_COMPLETED};
    use std::sync::atomic::Ordering;

    let prev_count = ACTIVE_STRANDS.fetch_sub(1, Ordering::AcqRel);
    TOTAL_COMPLETED.fetch_add(1, Ordering::Release);

    if prev_count == 1 {
        let _guard = SHUTDOWN_MUTEX
            .lock()
            .expect("weave: shutdown mutex poisoned");
        SHUTDOWN_CONDVAR.notify_all();
    }
}
