//! The top-level specialized codegen: walking a word definition, dispatching
//! on statement kind, generating the `define … { … }` prologue, lowering
//! `if / else`, and emitting returns. The per-operation and per-call
//! emitters live in sibling files (`codegen_ops`, `codegen_safe_math`,
//! `codegen_calls`).

use super::CodeGen;
use super::context::RegisterContext;
use super::types::{RegisterType, SpecSignature};
use crate::ast::{Statement, WordDef};
use crate::codegen::CodeGenError;
use crate::codegen::mangle_name;
use std::fmt::Write as _;

impl CodeGen {
    /// Generate a specialized version of a word.
    ///
    /// This creates a register-based function that passes values directly in
    /// CPU registers instead of through the tagged pointer stack.
    ///
    /// The generated function:
    /// - Takes primitive arguments directly (i64 for Int/Bool, double for Float)
    /// - Returns the result in a register (not via stack pointer)
    /// - Uses `musttail` for recursive calls to guarantee TCO
    /// - Handles control flow with phi nodes for value merging
    pub fn codegen_specialized_word(
        &mut self,
        word: &WordDef,
        sig: &SpecSignature,
    ) -> Result<(), CodeGenError> {
        let base_name = format!("seq_{}", mangle_name(&word.name));
        let spec_name = format!("{}{}", base_name, sig.suffix());

        // Generate function signature
        // For single output: define i64 @name(i64 %arg0) {
        // For multiple outputs: define { i64, i64 } @name(i64 %arg0, i64 %arg1) {
        let return_type = if sig.outputs.len() == 1 {
            sig.outputs[0].llvm_type().to_string()
        } else {
            let types: Vec<_> = sig.outputs.iter().map(|t| t.llvm_type()).collect();
            format!("{{ {} }}", types.join(", "))
        };

        let params: Vec<String> = sig
            .inputs
            .iter()
            .enumerate()
            .map(|(i, ty)| format!("{} %arg{}", ty.llvm_type(), i))
            .collect();

        writeln!(
            &mut self.output,
            "define {} @{}({}) {{",
            return_type,
            spec_name,
            params.join(", ")
        )?;
        writeln!(&mut self.output, "entry:")?;

        let initial_params: Vec<(String, RegisterType)> = sig
            .inputs
            .iter()
            .enumerate()
            .map(|(i, ty)| (format!("arg{}", i), *ty))
            .collect();
        let mut ctx = RegisterContext::from_params(&initial_params);

        let body_len = word.body.len();
        let mut prev_int_literal: Option<i64> = None;
        for (i, stmt) in word.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            self.codegen_specialized_statement(
                &mut ctx,
                stmt,
                &word.name,
                sig,
                is_last,
                &mut prev_int_literal,
            )?;
        }

        writeln!(&mut self.output, "}}")?;
        writeln!(&mut self.output)?;

        // Record that this word is specialized
        self.specialized_words
            .insert(word.name.clone(), sig.clone());

        Ok(())
    }

    /// Generate specialized code for a single statement
    pub(super) fn codegen_specialized_statement(
        &mut self,
        ctx: &mut RegisterContext,
        stmt: &Statement,
        word_name: &str,
        sig: &SpecSignature,
        is_last: bool,
        prev_int_literal: &mut Option<i64>,
    ) -> Result<(), CodeGenError> {
        // Track previous int literal for pick/roll optimization
        let prev_int = *prev_int_literal;
        *prev_int_literal = None; // Reset unless this is an IntLiteral

        match stmt {
            Statement::IntLiteral(n) => {
                let var = self.fresh_temp();
                writeln!(&mut self.output, "  %{} = add i64 0, {}", var, n)?;
                ctx.push(var, RegisterType::I64);
                *prev_int_literal = Some(*n); // Track for next statement
            }

            Statement::FloatLiteral(f) => {
                let var = self.fresh_temp();
                // Use bitcast from integer bits for exact IEEE 754 representation.
                // This avoids precision loss from decimal string conversion (e.g., 0.1
                // cannot be exactly represented in binary floating point). By storing
                // the raw bit pattern and using bitcast, we preserve the exact value.
                let bits = f.to_bits();
                writeln!(
                    &mut self.output,
                    "  %{} = bitcast i64 {} to double",
                    var, bits
                )?;
                ctx.push(var, RegisterType::Double);
            }

            Statement::BoolLiteral(b) => {
                let var = self.fresh_temp();
                let val = if *b { 1 } else { 0 };
                writeln!(&mut self.output, "  %{} = add i64 0, {}", var, val)?;
                ctx.push(var, RegisterType::I64);
            }

            Statement::WordCall { name, .. } => {
                self.codegen_specialized_word_call(ctx, name, word_name, sig, is_last, prev_int)?;
            }

            Statement::If {
                then_branch,
                else_branch,
                span: _,
            } => {
                self.codegen_specialized_if(
                    ctx,
                    then_branch,
                    else_branch.as_ref(),
                    word_name,
                    sig,
                    is_last,
                )?;
            }

            // These shouldn't appear in specializable words (checked in can_specialize)
            Statement::StringLiteral(_)
            | Statement::Symbol(_)
            | Statement::Quotation { .. }
            | Statement::Match { .. } => {
                return Err(CodeGenError::Logic(format!(
                    "Non-specializable statement in specialized word: {:?}",
                    stmt
                )));
            }
        }

        // Emit return if this is the last statement and it's not a control flow op
        // that already emits returns (like if, or recursive calls)
        let already_returns = match stmt {
            Statement::If { .. } => true,
            Statement::WordCall { name, .. } if name == word_name => true,
            _ => false,
        };
        if is_last && !already_returns {
            self.emit_specialized_return(ctx, sig)?;
        }

        Ok(())
    }

    /// Emit return statement for specialized function
    pub(super) fn emit_specialized_return(
        &mut self,
        ctx: &RegisterContext,
        sig: &SpecSignature,
    ) -> Result<(), CodeGenError> {
        let output_count = sig.outputs.len();

        if output_count == 0 {
            writeln!(&mut self.output, "  ret void")?;
        } else if output_count == 1 {
            let (var, ty) = ctx
                .values
                .last()
                .ok_or_else(|| CodeGenError::Logic("Empty context at return".to_string()))?;
            writeln!(&mut self.output, "  ret {} %{}", ty.llvm_type(), var)?;
        } else {
            // Multi-output: build struct return.
            // Values in context are bottom-to-top, matching sig.outputs order.
            if ctx.values.len() < output_count {
                return Err(CodeGenError::Logic(format!(
                    "Not enough values for multi-output return: need {}, have {}",
                    output_count,
                    ctx.values.len()
                )));
            }

            let start_idx = ctx.values.len() - output_count;
            let return_values: Vec<_> = ctx.values[start_idx..].to_vec();

            let struct_type = sig.llvm_return_type();

            let mut current_struct = "undef".to_string();
            for (i, (var, ty)) in return_values.iter().enumerate() {
                let new_struct = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = insertvalue {} {}, {} %{}, {}",
                    new_struct,
                    struct_type,
                    current_struct,
                    ty.llvm_type(),
                    var,
                    i
                )?;
                current_struct = format!("%{}", new_struct);
            }

            writeln!(&mut self.output, "  ret {} {}", struct_type, current_struct)?;
        }
        Ok(())
    }

    /// Generate specialized if/else statement
    pub(super) fn codegen_specialized_if(
        &mut self,
        ctx: &mut RegisterContext,
        then_branch: &[Statement],
        else_branch: Option<&Vec<Statement>>,
        word_name: &str,
        sig: &SpecSignature,
        is_last: bool,
    ) -> Result<(), CodeGenError> {
        // Pop condition
        let (cond_var, _) = ctx
            .pop()
            .ok_or_else(|| CodeGenError::Logic("Empty context at if condition".to_string()))?;

        let cmp_result = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = icmp ne i64 %{}, 0",
            cmp_result, cond_var
        )?;

        let then_label = self.fresh_block("if_then");
        let else_label = self.fresh_block("if_else");
        let merge_label = self.fresh_block("if_merge");

        writeln!(
            &mut self.output,
            "  br i1 %{}, label %{}, label %{}",
            cmp_result, then_label, else_label
        )?;

        // Then branch
        writeln!(&mut self.output, "{}:", then_label)?;
        let mut then_ctx = ctx.clone();
        let mut then_prev_int: Option<i64> = None;
        for (i, stmt) in then_branch.iter().enumerate() {
            let is_stmt_last = i == then_branch.len() - 1 && is_last;
            self.codegen_specialized_statement(
                &mut then_ctx,
                stmt,
                word_name,
                sig,
                is_stmt_last,
                &mut then_prev_int,
            )?;
        }
        // If the then branch is empty and this is the last statement, emit return
        if is_last && then_branch.is_empty() {
            self.emit_specialized_return(&then_ctx, sig)?;
        }
        let then_emitted_return = is_last;
        let then_pred = if then_emitted_return {
            None
        } else {
            writeln!(&mut self.output, "  br label %{}", merge_label)?;
            Some(then_label.clone())
        };

        // Else branch
        writeln!(&mut self.output, "{}:", else_label)?;
        let mut else_ctx = ctx.clone();
        let mut else_prev_int: Option<i64> = None;
        if let Some(else_stmts) = else_branch {
            for (i, stmt) in else_stmts.iter().enumerate() {
                let is_stmt_last = i == else_stmts.len() - 1 && is_last;
                self.codegen_specialized_statement(
                    &mut else_ctx,
                    stmt,
                    word_name,
                    sig,
                    is_stmt_last,
                    &mut else_prev_int,
                )?;
            }
        }
        // If the else branch is empty (or None) and this is the last statement, emit return
        if is_last && (else_branch.is_none() || else_branch.as_ref().is_some_and(|b| b.is_empty()))
        {
            self.emit_specialized_return(&else_ctx, sig)?;
        }
        let else_emitted_return = is_last;
        let else_pred = if else_emitted_return {
            None
        } else {
            writeln!(&mut self.output, "  br label %{}", merge_label)?;
            Some(else_label.clone())
        };

        // Merge block with phi nodes if either branch continues
        if then_pred.is_some() || else_pred.is_some() {
            writeln!(&mut self.output, "{}:", merge_label)?;

            if let (Some(then_p), Some(else_p)) = (&then_pred, &else_pred) {
                // Both branches continue - merge all values with phi nodes
                if then_ctx.values.len() != else_ctx.values.len() {
                    return Err(CodeGenError::Logic(format!(
                        "Stack depth mismatch in if branches: then has {}, else has {}",
                        then_ctx.values.len(),
                        else_ctx.values.len()
                    )));
                }

                ctx.values.clear();
                for i in 0..then_ctx.values.len() {
                    let (then_var, then_ty) = &then_ctx.values[i];
                    let (else_var, else_ty) = &else_ctx.values[i];

                    if then_ty != else_ty {
                        return Err(CodeGenError::Logic(format!(
                            "Type mismatch at position {} in if branches: {:?} vs {:?}",
                            i, then_ty, else_ty
                        )));
                    }

                    if then_var == else_var {
                        ctx.push(then_var.clone(), *then_ty);
                    } else {
                        let phi_result = self.fresh_temp();
                        writeln!(
                            &mut self.output,
                            "  %{} = phi {} [ %{}, %{} ], [ %{}, %{} ]",
                            phi_result,
                            then_ty.llvm_type(),
                            then_var,
                            then_p,
                            else_var,
                            else_p
                        )?;
                        ctx.push(phi_result, *then_ty);
                    }
                }
            } else if then_pred.is_some() {
                *ctx = then_ctx;
            } else {
                *ctx = else_ctx;
            }

            if is_last && (then_pred.is_some() || else_pred.is_some()) {
                self.emit_specialized_return(ctx, sig)?;
            }
        }

        Ok(())
    }
}
