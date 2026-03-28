//! Stack Value Layout Helpers
//!
//! Generates LLVM IR for 8-byte tagged pointer stack values.
//!
//! ## Encoding
//!
//! ```text
//! %Value = type i64
//! - Odd values: Int (value << 1 | 1)
//! - 0x0: Bool false
//! - 0x2: Bool true
//! - Even > 2: Heap pointer to Arc<Value>
//! - GEP stride: 8 bytes per i64
//! ```

use super::{CodeGen, CodeGenError};
use std::fmt::Write as _;

impl CodeGen {
    // =========================================================================
    // Type definition
    // =========================================================================

    /// Emit the %Value type definition.
    pub(super) fn emit_value_type_def(&self, ir: &mut String) -> Result<(), CodeGenError> {
        writeln!(ir, "; Value type (tagged pointer - 8 bytes)")?;
        writeln!(ir, "%Value = type i64")?;
        writeln!(ir)?;
        Ok(())
    }

    // =========================================================================
    // Stack pointer arithmetic
    // =========================================================================

    /// Emit a GEP to offset the stack pointer by N value slots.
    pub(super) fn emit_stack_gep(
        &mut self,
        base: &str,
        offset: i64,
    ) -> Result<String, CodeGenError> {
        let tmp = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = getelementptr i64, ptr %{}, i64 {}",
            tmp, base, offset
        )?;
        Ok(tmp)
    }

    /// Emit a GEP with a dynamic (runtime) offset in an SSA variable.
    pub(super) fn emit_dynamic_stack_gep(
        &mut self,
        base: &str,
        offset_var: &str,
    ) -> Result<String, CodeGenError> {
        let tmp = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = getelementptr i64, ptr %{}, i64 %{}",
            tmp, base, offset_var
        )?;
        Ok(tmp)
    }

    // =========================================================================
    // Value slot access
    // =========================================================================

    /// Load the integer payload from a value at the given stack pointer.
    /// Loads the tagged i64 and extracts via arithmetic shift.
    pub(super) fn emit_load_int_payload(
        &mut self,
        value_ptr: &str,
    ) -> Result<String, CodeGenError> {
        let tagged = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = load i64, ptr %{}",
            tagged, value_ptr
        )?;
        // Arithmetic shift right preserves sign for negative integers
        let val = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = ashr i64 %{}, 1", val, tagged)?;
        Ok(val)
    }

    /// Store an integer value at the given stack pointer.
    /// Writes tagged integer (value << 1 | 1).
    pub(super) fn emit_store_int(
        &mut self,
        value_ptr: &str,
        int_var: &str,
    ) -> Result<(), CodeGenError> {
        // Assumes value fits in 63-bit signed range (-(2^62) to 2^62-1).
        let shifted = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = shl i64 %{}, 1", shifted, int_var)?;
        let tagged = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = or i64 %{}, 1", tagged, shifted)?;
        writeln!(
            &mut self.output,
            "  store i64 %{}, ptr %{}",
            tagged, value_ptr
        )?;
        Ok(())
    }

    /// Store a boolean result at the given stack pointer.
    /// Writes 0 (false) or 2 (true). `bool_var` is an i64 holding 0 or 1.
    pub(super) fn emit_store_bool(
        &mut self,
        value_ptr: &str,
        bool_var: &str,
    ) -> Result<(), CodeGenError> {
        // false = 0, true = 2 → shift left by 1
        let tagged = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = shl i64 %{}, 1", tagged, bool_var)?;
        writeln!(
            &mut self.output,
            "  store i64 %{}, ptr %{}",
            tagged, value_ptr
        )?;
        Ok(())
    }

    // =========================================================================
    // Loading two operands from stack (binary ops pattern)
    // =========================================================================

    /// Load two integer payloads from the top two stack positions.
    pub(super) fn emit_load_two_int_operands(
        &mut self,
        stack_var: &str,
    ) -> Result<(String, String, String), CodeGenError> {
        let ptr_b = self.emit_stack_gep(stack_var, -1)?;
        let ptr_a = self.emit_stack_gep(stack_var, -2)?;
        let val_a = self.emit_load_int_payload(&ptr_a)?;
        let val_b = self.emit_load_int_payload(&ptr_b)?;
        Ok((ptr_a, val_a, val_b))
    }

    /// Load one integer payload from the top of the stack.
    pub(super) fn emit_load_top_int(
        &mut self,
        stack_var: &str,
    ) -> Result<(String, String), CodeGenError> {
        let top_ptr = self.emit_stack_gep(stack_var, -1)?;
        let val = self.emit_load_int_payload(&top_ptr)?;
        Ok((top_ptr, val))
    }

    /// Store an integer result in place at the top of stack (for unary ops).
    pub(super) fn emit_store_int_result_in_place(
        &mut self,
        top_ptr: &str,
        result_var: &str,
    ) -> Result<(), CodeGenError> {
        self.emit_store_int(top_ptr, result_var)
    }

    // =========================================================================
    // Full value load/store (for swap, rot, tuck, etc.)
    // =========================================================================

    /// Load a full opaque value from a stack pointer.
    pub(super) fn emit_load_value(&mut self, ptr: &str) -> Result<String, CodeGenError> {
        let val = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = load i64, ptr %{}", val, ptr)?;
        Ok(val)
    }

    /// Store a full opaque value to a stack pointer.
    pub(super) fn emit_store_value(&mut self, ptr: &str, val: &str) -> Result<(), CodeGenError> {
        writeln!(&mut self.output, "  store i64 %{}, ptr %{}", val, ptr)?;
        Ok(())
    }

    // =========================================================================
    // Array size calculation
    // =========================================================================

    /// Return the size of a single Value in bytes (for memmove calculations).
    pub(super) fn value_size_bytes(&self) -> u64 {
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_def() {
        let cg = CodeGen::new();
        let mut ir = String::new();
        cg.emit_value_type_def(&mut ir).unwrap();
        assert!(ir.contains("%Value = type i64"));
        assert!(ir.contains("8 bytes"));
    }

    #[test]
    fn test_stack_gep() {
        let mut cg = CodeGen::new();
        let tmp = cg.emit_stack_gep("sp", -1).unwrap();
        assert!(cg.output.contains("getelementptr i64"));
        assert!(!cg.output.contains("%Value"));
        assert!(cg.output.contains(&format!("%{}", tmp)));
    }

    #[test]
    fn test_load_int_payload() {
        let mut cg = CodeGen::new();
        let val = cg.emit_load_int_payload("ptr_a").unwrap();
        assert!(cg.output.contains("load i64, ptr %ptr_a"));
        assert!(cg.output.contains("ashr i64"));
        assert!(!val.is_empty());
    }

    #[test]
    fn test_store_int() {
        let mut cg = CodeGen::new();
        cg.emit_store_int("ptr_a", "val").unwrap();
        assert!(cg.output.contains("shl i64 %val, 1"));
        assert!(cg.output.contains("or i64"));
        assert!(!cg.output.contains("store i64 0, ptr"));
    }

    #[test]
    fn test_store_bool() {
        let mut cg = CodeGen::new();
        cg.emit_store_bool("ptr_a", "bval").unwrap();
        assert!(cg.output.contains("shl i64 %bval, 1"));
        assert!(!cg.output.contains("store i64 2"));
    }

    #[test]
    fn test_value_size_bytes() {
        assert_eq!(CodeGen::new().value_size_bytes(), 8);
    }

    #[test]
    fn test_load_two_int_operands() {
        let mut cg = CodeGen::new();
        let (_ptr_a, val_a, val_b) = cg.emit_load_two_int_operands("sp").unwrap();
        assert!(cg.output.contains("getelementptr i64"));
        assert!(cg.output.contains("ashr i64"));
        assert!(!val_a.is_empty());
        assert!(!val_b.is_empty());
    }

    #[test]
    fn test_load_value() {
        let mut cg = CodeGen::new();
        let val = cg.emit_load_value("ptr_a").unwrap();
        assert!(cg.output.contains("load i64, ptr %ptr_a"));
        assert!(!val.is_empty());
    }

    #[test]
    fn test_store_value() {
        let mut cg = CodeGen::new();
        cg.emit_store_value("ptr_a", "val").unwrap();
        assert!(cg.output.contains("store i64 %val, ptr %ptr_a"));
    }

    #[test]
    fn test_dynamic_stack_gep() {
        let mut cg = CodeGen::new();
        let tmp = cg.emit_dynamic_stack_gep("sp", "offset").unwrap();
        assert!(
            cg.output
                .contains("getelementptr i64, ptr %sp, i64 %offset")
        );
        assert!(!tmp.is_empty());
    }
}
