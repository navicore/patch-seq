//! Abstract Syntax Tree for Seq
//!
//! Minimal AST sufficient for hello-world and basic programs.
//! Will be extended as we add more language features.

mod program;
mod types;

#[cfg(test)]
mod tests;

pub use types::*;
