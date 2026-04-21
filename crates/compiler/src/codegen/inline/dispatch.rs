//! Inline Operation Dispatch
//!
//! `try_codegen_inline_op` inspects the word name and — when it matches a
//! recognised operation — delegates to an inline helper. Helpers for
//! arithmetic/comparison live in `ops.rs`; helpers for stack shuffles,
//! boolean logic, unary integer ops, aux-slot transfer, and pick/roll
//! live in `shuffle.rs`. Any unrecognised name falls back to a runtime
//! call at the caller level.

use super::super::{CodeGen, CodeGenError};

impl CodeGen {
    /// Try to generate inline code for a tagged stack operation.
    /// Returns `Some(result_var)` if the operation was inlined, `None` otherwise.
    pub(in crate::codegen) fn try_codegen_inline_op(
        &mut self,
        stack_var: &str,
        name: &str,
    ) -> Result<Option<String>, CodeGenError> {
        match name {
            // Stack shuffles
            "drop" => self.codegen_inline_drop(stack_var),
            "dup" => self.codegen_inline_dup(stack_var),
            "swap" => self.codegen_inline_swap(stack_var),
            "over" => self.codegen_inline_over(stack_var),
            "rot" => self.codegen_inline_rot(stack_var),
            "nip" => self.codegen_inline_nip(stack_var),
            "tuck" => self.codegen_inline_tuck(stack_var),
            "2dup" => self.codegen_inline_two_dup(stack_var),
            "3drop" => self.codegen_inline_three_drop(stack_var),

            // Integer arithmetic
            "i.add" | "i.+" => self.codegen_inline_binary_op(stack_var, "add", "sub"),
            "i.subtract" | "i.-" => self.codegen_inline_binary_op(stack_var, "sub", "add"),
            "i.multiply" | "i.*" => self.codegen_inline_binary_op(stack_var, "mul", "div"),

            // Integer comparisons
            "i.=" | "i.eq" => self.codegen_inline_comparison(stack_var, "eq"),
            "i.<>" | "i.neq" => self.codegen_inline_comparison(stack_var, "ne"),
            "i.<" | "i.lt" => self.codegen_inline_comparison(stack_var, "slt"),
            "i.>" | "i.gt" => self.codegen_inline_comparison(stack_var, "sgt"),
            "i.<=" | "i.lte" => self.codegen_inline_comparison(stack_var, "sle"),
            "i.>=" | "i.gte" => self.codegen_inline_comparison(stack_var, "sge"),

            // Float arithmetic
            "f.add" | "f.+" => self.codegen_inline_float_binary_op(stack_var, "fadd"),
            "f.subtract" | "f.-" => self.codegen_inline_float_binary_op(stack_var, "fsub"),
            "f.multiply" | "f.*" => self.codegen_inline_float_binary_op(stack_var, "fmul"),
            "f.divide" | "f./" => self.codegen_inline_float_binary_op(stack_var, "fdiv"),

            // Float comparisons
            "f.=" | "f.eq" => self.codegen_inline_float_comparison(stack_var, "oeq"),
            "f.<>" | "f.neq" => self.codegen_inline_float_comparison(stack_var, "one"),
            "f.<" | "f.lt" => self.codegen_inline_float_comparison(stack_var, "olt"),
            "f.>" | "f.gt" => self.codegen_inline_float_comparison(stack_var, "ogt"),
            "f.<=" | "f.lte" => self.codegen_inline_float_comparison(stack_var, "ole"),
            "f.>=" | "f.gte" => self.codegen_inline_float_comparison(stack_var, "oge"),

            // Boolean
            "and" => self.codegen_inline_bool_binary(stack_var, "and"),
            "or" => self.codegen_inline_bool_binary(stack_var, "or"),
            "not" => self.codegen_inline_not(stack_var),

            // Bitwise
            "band" => self.codegen_inline_int_bitwise_binary(stack_var, "and"),
            "bor" => self.codegen_inline_int_bitwise_binary(stack_var, "or"),
            "bxor" => self.codegen_inline_int_bitwise_binary(stack_var, "xor"),
            "shl" => self.codegen_inline_shift(stack_var, true),
            "shr" => self.codegen_inline_shift(stack_var, false),

            // Integer unary
            "i.neg" | "negate" => self.codegen_inline_negate(stack_var),
            "bnot" => self.codegen_inline_bnot(stack_var),

            // Bit-count intrinsics
            "popcount" => self.codegen_inline_int_unary_intrinsic(stack_var, "llvm.ctpop.i64"),
            "clz" => self.codegen_inline_int_unary_intrinsic(stack_var, "llvm.ctlz.i64"),
            "ctz" => self.codegen_inline_int_unary_intrinsic(stack_var, "llvm.cttz.i64"),

            // pick / roll: prefer the constant-N path when the previous
            // statement pushed a literal, otherwise fall back to the
            // dynamic-N helper that reads N off the stack.
            "pick" => {
                let stack_var = self.spill_virtual_stack(stack_var)?;
                if let Some(n) = self.prev_stmt_int_value
                    && n >= 0
                {
                    return self.codegen_pick_constant(&stack_var, n as usize);
                }
                self.codegen_pick_dynamic(&stack_var)
            }
            "roll" => {
                let stack_var = self.spill_virtual_stack(stack_var)?;
                if let Some(n) = self.prev_stmt_int_value
                    && n >= 0
                {
                    return self.codegen_roll_constant(&stack_var, n as usize);
                }
                self.codegen_roll_dynamic(&stack_var)
            }

            // Aux slots
            ">aux" => self.codegen_inline_aux_push(stack_var),
            "aux>" => self.codegen_inline_aux_pop(stack_var),

            // Not an inline-able operation
            _ => Ok(None),
        }
    }
}
