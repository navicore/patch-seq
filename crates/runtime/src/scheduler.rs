//! Scheduler - Green Thread Management with May
//!
//! CSP-style concurrency for Seq using May coroutines.
//! Each strand is a lightweight green thread that can communicate via channels.
//!
//! ## Non-Blocking Guarantee
//!
//! Channel operations (`send`, `receive`) use May's cooperative blocking and NEVER
//! block OS threads. However, I/O operations (`write_line`, `read_line` in io.rs)
//! currently use blocking syscalls. Future work will make all I/O non-blocking.
//!
//! ## Panic Behavior
//!
//! Functions panic on invalid input (null stacks, negative IDs, closed channels).
//! In a production system, consider implementing error channels or Result-based
//! error handling instead of panicking.

use crate::stack::Stack;
use crate::tagged_stack::StackValue;
use may::coroutine;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, Once};
use std::time::{Duration, Instant};

static SCHEDULER_INIT: Once = Once::new();
static SCHEDULER_START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

// Strand lifecycle tracking
//
// Design rationale:
// - ACTIVE_STRANDS: Lock-free atomic counter for the hot path (spawn/complete)
//   Every strand increments on spawn, decrements on complete. This is extremely
//   fast (lock-free atomic ops) and suitable for high-frequency operations.
//
// - SHUTDOWN_CONDVAR/MUTEX: Event-driven synchronization for the cold path (shutdown wait)
//   Used only when waiting for all strands to complete (program shutdown).
//   Condvar provides event-driven wakeup instead of polling, which is critical
//   for a systems language - no CPU waste, proper OS-level blocking.
//
// Why not track JoinHandles?
// Strands are like Erlang processes - potentially hundreds of thousands of concurrent
// entities with independent lifecycles. Storing handles would require global mutable
// state with synchronization overhead on the hot path. The counter + condvar approach
// keeps the hot path lock-free while providing proper shutdown synchronization.
pub static ACTIVE_STRANDS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static SHUTDOWN_CONDVAR: Condvar = Condvar::new();
pub(crate) static SHUTDOWN_MUTEX: Mutex<()> = Mutex::new(());

// Strand lifecycle statistics (for diagnostics)
//
// These counters provide observability into strand lifecycle without any locking.
// All operations are lock-free atomic increments/loads.
//
// - TOTAL_SPAWNED: Monotonically increasing count of all strands ever spawned
// - TOTAL_COMPLETED: Monotonically increasing count of all strands that completed
// - PEAK_STRANDS: High-water mark of concurrent strands (helps detect strand leaks)
//
// Useful diagnostics:
// - Currently running: ACTIVE_STRANDS
// - Completed successfully: TOTAL_COMPLETED
// - Potential leaks: TOTAL_SPAWNED - TOTAL_COMPLETED - ACTIVE_STRANDS > 0 (strands lost)
// - Peak concurrency: PEAK_STRANDS
pub static TOTAL_SPAWNED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_COMPLETED: AtomicU64 = AtomicU64::new(0);
pub static PEAK_STRANDS: AtomicUsize = AtomicUsize::new(0);

// Unique strand ID generation
static NEXT_STRAND_ID: AtomicU64 = AtomicU64::new(1);

// =============================================================================
// Lock-Free Strand Registry (only when diagnostics feature is enabled)
// =============================================================================
//
// A fixed-size array of slots for tracking active strands without locks.
// Each slot stores a strand ID (0 = free) and spawn timestamp.
//
// Design principles:
// - Fixed size: No dynamic allocation, predictable memory footprint
// - Lock-free: All operations use atomic CAS, no mutex contention
// - Bounded: If registry is full, strands still run but aren't tracked
// - Zero cost when not querying: Only diagnostics reads the registry
//
// Slot encoding:
// - strand_id == 0: slot is free
// - strand_id > 0: slot contains an active strand
//
// The registry size can be configured via SEQ_STRAND_REGISTRY_SIZE env var.
// Default is 1024 slots, which is sufficient for most applications.
//
// When the "diagnostics" feature is disabled, the registry is not compiled,
// eliminating the SystemTime::now() syscall and O(n) scans on every spawn.

#[cfg(feature = "diagnostics")]
/// Default strand registry size (number of trackable concurrent strands)
const DEFAULT_REGISTRY_SIZE: usize = 1024;

#[cfg(feature = "diagnostics")]
/// A slot in the strand registry
///
/// Uses two atomics to store strand info without locks.
/// A slot is free when strand_id == 0.
pub struct StrandSlot {
    /// Strand ID (0 = free, >0 = active strand)
    pub strand_id: AtomicU64,
    /// Spawn timestamp (seconds since UNIX epoch, for detecting stuck strands)
    pub spawn_time: AtomicU64,
}

#[cfg(feature = "diagnostics")]
impl StrandSlot {
    const fn new() -> Self {
        Self {
            strand_id: AtomicU64::new(0),
            spawn_time: AtomicU64::new(0),
        }
    }
}

#[cfg(feature = "diagnostics")]
/// Lock-free strand registry
///
/// Provides O(n) registration (scan for free slot) and O(n) unregistration.
/// This is acceptable because:
/// 1. N is bounded (default 1024)
/// 2. Registration/unregistration are infrequent compared to strand work
/// 3. No locks means no contention, just atomic ops
pub struct StrandRegistry {
    slots: Box<[StrandSlot]>,
    /// Number of slots that couldn't be registered (registry full)
    pub overflow_count: AtomicU64,
}

#[cfg(feature = "diagnostics")]
impl StrandRegistry {
    /// Create a new registry with the given capacity
    fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(StrandSlot::new());
        }
        Self {
            slots: slots.into_boxed_slice(),
            overflow_count: AtomicU64::new(0),
        }
    }

    /// Register a strand, returning the slot index if successful
    ///
    /// Uses CAS to atomically claim a free slot.
    /// Returns None if the registry is full (strand still runs, just not tracked).
    pub fn register(&self, strand_id: u64) -> Option<usize> {
        let spawn_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Scan for a free slot
        for (idx, slot) in self.slots.iter().enumerate() {
            // Set spawn time first, before claiming the slot
            // This prevents a race where a reader sees strand_id != 0 but spawn_time == 0
            // If we fail to claim the slot, the owner will overwrite this value anyway
            slot.spawn_time.store(spawn_time, Ordering::Relaxed);

            // Try to claim this slot (CAS from 0 to strand_id)
            // AcqRel ensures the spawn_time write above is visible before strand_id becomes non-zero
            if slot
                .strand_id
                .compare_exchange(0, strand_id, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return Some(idx);
            }
        }

        // Registry full - track overflow but strand still runs
        self.overflow_count.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Unregister a strand by ID
    ///
    /// Scans for the slot containing this strand ID and clears it.
    /// Returns true if found and cleared, false if not found.
    ///
    /// Note: ABA problem is not a concern here because strand IDs are monotonically
    /// increasing u64 values. ID reuse would require 2^64 spawns, which is practically
    /// impossible (at 1 billion spawns/sec, it would take ~584 years).
    pub fn unregister(&self, strand_id: u64) -> bool {
        for slot in self.slots.iter() {
            // Check if this slot contains our strand
            if slot
                .strand_id
                .compare_exchange(strand_id, 0, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                // Successfully cleared the slot
                slot.spawn_time.store(0, Ordering::Release);
                return true;
            }
        }
        false
    }

    /// Iterate over active strands (for diagnostics)
    ///
    /// Returns an iterator of (strand_id, spawn_time) for non-empty slots.
    /// Note: This is a snapshot and may be slightly inconsistent due to concurrent updates.
    pub fn active_strands(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.slots.iter().filter_map(|slot| {
            // Acquire on strand_id synchronizes with the Release in register()
            let id = slot.strand_id.load(Ordering::Acquire);
            if id > 0 {
                // Relaxed is sufficient here - we've already synchronized via strand_id Acquire
                // and spawn_time is written before strand_id in register()
                let time = slot.spawn_time.load(Ordering::Relaxed);
                Some((id, time))
            } else {
                None
            }
        })
    }

    /// Get the registry capacity
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

// Global strand registry (lazy initialized)
#[cfg(feature = "diagnostics")]
static STRAND_REGISTRY: std::sync::OnceLock<StrandRegistry> = std::sync::OnceLock::new();

/// Get or initialize the global strand registry
#[cfg(feature = "diagnostics")]
pub fn strand_registry() -> &'static StrandRegistry {
    STRAND_REGISTRY.get_or_init(|| {
        let size = std::env::var("SEQ_STRAND_REGISTRY_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REGISTRY_SIZE);
        StrandRegistry::new(size)
    })
}

/// Get elapsed time since scheduler was initialized
pub fn scheduler_elapsed() -> Option<Duration> {
    SCHEDULER_START_TIME.get().map(|start| start.elapsed())
}

/// Default coroutine stack size: 128KB (0x20000 bytes)
/// Reduced from 1MB for better spawn performance (~16% faster in benchmarks).
/// Can be overridden via SEQ_STACK_SIZE environment variable.
const DEFAULT_STACK_SIZE: usize = 0x20000;

/// Parse stack size from an optional string value.
/// Returns the parsed size, or DEFAULT_STACK_SIZE if the value is missing, zero, or invalid.
/// Prints a warning to stderr for invalid values.
fn parse_stack_size(env_value: Option<String>) -> usize {
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

/// Default coroutine pool capacity.
/// May reuses completed coroutine stacks from this pool to avoid allocations.
/// Default of 1000 is often too small for spawn-heavy workloads.
const DEFAULT_POOL_CAPACITY: usize = 10000;

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
            use std::sync::atomic::{AtomicBool, Ordering};
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
    let _ = strand_registry().register(strand_id);

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
            strand_registry().unregister(strand_id);

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
fn free_stack(_stack: Stack) {
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

/// Yield execution to allow other coroutines to run
///
/// # Safety
/// Always safe to call from within a May coroutine.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_yield_strand(stack: Stack) -> Stack {
    coroutine::yield_now();
    stack
}

// =============================================================================
// Cooperative Yield Safety Valve
// =============================================================================
//
// Prevents tight TCO loops from starving other strands and making the process
// unresponsive. When enabled via SEQ_YIELD_INTERVAL, yields after N tail calls.
//
// Configuration:
//   SEQ_YIELD_INTERVAL=10000  - Yield every 10,000 tail calls (default: 0 = disabled)
//
// Scope:
//   - Covers: User-defined word tail calls (musttail) and quotation tail calls
//   - Does NOT cover: Closure calls (they use regular calls, bounded by stack)
//   - Does NOT cover: Non-tail recursive calls (bounded by stack)
//   This is intentional: the safety valve targets unbounded TCO loops.
//
// Design:
//   - Zero overhead when disabled (threshold=0 short-circuits immediately)
//   - Thread-local counter avoids synchronization overhead
//   - Called before every musttail in generated code
//   - Threshold is cached on first access via OnceLock
//
// Thread-Local Counter Behavior:
//   The counter is per-OS-thread, not per-coroutine. Multiple coroutines on the
//   same OS thread share the counter, which may cause yields slightly more
//   frequently than the configured interval. This is intentional:
//   - Avoids coroutine-local storage overhead
//   - Still achieves the goal of preventing starvation
//   - Actual yield frequency is still bounded by the threshold

use std::cell::Cell;
use std::sync::OnceLock;

/// Cached yield interval threshold (0 = disabled)
static YIELD_THRESHOLD: OnceLock<u64> = OnceLock::new();

thread_local! {
    /// Per-thread tail call counter
    static TAIL_CALL_COUNTER: Cell<u64> = const { Cell::new(0) };
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

// Public re-exports with short names for internal use
pub use patch_seq_maybe_yield as maybe_yield;
pub use patch_seq_scheduler_init as scheduler_init;
pub use patch_seq_scheduler_run as scheduler_run;
pub use patch_seq_scheduler_shutdown as scheduler_shutdown;
pub use patch_seq_spawn_strand as spawn_strand;
pub use patch_seq_strand_spawn as strand_spawn;
pub use patch_seq_wait_all_strands as wait_all_strands;
pub use patch_seq_yield_strand as yield_strand;

#[cfg(test)]
mod tests;
