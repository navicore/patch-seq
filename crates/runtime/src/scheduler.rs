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
//!
//! ## Module Layout
//!
//! Per-concern sub-modules:
//! - `lifecycle` — init / run / shutdown / wait_all_strands / scheduler_elapsed
//! - `spawn` — strand_spawn (+ with_base) / spawn_strand (legacy) / free_stack
//! - `yield_ops` — yield_strand (explicit) / maybe_yield (TCO safety valve)
//! - `registry` — lock-free strand registry (diagnostics feature only)
//!
//! Shared lifecycle state (`ACTIVE_STRANDS`, `SHUTDOWN_*`, `TOTAL_*`,
//! `PEAK_STRANDS`) lives on this aggregator so all sub-modules and
//! consumers (`weave`, `diagnostics`, `report`) reference one source of truth.

use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Condvar, Mutex};

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

mod lifecycle;
mod spawn;
mod yield_ops;

#[cfg(feature = "diagnostics")]
mod registry;

pub use lifecycle::{
    patch_seq_scheduler_init, patch_seq_scheduler_run, patch_seq_scheduler_shutdown,
    patch_seq_wait_all_strands, scheduler_elapsed,
};
pub use spawn::{patch_seq_spawn_strand, patch_seq_strand_spawn, patch_seq_strand_spawn_with_base};
pub use yield_ops::{patch_seq_maybe_yield, patch_seq_yield_strand};

#[cfg(feature = "diagnostics")]
pub use registry::{StrandRegistry, StrandSlot, strand_registry};

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
