//! Signal handling API for Seq
//!
//! Provides Unix signal handling with a safe, flag-based approach:
//! - Signals are trapped and set atomic flags (no code runs in signal context)
//! - User code polls for signals at safe points
//! - Fits Seq's explicit, predictable style
//!
//! # Example
//! ```seq
//! signal.SIGINT signal.trap
//! signal.SIGTERM signal.trap
//!
//! : main-loop ( -- )
//!   signal.SIGINT signal.received? if
//!     "Shutting down..." io.write-line
//!     return
//!   then
//!   do-work
//!   main-loop
//! ;
//! ```
//!
//! # Safety
//!
//! Signal handlers execute in an interrupt context with severe restrictions.
//! This module uses only async-signal-safe operations (atomic flag setting).
//! All Seq code execution happens outside the signal handler, when the user
//! explicitly checks for received signals.
//!
//! # Thread Safety and Concurrent Access
//!
//! This module is designed to be safe for concurrent use from multiple strands:
//!
//! - **Handler installation** (`signal.trap`, `signal.default`, `signal.ignore`):
//!   Protected by a mutex to ensure only one strand modifies handlers at a time.
//!   Concurrent calls will serialize safely.
//!
//! - **Flag operations** (`signal.received?`, `signal.pending?`, `signal.clear`):
//!   Use lock-free atomic operations with appropriate memory ordering:
//!   - `signal.received?`: Atomic swap with Acquire ordering (read-modify-write)
//!   - `signal.pending?`: Atomic load with Acquire ordering (read-only)
//!   - `signal.clear`: Atomic store with Release ordering (write-only)
//!
//!   Multiple strands can safely check the same signal. However, `signal.received?`
//!   clears the flag atomically, so if two strands both call it, only one will
//!   observe `true`. Use `signal.pending?` if you need non-destructive reads.
//!
//! - **Signal handler**: Executes outside the strand context (in OS interrupt
//!   context) and only performs a single atomic store. This is async-signal-safe.
//!
//! This module uses `sigaction()` instead of the deprecated `signal()` function
//! for well-defined behavior in multithreaded environments.
//!
//! # Platform Support
//!
//! - Unix: Full signal support using sigaction()
//! - Windows: Stub implementations (signals not supported, all operations no-op)
//!
//! # Module Layout
//!
//! Per-concern sub-modules:
//! - `constants` — 9 `SIG*` constant getters (unix + non-unix stubs)
//! - `handlers` — unix-only sigaction wrappers (`install`/`restore`/`ignore`)
//!   plus the async-signal-safe `flag_signal_handler`
//! - `ops` — user-facing FFI ops (`trap`/`received?`/`pending?`/`default`/
//!   `ignore`/`clear`), both unix and non-unix stubs
//!
//! Shared state (`SIGNAL_FLAGS` and `MAX_SIGNAL`) stays on this aggregator so
//! every sub-module points at the same flag table.

use std::sync::atomic::AtomicBool;

/// Maximum signal number we support (covers all standard Unix signals)
pub(super) const MAX_SIGNAL: usize = 32;

/// Atomic flags for each signal - set by signal handler, cleared by user code
pub(super) static SIGNAL_FLAGS: [AtomicBool; MAX_SIGNAL] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

mod constants;
mod ops;

#[cfg(unix)]
mod handlers;

pub use constants::*;
pub use ops::*;

#[cfg(test)]
mod tests;
