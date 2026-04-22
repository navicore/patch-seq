//! Weave operations for generator/coroutine-style concurrency
//!
//! A "weave" is a strand that can yield values back to its caller and be resumed.
//! Unlike regular strands (fire-and-forget), weaves allow bidirectional communication
//! with structured yield/resume semantics.
//!
//! ## Zero-Mutex Design
//!
//! Like channels, weaves pass their communication handles directly on the stack.
//! There is NO global registry and NO mutex contention. The weave context travels
//! with the stack values.
//!
//! ## API
//!
//! - `strand.weave`: ( Quotation -- WeaveHandle ) - creates a woven strand, returns handle
//! - `strand.resume`: ( WeaveHandle a -- WeaveHandle a Bool ) - resume with value
//! - `strand.weave-cancel`: ( WeaveHandle -- ) - cancel a weave and release its resources
//! - `yield`: ( WeaveCtx a -- WeaveCtx a ) - yield a value (only valid inside weave)
//!
//! ## Architecture
//!
//! Each weave has two internal channels that travel as values:
//! - The WeaveHandle (returned to caller) contains the yield_chan for receiving
//! - The WeaveCtx (on weave's stack) contains both channels for yield to use
//!
//! Flow:
//! 1. strand.weave creates channels, spawns coroutine with WeaveCtx on stack
//! 2. The coroutine waits on resume_chan for the first resume value
//! 3. Caller calls strand.resume with WeaveHandle, sending value to resume_chan
//! 4. Coroutine wakes, receives value, runs until yield
//! 5. yield uses WeaveCtx to send/receive, returns with new resume value
//! 6. When quotation returns, WeaveCtx signals completion
//!
//! ## Resource Management
//!
//! **Best practice:** Weaves should either be resumed until completion OR explicitly
//! cancelled with `strand.weave-cancel` to cleanly release resources.
//!
//! However, dropping a WeaveHandle without doing either is safe - the program will
//! still exit normally. The un-resumed weave is "dormant" (not counted as an active
//! strand) until its first resume, so it won't block program shutdown. The dormant
//! coroutine will be cleaned up when the program exits.
//!
//! **Resource implications of dormant weaves:** Each dormant weave consumes memory
//! for its coroutine stack (default 128KB, configurable via SEQ_STACK_SIZE) until
//! program exit. For short-lived programs or REPL sessions this is fine, but
//! long-running servers should properly cancel weaves to avoid accumulating memory.
//!
//! Proper cleanup options:
//!
//! **Option 1: Resume until completion**
//! ```seq
//! [ generator-body ] strand.weave  # Create weave
//! 0 strand.resume                   # Resume until...
//! if                                # ...has_more is false
//!   # process value...
//!   drop 0 strand.resume           # Keep resuming
//! else
//!   drop drop                       # Clean up when done
//! then
//! ```
//!
//! **Option 2: Explicit cancellation**
//! ```seq
//! [ generator-body ] strand.weave  # Create weave
//! 0 strand.resume                   # Get first value
//! if
//!   drop                           # We only needed the first value
//!   strand.weave-cancel            # Cancel and clean up
//! else
//!   drop drop
//! then
//! ```
//!
//! ## Implementation Notes
//!
//! Control flow (completion, cancellation) is handled via a type-safe `WeaveMessage`
//! enum rather than sentinel values. This means **any** Value can be safely yielded
//! and resumed, including edge cases like `i64::MIN`.
//!
//! ## Error Handling
//!
//! All weave functions are `extern "C"` and never panic (panicking across FFI is UB).
//!
//! - **Type mismatches** (e.g., `strand.resume` without a WeaveHandle): These indicate
//!   a compiler bug or memory corruption. The function prints an error to stderr and
//!   calls `std::process::abort()` to terminate immediately.
//!
//! - **Channel errors in `yield`**: If channels close unexpectedly while a coroutine
//!   is yielding, the coroutine cleans up and blocks forever. The main program can
//!   still terminate normally since the strand is marked as completed.
//!
//! - **Channel errors in `resume`**: Returns `(handle, placeholder, false)` to indicate
//!   the weave has completed or failed. The caller should check the Bool result.
//!
//! ## Module Layout
//!
//! Per-concern sub-modules:
//! - `spawn` — `patch_seq_weave` (creator, ~240 L with both Quotation and Closure paths)
//! - `resume` — `patch_seq_resume` + `patch_seq_weave_cancel` (caller-side)
//! - `yield_op` — `patch_seq_yield` (weave-side)
//! - `strand_lifecycle` — shared `cleanup_strand` / `block_forever` helpers

mod resume;
mod spawn;
mod strand_lifecycle;
mod yield_op;

pub use resume::{patch_seq_resume, patch_seq_weave_cancel};
pub use spawn::patch_seq_weave;
pub use yield_op::patch_seq_yield;

// Public re-exports
pub use patch_seq_resume as resume;
pub use patch_seq_weave as weave;
pub use patch_seq_weave_cancel as weave_cancel;
pub use patch_seq_yield as weave_yield;

#[cfg(test)]
mod tests;
