//! Cooperative yield: `yield_strand` (explicit) and `maybe_yield` (safety valve).
//!
//! ## Cooperative Yield Safety Valve
//!
//! Prevents tight TCO loops from starving other strands and making the process
//! unresponsive. When enabled via `SEQ_YIELD_INTERVAL`, yields after N tail calls.
//!
//! Configuration:
//!   `SEQ_YIELD_INTERVAL=10000`  - Yield every 10,000 tail calls (default: 0 = disabled)
//!
//! Scope:
//!   - Covers: User-defined word tail calls (musttail) and quotation tail calls
//!   - Does NOT cover: Closure calls (they use regular calls, bounded by stack)
//!   - Does NOT cover: Non-tail recursive calls (bounded by stack)
//!
//! This is intentional: the safety valve targets unbounded TCO loops.
//!
//! Design:
//!   - Zero overhead when disabled (threshold=0 short-circuits immediately)
//!   - Thread-local counter avoids synchronization overhead
//!   - Called before every musttail in generated code
//!   - Threshold is cached on first access via OnceLock
//!
//! Thread-Local Counter Behavior:
//!   The counter is per-OS-thread, not per-coroutine. Multiple coroutines on the
//!   same OS thread share the counter, which may cause yields slightly more
//!   frequently than the configured interval. This is intentional:
//!   - Avoids coroutine-local storage overhead
//!   - Still achieves the goal of preventing starvation
//!   - Actual yield frequency is still bounded by the threshold

use crate::stack::Stack;
use may::coroutine;
use std::cell::Cell;
use std::sync::OnceLock;

/// Cached yield interval threshold (0 = disabled)
static YIELD_THRESHOLD: OnceLock<u64> = OnceLock::new();

thread_local! {
    /// Per-thread tail call counter
    pub(super) static TAIL_CALL_COUNTER: Cell<u64> = const { Cell::new(0) };
}

/// Get the yield threshold from environment (cached)
///
/// Returns 0 (disabled) if SEQ_YIELD_INTERVAL is not set or invalid.
/// Prints a warning to stderr if the value is set but invalid.
fn get_yield_threshold() -> u64 {
    *YIELD_THRESHOLD.get_or_init(|| {
        match std::env::var("SEQ_YIELD_INTERVAL") {
            Ok(s) if s.is_empty() => 0,
            Ok(s) => match s.parse::<u64>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!(
                        "Warning: SEQ_YIELD_INTERVAL='{}' is not a valid positive integer, yield safety valve disabled",
                        s
                    );
                    0
                }
            },
            Err(_) => 0,
        }
    })
}

/// Yield execution to allow other coroutines to run
///
/// # Safety
/// Always safe to call from within a May coroutine.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_yield_strand(stack: Stack) -> Stack {
    coroutine::yield_now();
    stack
}

/// Maybe yield to other coroutines based on tail call count
///
/// Called before every tail call in generated code. When SEQ_YIELD_INTERVAL
/// is set, yields after that many tail calls to prevent starvation.
///
/// # Performance
/// - Disabled (default): Single branch on cached threshold (< 1ns)
/// - Enabled: Increment + compare + occasional yield (~10-20ns average)
///
/// # Safety
/// Always safe to call. No-op when not in a May coroutine context.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_maybe_yield() {
    let threshold = get_yield_threshold();

    // Fast path: disabled
    if threshold == 0 {
        return;
    }

    TAIL_CALL_COUNTER.with(|counter| {
        let count = counter.get().wrapping_add(1);
        counter.set(count);

        if count >= threshold {
            counter.set(0);
            coroutine::yield_now();
        }
    });
}
