//! String and Symbol Global Handling
//!
//! This module handles deduplication of string and symbol literals
//! as LLVM IR global constants.

use super::{CodeGen, CodeGenError};
use std::fmt::Write as _;

impl CodeGen {
    /// Escape a byte slice for LLVM IR string literals.
    ///
    /// Byte-by-byte: printable ASCII (excluding `"` and `\`) emits as the
    /// raw character; everything else emits as `\NN` hex. Embedded NULs
    /// and high-bit bytes (0x80–0xFF) round-trip exactly — Seq strings
    /// are byte-clean.
    pub(super) fn escape_llvm_string(bytes: &[u8]) -> Result<String, std::fmt::Error> {
        let mut result = String::new();
        for &b in bytes {
            match b {
                // LLVM IR string constants only spec the `\NN` hex escape.
                // `\\` happens to be tolerated by the assembler today but
                // isn't documented; emit `\5C` to stay strictly on-spec.
                b'\\' => result.push_str(r"\5C"),
                b'"' => result.push_str(r#"\22"#),
                // Printable ASCII excluding `"` (0x22) and `\` (0x5C).
                0x20..=0x21 | 0x23..=0x5B | 0x5D..=0x7E => result.push(b as char),
                _ => write!(&mut result, r"\{:02X}", b)?,
            }
        }
        Ok(result)
    }

    /// Get or create a global byte-string constant.
    ///
    /// Storage layout is `[len+1 x i8]` with a trailing NUL — that way
    /// C-string consumers (variant-tag comparison, symbol push) keep
    /// working without breaking embedded NULs in byte literals, since
    /// the codegen for byte literals passes an explicit length to the
    /// runtime and ignores the trailing NUL.
    pub(super) fn get_string_global(&mut self, bytes: &[u8]) -> Result<String, CodeGenError> {
        if let Some(global_name) = self.string_constants.get(bytes) {
            return Ok(global_name.clone());
        }

        let global_name = format!("@.str.{}", self.string_counter);
        self.string_counter += 1;

        let escaped = Self::escape_llvm_string(bytes)?;
        let len = bytes.len() + 1; // +1 for trailing NUL

        writeln!(
            &mut self.string_globals,
            "{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            global_name, len, escaped
        )?;

        self.string_constants
            .insert(bytes.to_vec(), global_name.clone());
        Ok(global_name)
    }

    /// Get or create a global interned symbol constant (Issue #166)
    ///
    /// Creates a static SeqString structure with capacity=0 to mark it as interned.
    /// This enables O(1) symbol equality via pointer comparison.
    pub(super) fn get_symbol_global(&mut self, symbol_name: &str) -> Result<String, CodeGenError> {
        // Deduplicate: return existing global if we've seen this symbol
        if let Some(global_name) = self.symbol_constants.get(symbol_name) {
            return Ok(global_name.clone());
        }

        // Get or create the underlying string data
        let str_global = self.get_string_global(symbol_name.as_bytes())?;

        // Create the SeqString structure global
        let sym_global = format!("@.sym.{}", self.symbol_counter);
        self.symbol_counter += 1;

        // SeqString layout: { ptr, i64 len, i64 capacity, i8 global }
        // capacity=0 marks this as an interned symbol (never freed)
        // global=1 marks it as static data
        writeln!(
            &mut self.symbol_globals,
            "{} = private unnamed_addr constant {{ ptr, i64, i64, i8 }} {{ ptr {}, i64 {}, i64 0, i8 1 }}",
            sym_global,
            str_global,
            symbol_name.len()
        )?;

        self.symbol_constants
            .insert(symbol_name.to_string(), sym_global.clone());
        Ok(sym_global)
    }

    /// Generate LLVM IR for entire program
    pub(super) fn emit_string_and_symbol_globals(
        &self,
        ir: &mut String,
    ) -> Result<(), CodeGenError> {
        // String constants
        if !self.string_globals.is_empty() {
            ir.push_str(&self.string_globals);
            writeln!(ir)?;
        }

        // Symbol constants (interned symbols for O(1) equality)
        if !self.symbol_globals.is_empty() {
            ir.push_str(&self.symbol_globals);
            writeln!(ir)?;
        }
        Ok(())
    }
}
