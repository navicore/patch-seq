//! Closure support for Seq
//!
//! Provides runtime functions for creating and managing closures (quotations with captured environments).
//!
//! A closure consists of:
//! - Function pointer (the compiled quotation code)
//! - Environment (Arc-shared array of captured values for TCO support)
//!
//! Note: These extern "C" functions use Value and slice pointers, which aren't technically FFI-safe,
//! but they work correctly when called from LLVM-generated code (not actual C interop).
//!
//! ## Type Support Status
//!
//! Currently supported capture types:
//! - **Int** (via `env_get_int`)
//! - **Bool** (via `env_get_bool`) - returns i64 (0/1)
//! - **Float** (via `env_get_float`) - returns f64
//! - **String** (via `env_get_string`)
//! - **Quotation** (via `env_get_quotation`) - returns function pointer as i64
//! - **Variant / Map and other heterogeneous values** (via the generic
//!   `env_push_value` path) - shipped in PR #402.
//!
//! Types still to be added:
//! - Closure (nested closures with their own environments)
//!
//! See <https://github.com/navicore/patch-seq> for roadmap.
//!
//! ## Module Layout
//!
//! Per-concern sub-modules:
//! - `env` — environment allocation (`create_env`), population (`env_set`),
//!   and the generic `env_get` reader.
//! - `accessors` — type-specific readers (`env_get_int`, `env_get_bool`,
//!   `env_get_float`, `env_get_string`, `env_get_quotation`,
//!   `env_push_string`, `env_push_value`) that avoid returning a full
//!   `Value` enum across the FFI boundary.
//! - `construct` — closure value creation: `make_closure` (wraps a
//!   prebuilt env) and `push_closure` (pops captures off the stack).

/// Maximum number of captured values allowed in a closure environment.
/// This prevents unbounded memory allocation and potential resource exhaustion.
pub const MAX_CAPTURES: usize = 1024;

mod accessors;
mod construct;
mod env;

pub use accessors::{
    patch_seq_env_get_bool, patch_seq_env_get_float, patch_seq_env_get_int,
    patch_seq_env_get_quotation, patch_seq_env_get_string, patch_seq_env_push_string,
    patch_seq_env_push_value,
};
pub use construct::{patch_seq_make_closure, patch_seq_push_closure};
pub use env::{patch_seq_create_env, patch_seq_env_get, patch_seq_env_set};

// Public re-exports with short names for internal use
pub use patch_seq_create_env as create_env;
pub use patch_seq_env_get as env_get;
pub use patch_seq_env_get_bool as env_get_bool;
pub use patch_seq_env_get_float as env_get_float;
pub use patch_seq_env_get_int as env_get_int;
pub use patch_seq_env_get_quotation as env_get_quotation;
pub use patch_seq_env_get_string as env_get_string;
pub use patch_seq_env_push_string as env_push_string;
pub use patch_seq_env_push_value as env_push_value;
pub use patch_seq_env_set as env_set;
pub use patch_seq_make_closure as make_closure;
pub use patch_seq_push_closure as push_closure;

#[cfg(test)]
mod tests;
