//! Resource Leak Detection (Phase 2a)
//!
//! Data flow analysis to detect resource leaks within single word definitions.
//! Tracks resources (weave handles, channels) through stack operations and
//! control flow to ensure proper cleanup.
//!
//! # Architecture
//!
//! 1. **Resource Tagging**: Values from resource-creating words are tagged
//!    with their creation location.
//!
//! 2. **Stack Simulation**: Abstract interpretation tracks tagged values
//!    through stack operations (dup, swap, drop, etc.).
//!
//! 3. **Control Flow**: If/else and match branches must handle resources
//!    consistently - either all consume or all preserve.
//!
//! 4. **Escape Analysis**: Resources returned from a word are the caller's
//!    responsibility - no warning emitted.
//!
//! # Known Limitations
//!
//! - **`strand.resume` completion not tracked**: When `strand.resume` returns
//!   false, the weave completed and handle is consumed. We can't determine this
//!   statically, so we assume the handle remains active. Use pattern-based lint
//!   rules to catch unchecked resume results.
//!
//! - **Unknown word effects**: User-defined words and FFI calls have unknown
//!   stack effects. We conservatively leave the stack unchanged, which may
//!   cause false negatives if those words consume or create resources.
//!
//! - **Cross-word analysis is basic**: Resources returned from user-defined
//!   words are tracked via `ProgramResourceAnalyzer`, but external/FFI words
//!   with unknown effects are treated conservatively (no stack change assumed).

mod program;
mod state;
mod word;

#[cfg(test)]
mod tests;

pub use program::ProgramResourceAnalyzer;
pub use word::ResourceAnalyzer;
