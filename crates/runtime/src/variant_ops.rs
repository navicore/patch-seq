//! Variant operations for Seq
//!
//! Provides runtime functions for accessing variant fields, tags, and
//! metadata, for building variants of small fixed arities, and for
//! functional-style append/init/last updates plus field unpacking for
//! pattern matches.

mod access;
mod make;
mod modify;

#[cfg(test)]
mod tests;

pub use access::*;
pub use make::*;
pub use modify::*;

// Short-name aliases used by tests and internal callers.
pub use patch_seq_make_variant_0 as make_variant_0;
pub use patch_seq_make_variant_1 as make_variant_1;
pub use patch_seq_make_variant_2 as make_variant_2;
pub use patch_seq_make_variant_3 as make_variant_3;
pub use patch_seq_make_variant_4 as make_variant_4;
pub use patch_seq_make_variant_5 as make_variant_5;
pub use patch_seq_make_variant_6 as make_variant_6;
pub use patch_seq_make_variant_7 as make_variant_7;
pub use patch_seq_make_variant_8 as make_variant_8;
pub use patch_seq_make_variant_9 as make_variant_9;
pub use patch_seq_make_variant_10 as make_variant_10;
pub use patch_seq_make_variant_11 as make_variant_11;
pub use patch_seq_make_variant_12 as make_variant_12;
pub use patch_seq_unpack_variant as unpack_variant;
pub use patch_seq_variant_append as variant_append;
pub use patch_seq_variant_field_at as variant_field_at;
pub use patch_seq_variant_field_count as variant_field_count;
pub use patch_seq_variant_init as variant_init;
pub use patch_seq_variant_last as variant_last;
pub use patch_seq_variant_tag as variant_tag;
