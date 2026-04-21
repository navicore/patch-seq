//! Inline helpers for stack-shuffle, boolean, unary-integer, aux-slot, and
//! pick/roll operations. `dispatch::try_codegen_inline_op` dispatches to
//! these; they each take the current SP variable and return the new SP (or
//! the same one, for in-place ops).

use super::super::{CodeGen, CodeGenError};
use std::fmt::Write as _;

impl CodeGen {
    // =========================================================================
    // Stack shuffles
    // =========================================================================

    /// drop: ( a -- ). Calls the runtime so heap values are properly dropped.
    pub(super) fn codegen_inline_drop(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let result_var = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = call ptr @patch_seq_drop_op(ptr %{})",
            result_var, stack_var
        )?;
        Ok(Some(result_var))
    }

    /// dup: ( a -- a a ). Uses a fast raw-value copy when the top is known
    /// trivially copyable (literal or primitive), otherwise delegates to the
    /// runtime clone.
    pub(super) fn codegen_inline_dup(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let top_ptr = self.emit_stack_gep(stack_var, -1)?;

        let use_fast_path =
            self.prev_stmt_is_trivial_literal || self.is_trivially_copyable_at_current_stmt();

        if use_fast_path {
            let val = self.emit_load_value(&top_ptr)?;
            self.emit_store_value(stack_var, &val)?;
        } else {
            writeln!(
                &mut self.output,
                "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
                top_ptr, stack_var
            )?;
        }

        let result_var = self.emit_stack_gep(stack_var, 1)?;
        Ok(Some(result_var))
    }

    /// swap: ( a b -- b a )
    pub(super) fn codegen_inline_swap(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let ptr_b = self.emit_stack_gep(stack_var, -1)?;
        let ptr_a = self.emit_stack_gep(stack_var, -2)?;
        let val_a = self.emit_load_value(&ptr_a)?;
        let val_b = self.emit_load_value(&ptr_b)?;
        self.emit_store_value(&ptr_a, &val_b)?;
        self.emit_store_value(&ptr_b, &val_a)?;
        Ok(Some(stack_var.to_string()))
    }

    /// over: ( a b -- a b a )
    pub(super) fn codegen_inline_over(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let ptr_a = self.emit_stack_gep(stack_var, -2)?;
        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            ptr_a, stack_var
        )?;
        let result_var = self.emit_stack_gep(stack_var, 1)?;
        Ok(Some(result_var))
    }

    /// rot: ( a b c -- b c a )
    pub(super) fn codegen_inline_rot(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let ptr_c = self.emit_stack_gep(stack_var, -1)?;
        let ptr_b = self.emit_stack_gep(stack_var, -2)?;
        let ptr_a = self.emit_stack_gep(stack_var, -3)?;

        let val_a = self.emit_load_value(&ptr_a)?;
        let val_b = self.emit_load_value(&ptr_b)?;
        let val_c = self.emit_load_value(&ptr_c)?;

        self.emit_store_value(&ptr_a, &val_b)?;
        self.emit_store_value(&ptr_b, &val_c)?;
        self.emit_store_value(&ptr_c, &val_a)?;

        Ok(Some(stack_var.to_string()))
    }

    /// nip: ( a b -- b )
    pub(super) fn codegen_inline_nip(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let result_var = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = call ptr @patch_seq_nip(ptr %{})",
            result_var, stack_var
        )?;
        Ok(Some(result_var))
    }

    /// tuck: ( a b -- b a b )
    pub(super) fn codegen_inline_tuck(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let ptr_b = self.emit_stack_gep(stack_var, -1)?;
        let ptr_a = self.emit_stack_gep(stack_var, -2)?;

        let val_a = self.emit_load_value(&ptr_a)?;
        let val_b = self.emit_load_value(&ptr_b)?;

        // Clone b to the new top position
        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            ptr_b, stack_var
        )?;

        // Result: b a b
        self.emit_store_value(&ptr_a, &val_b)?;
        self.emit_store_value(&ptr_b, &val_a)?;

        let result_var = self.emit_stack_gep(stack_var, 1)?;
        Ok(Some(result_var))
    }

    /// 2dup: ( a b -- a b a b )
    pub(super) fn codegen_inline_two_dup(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let ptr_b = self.emit_stack_gep(stack_var, -1)?;
        let ptr_a = self.emit_stack_gep(stack_var, -2)?;

        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            ptr_a, stack_var
        )?;
        let new_ptr = self.emit_stack_gep(stack_var, 1)?;
        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            ptr_b, new_ptr
        )?;

        let result_var = self.emit_stack_gep(stack_var, 2)?;
        Ok(Some(result_var))
    }

    /// 3drop: ( a b c -- ). Three runtime drop calls in sequence.
    pub(super) fn codegen_inline_three_drop(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;

        let drop1 = self.fresh_temp();
        let drop2 = self.fresh_temp();
        let drop3 = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = call ptr @patch_seq_drop_op(ptr %{})",
            drop1, stack_var
        )?;
        writeln!(
            &mut self.output,
            "  %{} = call ptr @patch_seq_drop_op(ptr %{})",
            drop2, drop1
        )?;
        writeln!(
            &mut self.output,
            "  %{} = call ptr @patch_seq_drop_op(ptr %{})",
            drop3, drop2
        )?;
        Ok(Some(drop3))
    }

    // =========================================================================
    // Boolean logic
    // =========================================================================

    /// Boolean binary op (`and` / `or`) on two values. `llvm_op` is `"and"`
    /// or `"or"`. Result is normalised to 0/1 and stored as a Bool at the
    /// lower operand slot.
    pub(super) fn codegen_inline_bool_binary(
        &mut self,
        stack_var: &str,
        llvm_op: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let (ptr_a, val_a, val_b) = self.emit_load_two_int_operands(stack_var)?;

        let combined = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = {} i64 %{}, %{}",
            combined, llvm_op, val_a, val_b
        )?;
        let bool_result = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = icmp ne i64 %{}, 0",
            bool_result, combined
        )?;
        let zext = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = zext i1 %{} to i64",
            zext, bool_result
        )?;

        self.emit_store_bool(&ptr_a, &zext)?;
        let result_var = self.emit_stack_gep(stack_var, -1)?;
        Ok(Some(result_var))
    }

    /// not: ( a -- !a ). True iff a was zero.
    pub(super) fn codegen_inline_not(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let (top_ptr, val) = self.emit_load_top_int(stack_var)?;

        let is_zero = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = icmp eq i64 %{}, 0", is_zero, val)?;
        let zext = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = zext i1 %{} to i64",
            zext, is_zero
        )?;

        self.emit_store_bool(&top_ptr, &zext)?;
        Ok(Some(stack_var.to_string()))
    }

    // =========================================================================
    // Integer unary
    // =========================================================================

    /// i.neg / negate: ( a -- -a ).
    pub(super) fn codegen_inline_negate(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let (top_ptr, val) = self.emit_load_top_int(stack_var)?;
        let neg_result = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = sub i64 0, %{}", neg_result, val)?;
        self.emit_store_int_result_in_place(&top_ptr, &neg_result)?;
        Ok(Some(stack_var.to_string()))
    }

    /// bnot: ( a -- ~a ). Bitwise NOT via xor with -1.
    pub(super) fn codegen_inline_bnot(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let (top_ptr, val) = self.emit_load_top_int(stack_var)?;
        let not_result = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = xor i64 %{}, -1", not_result, val)?;
        self.emit_store_int_result_in_place(&top_ptr, &not_result)?;
        Ok(Some(stack_var.to_string()))
    }

    // =========================================================================
    // Aux slot transfer
    // =========================================================================

    /// >aux: ( T -- ). Move top of main stack to the current aux slot.
    pub(super) fn codegen_inline_aux_push(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        let slot_idx = self.current_aux_sp;
        let slot_name = self.current_aux_slots[slot_idx].clone();

        let top_ptr = self.emit_stack_gep(stack_var, -1)?;
        let val = self.emit_load_value(&top_ptr)?;

        self.emit_store_value(&slot_name, &val)?;
        self.current_aux_sp += 1;

        // Reuse top_ptr as the new SP — it already points to sp-1 (the slot
        // we just consumed), so no additional GEP needed.
        Ok(Some(top_ptr))
    }

    /// aux>: ( -- T ). Move top of aux stack to main stack.
    pub(super) fn codegen_inline_aux_pop(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let stack_var = self.spill_virtual_stack(stack_var)?;
        let stack_var = stack_var.as_str();

        debug_assert!(
            self.current_aux_sp > 0,
            "aux>: aux stack underflow (typechecker should have caught this)"
        );
        self.current_aux_sp -= 1;
        let slot_idx = self.current_aux_sp;
        let slot_name = self.current_aux_slots[slot_idx].clone();

        let val = self.emit_load_value(&slot_name)?;
        self.emit_store_value(stack_var, &val)?;

        let result_var = self.emit_stack_gep(stack_var, 1)?;
        Ok(Some(result_var))
    }

    // =========================================================================
    // pick / roll
    // =========================================================================

    /// pick with runtime-N: ( ... xn ... x1 x0 n -- ... xn ... x1 x0 xn )
    pub(super) fn codegen_pick_dynamic(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let n_ptr = self.emit_stack_gep(stack_var, -1)?;
        let n_val = self.emit_load_int_payload(&n_ptr)?;

        // Calculate offset: -(n + 2) from stack_var
        let offset = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = add i64 %{}, 2", offset, n_val)?;
        let neg_offset = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = sub i64 0, %{}",
            neg_offset, offset
        )?;

        let src_ptr = self.emit_dynamic_stack_gep(stack_var, &neg_offset)?;

        // Clone value from src to n_ptr (replacing n)
        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            src_ptr, n_ptr
        )?;

        Ok(Some(stack_var.to_string()))
    }

    /// pick with constant N known at compile time (Issue #192).
    pub(super) fn codegen_pick_constant(
        &mut self,
        stack_var: &str,
        n: usize,
    ) -> Result<Option<String>, CodeGenError> {
        // Destination: replace n at top of stack (sp - 1)
        let n_ptr = self.emit_stack_gep(stack_var, -1)?;

        // Source offset: -(n + 2) from stack_var
        let neg_offset = -((n + 2) as i64);
        let src_ptr = self.emit_stack_gep(stack_var, neg_offset)?;

        writeln!(
            &mut self.output,
            "  call void @patch_seq_clone_value(ptr %{}, ptr %{})",
            src_ptr, n_ptr
        )?;

        Ok(Some(stack_var.to_string()))
    }

    /// roll with runtime-N: ( ... xn xn-1 ... x1 x0 n -- ... xn-1 ... x1 x0 xn )
    pub(super) fn codegen_roll_dynamic(
        &mut self,
        stack_var: &str,
    ) -> Result<Option<String>, CodeGenError> {
        let n_ptr = self.emit_stack_gep(stack_var, -1)?;
        let n_val = self.emit_load_int_payload(&n_ptr)?;

        // Pop n: new SP is stack_var - 1
        let popped_sp = self.emit_stack_gep(stack_var, -1)?;

        // Calculate offset to xn: -(n + 1) from popped_sp
        let offset = self.fresh_temp();
        writeln!(&mut self.output, "  %{} = add i64 %{}, 1", offset, n_val)?;
        let neg_offset = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = sub i64 0, %{}",
            neg_offset, offset
        )?;

        let src_ptr = self.emit_dynamic_stack_gep(&popped_sp, &neg_offset)?;

        // Load the value to roll
        let rolled_val = self.emit_load_value(&src_ptr)?;

        // memmove: shift items down
        let src_plus_one = self.emit_stack_gep(&src_ptr, 1)?;

        // Size in bytes = n * value_size
        let value_size = self.value_size_bytes();
        let size_bytes = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = mul i64 %{}, {}",
            size_bytes, n_val, value_size
        )?;

        writeln!(
            &mut self.output,
            "  call void @llvm.memmove.p0.p0.i64(ptr %{}, ptr %{}, i64 %{}, i1 false)",
            src_ptr, src_plus_one, size_bytes
        )?;

        // Store rolled value at top
        let top_ptr = self.emit_stack_gep(&popped_sp, -1)?;
        self.emit_store_value(&top_ptr, &rolled_val)?;

        Ok(Some(popped_sp))
    }

    /// roll with constant N known at compile time (Issue #192).
    pub(super) fn codegen_roll_constant(
        &mut self,
        stack_var: &str,
        n: usize,
    ) -> Result<Option<String>, CodeGenError> {
        // Pop the N value from stack
        let popped_sp = self.emit_stack_gep(stack_var, -1)?;

        match n {
            0 => Ok(Some(popped_sp)),
            1 => {
                // 1 roll = swap
                let ptr_b = self.emit_stack_gep(&popped_sp, -1)?;
                let ptr_a = self.emit_stack_gep(&popped_sp, -2)?;
                let val_a = self.emit_load_value(&ptr_a)?;
                let val_b = self.emit_load_value(&ptr_b)?;
                self.emit_store_value(&ptr_a, &val_b)?;
                self.emit_store_value(&ptr_b, &val_a)?;
                Ok(Some(popped_sp))
            }
            2 => {
                // 2 roll = rot
                let ptr_c = self.emit_stack_gep(&popped_sp, -1)?;
                let ptr_b = self.emit_stack_gep(&popped_sp, -2)?;
                let ptr_a = self.emit_stack_gep(&popped_sp, -3)?;
                let val_a = self.emit_load_value(&ptr_a)?;
                let val_b = self.emit_load_value(&ptr_b)?;
                let val_c = self.emit_load_value(&ptr_c)?;
                self.emit_store_value(&ptr_a, &val_b)?;
                self.emit_store_value(&ptr_b, &val_c)?;
                self.emit_store_value(&ptr_c, &val_a)?;
                Ok(Some(popped_sp))
            }
            _ => {
                // n >= 3: use memmove with constant offsets
                let neg_offset = -((n + 1) as i64);
                let src_ptr = self.emit_stack_gep(&popped_sp, neg_offset)?;

                let rolled_val = self.emit_load_value(&src_ptr)?;

                let src_plus_one = self.emit_stack_gep(&src_ptr, 1)?;
                let size_bytes = n as u64 * self.value_size_bytes();
                writeln!(
                    &mut self.output,
                    "  call void @llvm.memmove.p0.p0.i64(ptr %{}, ptr %{}, i64 {}, i1 false)",
                    src_ptr, src_plus_one, size_bytes
                )?;

                let top_ptr = self.emit_stack_gep(&popped_sp, -1)?;
                self.emit_store_value(&top_ptr, &rolled_val)?;

                Ok(Some(popped_sp))
            }
        }
    }
}
