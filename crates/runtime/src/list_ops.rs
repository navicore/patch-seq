//! List operations for Seq
//!
//! Higher-order combinators and basic list helpers over the variant-based
//! list representation. FFI entry points are grouped into per-concern
//! sub-modules and re-exported from here so the flat public surface is
//! unchanged.

mod access;
mod basic;
mod combinators;

#[cfg(test)]
mod tests;

pub use access::*;
pub use basic::*;
pub use combinators::*;

// Short-name aliases used by tests and internal callers.
pub use patch_seq_list_each as list_each;
pub use patch_seq_list_empty as list_empty;
pub use patch_seq_list_filter as list_filter;
pub use patch_seq_list_fold as list_fold;
pub use patch_seq_list_get as list_get;
pub use patch_seq_list_length as list_length;
pub use patch_seq_list_make as list_make;
pub use patch_seq_list_map as list_map;
pub use patch_seq_list_push as list_push;
pub use patch_seq_list_reverse as list_reverse;
pub use patch_seq_list_set as list_set;
