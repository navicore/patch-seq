//! Strand (coroutine) spawn and lifecycle cleanup.

use crate::stack::Stack;
use crate::tagged_stack::StackValue;
use may::coroutine;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{
    ACTIVE_STRANDS, PEAK_STRANDS, SHUTDOWN_CONDVAR, SHUTDOWN_MUTEX, TOTAL_COMPLETED, TOTAL_SPAWNED,
};

// Unique strand ID generation
static NEXT_STRAND_ID: AtomicU64 = AtomicU64::new(1);

/// Spawn a strand (coroutine) with initial stack
///
/// # Safety
/// - `entry` must be a valid function pointer that can safely execute on any thread
/// - `initial_stack` must be either null or a valid pointer to a `StackValue` that:
///   - Was heap-allocated (e.g., via Box)
///   - Has a 'static lifetime or lives longer than the coroutine
///   - Is safe to access from the spawned thread
/// - The caller transfers ownership of `initial_stack` to the coroutine
/// - Returns a unique strand ID (positive integer)
///
/// # Memory Management
/// The spawned coroutine takes ownership of `initial_stack` and will automatically
/// free the final stack returned by `entry` upon completion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_strand_spawn(
    entry: extern "C" fn(Stack) -> Stack,
    initial_stack: Stack,
) -> i64 {
    // For backwards compatibility, use null base (won't support nested spawns)
    unsafe { patch_seq_strand_spawn_with_base(entry, initial_stack, std::ptr::null_mut()) }
}

/// Spawn a strand (coroutine) with initial stack and explicit stack base
///
/// This variant allows setting the STACK_BASE for the spawned strand, which is
/// required for the child to perform operations like clone_stack (nested spawn).
///
/// # Safety
/// - `entry` must be a valid function pointer that can safely execute on any thread
/// - `initial_stack` must be a valid pointer to a `StackValue` array
/// - `stack_base` must be the base of the stack (or null to skip setting STACK_BASE)
/// - The caller transfers ownership of `initial_stack` to the coroutine
/// - Returns a unique strand ID (positive integer)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_strand_spawn_with_base(
    entry: extern "C" fn(Stack) -> Stack,
    initial_stack: Stack,
    stack_base: Stack,
) -> i64 {
    // Generate unique strand ID
    let strand_id = NEXT_STRAND_ID.fetch_add(1, Ordering::Relaxed);

    // Increment active strand counter and track total spawned
    let new_count = ACTIVE_STRANDS.fetch_add(1, Ordering::Release) + 1;
    TOTAL_SPAWNED.fetch_add(1, Ordering::Relaxed);

    // Update peak strands if this is a new high-water mark
    // Uses a CAS loop to safely update the maximum without locks
    // Uses Acquire/Release ordering for proper synchronization with diagnostics reads
    let mut peak = PEAK_STRANDS.load(Ordering::Acquire);
    while new_count > peak {
        match PEAK_STRANDS.compare_exchange_weak(
            peak,
            new_count,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(current) => peak = current,
        }
    }

    // Register strand in the registry (for diagnostics visibility)
    // If registry is full, strand still runs but isn't tracked
    #[cfg(feature = "diagnostics")]
    let _ = super::registry::strand_registry().register(strand_id);

    // Function pointers are already Send, no wrapper needed
    let entry_fn = entry;

    // Convert pointers to usize (which is Send)
    // This is necessary because *mut T is !Send, but the caller guarantees thread safety
    let stack_addr = initial_stack as usize;
    let base_addr = stack_base as usize;

    unsafe {
        coroutine::spawn(move || {
            // Reconstruct pointers from addresses
            let stack_ptr = stack_addr as *mut StackValue;
            let base_ptr = base_addr as *mut StackValue;

            // Debug assertion: validate stack pointer alignment and reasonable address
            debug_assert!(
                stack_ptr.is_null()
                    || stack_addr.is_multiple_of(std::mem::align_of::<StackValue>()),
                "Stack pointer must be null or properly aligned"
            );
            debug_assert!(
                stack_ptr.is_null() || stack_addr > 0x1000,
                "Stack pointer appears to be in invalid memory region (< 0x1000)"
            );

            // Set STACK_BASE for this strand if provided
            // This enables nested spawns and other operations that need clone_stack
            if !base_ptr.is_null() {
                crate::stack::patch_seq_set_stack_base(base_ptr);
            }

            // Execute the entry function
            let final_stack = entry_fn(stack_ptr);

            // Clean up the final stack to prevent memory leak
            free_stack(final_stack);

            // Unregister strand from registry (uses captured strand_id)
            #[cfg(feature = "diagnostics")]
            super::registry::strand_registry().unregister(strand_id);

            // Decrement active strand counter first, then track completion
            // This ordering ensures the invariant SPAWNED = COMPLETED + ACTIVE + lost
            // is never violated from an external observer's perspective
            // Use AcqRel to establish proper synchronization (both acquire and release barriers)
            let prev_count = ACTIVE_STRANDS.fetch_sub(1, Ordering::AcqRel);

            // Track completion after decrementing active count
            TOTAL_COMPLETED.fetch_add(1, Ordering::Release);
            if prev_count == 1 {
                // We were the last strand - acquire mutex and signal shutdown
                // The mutex must be held when calling notify to prevent missed wakeups
                let _guard = SHUTDOWN_MUTEX.lock()
                    .expect("strand_spawn: shutdown mutex poisoned - strand panicked during shutdown notification");
                SHUTDOWN_CONDVAR.notify_all();
            }
        });
    }

    strand_id as i64
}

/// Free a stack allocated by the runtime
///
/// With the tagged stack implementation, stack cleanup is handled differently.
/// The contiguous array is freed when the TaggedStack is dropped.
/// This function just resets the thread-local arena.
///
/// # Safety
/// Stack pointer must be valid or null.
pub(super) fn free_stack(_stack: Stack) {
    // With tagged stack, the array is freed when TaggedStack is dropped.
    // We just need to reset the arena for thread-local strings.

    // Reset the thread-local arena to free all arena-allocated strings
    // This is safe because:
    // - Any arena strings in Values have been dropped above
    // - Global strings are unaffected (they have their own allocations)
    // - Channel sends clone to global, so no cross-strand arena pointers
    crate::arena::arena_reset();
}

/// Legacy spawn_strand function (kept for compatibility)
///
/// # Safety
/// `entry` must be a valid function pointer that can safely execute on any thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_spawn_strand(entry: extern "C" fn(Stack) -> Stack) {
    unsafe {
        patch_seq_strand_spawn(entry, std::ptr::null_mut());
    }
}
