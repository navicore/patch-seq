//! String operations for Seq
//!
//! All FFI entry points below are exported with C ABI for LLVM codegen to
//! call. They are grouped into per-concern sub-modules and re-exported
//! from here so the flat public surface (`string_ops::patch_seq_*`) is
//! unchanged.
//!
//! # Design Decision: split Return Value
//!
//! `split` uses Option A (push parts + count):
//! - "a b c" " " split → "a" "b" "c" 3
//!
//! This is the simplest approach, requiring no new types.
//! The count allows the caller to know how many parts were pushed.

mod access;
mod basic;
mod case;
mod conversion;
mod cstring;

#[cfg(test)]
mod tests;

pub use access::*;
pub use basic::*;
pub use case::*;
pub use conversion::*;
pub use cstring::*;

// Public re-exports with short names for internal use
pub use patch_seq_char_to_string as char_to_string;
pub use patch_seq_json_escape as json_escape;
pub use patch_seq_string_byte_length as string_byte_length;
pub use patch_seq_string_char_at as string_char_at;
pub use patch_seq_string_chomp as string_chomp;
pub use patch_seq_string_concat as string_concat;
pub use patch_seq_string_contains as string_contains;
pub use patch_seq_string_empty as string_empty;
pub use patch_seq_string_equal as string_equal;
pub use patch_seq_string_find as string_find;
pub use patch_seq_string_length as string_length;
pub use patch_seq_string_split as string_split;
pub use patch_seq_string_starts_with as string_starts_with;
pub use patch_seq_string_substring as string_substring;
pub use patch_seq_string_to_int as string_to_int;
pub use patch_seq_string_to_lower as string_to_lower;
pub use patch_seq_string_to_upper as string_to_upper;
pub use patch_seq_string_trim as string_trim;
