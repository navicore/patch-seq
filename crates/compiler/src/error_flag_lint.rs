//! Error Flag Detection (Phase 2b)
//!
//! Abstract stack simulation that tracks Bool values produced by fallible
//! operations. Warns when these "error flags" are dropped without being
//! checked via `if` or `cond`.
//!
//! This catches patterns that the TOML-based pattern linter misses:
//! - `file.slurp swap nip` (Bool moved by swap, then dropped by nip)
//! - `i./ >aux ... aux> drop` (Bool stashed on aux stack, dropped later)
//!
//! # Architecture
//!
//! Modeled on `resource_lint.rs`:
//! 1. Tag Bools from fallible ops with their origin
//! 2. Simulate stack operations to track tag movement
//! 3. When a tagged Bool is consumed by `if`/`cond`, mark checked
//! 4. When consumed by `drop`/`nip`/other, emit warning
//!
//! # Conservative Design
//!
//! - Only tracks Bools from known fallible builtins (not all Bools)
//! - If a tagged Bool flows into an unknown user word, assume checked
//!   (avoids false positives from cross-word analysis)
//! - Bools remaining on the stack at word end are assumed returned
//!   (escape analysis, same as resource_lint)

mod analyzer;
mod state;

#[cfg(test)]
mod tests;

pub use analyzer::ErrorFlagAnalyzer;
