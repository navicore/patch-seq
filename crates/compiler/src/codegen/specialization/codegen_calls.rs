//! Recursive-self and cross-word call lowering for specialized codegen.
//! Both emit an LLVM `call`, pop arguments from the register context, and
//! push the result(s). The recursive path additionally uses `musttail` to
//! guarantee TCO for tail positions.

use super::CodeGen;
use super::context::RegisterContext;
use super::types::SpecSignature;
use crate::codegen::CodeGenError;
use crate::codegen::mangle_name;
use std::fmt::Write as _;

impl CodeGen {
    /// Emit a recursive call to the specialized version of the current word.
    ///
    /// Uses `musttail` when `is_tail` is true to guarantee tail call optimization.
    /// This is critical for recursive algorithms like `fib` or `count-down` that
    /// would otherwise overflow the call stack.
    pub(super) fn emit_specialized_recursive_call(
        &mut self,
        ctx: &mut RegisterContext,
        word_name: &str,
        sig: &SpecSignature,
        is_tail: bool,
    ) -> Result<(), CodeGenError> {
        let spec_name = format!("seq_{}{}", mangle_name(word_name), sig.suffix());

        if ctx.values.len() < sig.inputs.len() {
            return Err(CodeGenError::Logic(format!(
                "Not enough values in context for recursive call to {}: need {}, have {}",
                word_name,
                sig.inputs.len(),
                ctx.values.len()
            )));
        }

        let mut args = Vec::new();
        for _ in 0..sig.inputs.len() {
            args.push(ctx.pop().unwrap());
        }
        args.reverse();

        let arg_strs: Vec<String> = args
            .iter()
            .map(|(var, ty)| format!("{} %{}", ty.llvm_type(), var))
            .collect();

        let return_type = sig.llvm_return_type();

        if is_tail {
            // Tail call - use musttail for guaranteed TCO
            let result = self.fresh_temp();
            writeln!(
                &mut self.output,
                "  %{} = musttail call {} @{}({})",
                result,
                return_type,
                spec_name,
                arg_strs.join(", ")
            )?;
            writeln!(&mut self.output, "  ret {} %{}", return_type, result)?;
        } else {
            let result = self.fresh_temp();
            writeln!(
                &mut self.output,
                "  %{} = call {} @{}({})",
                result,
                return_type,
                spec_name,
                arg_strs.join(", ")
            )?;

            if sig.outputs.len() == 1 {
                ctx.push(result, sig.outputs[0]);
            } else {
                for (i, out_ty) in sig.outputs.iter().enumerate() {
                    let extracted = self.fresh_temp();
                    writeln!(
                        &mut self.output,
                        "  %{} = extractvalue {} %{}, {}",
                        extracted, return_type, result, i
                    )?;
                    ctx.push(extracted, *out_ty);
                }
            }
        }

        Ok(())
    }

    /// Emit a call to another specialized word.
    pub(super) fn emit_specialized_word_dispatch(
        &mut self,
        ctx: &mut RegisterContext,
        name: &str,
    ) -> Result<(), CodeGenError> {
        let sig = self
            .specialized_words
            .get(name)
            .ok_or_else(|| CodeGenError::Logic(format!("Unknown specialized word: {}", name)))?
            .clone();

        let spec_name = format!("seq_{}{}", mangle_name(name), sig.suffix());

        let mut args = Vec::new();
        for _ in 0..sig.inputs.len() {
            args.push(ctx.pop().unwrap());
        }
        args.reverse();

        let arg_strs: Vec<String> = args
            .iter()
            .map(|(var, ty)| format!("{} %{}", ty.llvm_type(), var))
            .collect();

        let return_type = sig.llvm_return_type();

        let result = self.fresh_temp();
        writeln!(
            &mut self.output,
            "  %{} = call {} @{}({})",
            result,
            return_type,
            spec_name,
            arg_strs.join(", ")
        )?;

        if sig.outputs.len() == 1 {
            ctx.push(result, sig.outputs[0]);
        } else {
            for (i, out_ty) in sig.outputs.iter().enumerate() {
                let extracted = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = extractvalue {} %{}, {}",
                    extracted, return_type, result, i
                )?;
                ctx.push(extracted, *out_ty);
            }
        }

        Ok(())
    }
}
