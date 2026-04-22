//! FFI (Foreign Function Interface) Support
//!
//! This module handles parsing of FFI manifests and generating the LLVM IR
//! for calling external C functions from Seq code.
//!
//! FFI is purely a compiler/linker concern - the runtime remains free of
//! external dependencies.
//!
//! # Usage
//!
//! ```seq
//! include ffi:libedit
//!
//! : repl ( -- )
//!   "prompt> " readline
//!   dup string-empty not if
//!     dup add-history
//!     process-input
//!     repl
//!   else
//!     drop
//!   then
//! ;
//! ```

mod bindings;
mod manifest;

#[cfg(test)]
mod tests;

pub use bindings::{FfiBindings, FfiFunctionInfo};
pub use manifest::{
    FfiArg, FfiFunction, FfiLibrary, FfiManifest, FfiReturn, FfiType, LIBEDIT_MANIFEST, Ownership,
    PassMode, get_ffi_manifest, has_ffi_manifest, list_ffi_manifests,
};
