//! Resolved FFI bindings ready for codegen: the `FfiBindings` registry plus
//! the per-function info generated from a manifest.

use std::collections::HashMap;

use crate::types::Effect;

use super::manifest::{FfiArg, FfiManifest, FfiReturn};

// ============================================================================
// FFI Code Generation
// ============================================================================

/// Resolved FFI bindings ready for code generation
#[derive(Debug, Clone)]
pub struct FfiBindings {
    /// Map from Seq word name to C function info
    pub functions: HashMap<String, FfiFunctionInfo>,
    /// Linker flags to add
    pub linker_flags: Vec<String>,
}

/// Information about an FFI function for code generation
#[derive(Debug, Clone)]
pub struct FfiFunctionInfo {
    /// C function name
    pub c_name: String,
    /// Seq word name
    pub seq_name: String,
    /// Stack effect for type checking
    pub effect: Effect,
    /// Arguments
    pub args: Vec<FfiArg>,
    /// Return specification
    pub return_spec: Option<FfiReturn>,
}

impl FfiBindings {
    /// Create empty bindings
    pub fn new() -> Self {
        FfiBindings {
            functions: HashMap::new(),
            linker_flags: Vec::new(),
        }
    }

    /// Add bindings from a manifest
    pub fn add_manifest(&mut self, manifest: &FfiManifest) -> Result<(), String> {
        // Add linker flags
        self.linker_flags.extend(manifest.linker_flags());

        // Add function bindings
        for func in manifest.functions() {
            let effect = func.effect()?;
            let info = FfiFunctionInfo {
                c_name: func.c_name.clone(),
                seq_name: func.seq_name.clone(),
                effect,
                args: func.args.clone(),
                return_spec: func.return_spec.clone(),
            };

            if self.functions.contains_key(&func.seq_name) {
                return Err(format!(
                    "FFI function '{}' is already defined",
                    func.seq_name
                ));
            }

            self.functions.insert(func.seq_name.clone(), info);
        }

        Ok(())
    }

    /// Check if a word is an FFI function
    pub fn is_ffi_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get all FFI function names for AST validation
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for FfiBindings {
    fn default() -> Self {
        Self::new()
    }
}
