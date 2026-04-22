//! Dataflow combinators: dip, keep, bi.

use crate::types::{SideEffect, StackType, Type};
use crate::unification::Subst;

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn infer_dip(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop the quotation
        let (stack_after_quot, quot_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}dip: stack underflow - expected quotation on stack",
                line_prefix
            )
        })?;

        // Extract the quotation's effect
        let quot_effect = match &quot_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                // Unknown quotation type — fall back to generic builtin signature
                let effect = self
                    .lookup_word_effect("dip")
                    .ok_or_else(|| "Unknown word: 'dip'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "dip", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}dip: expected quotation or closure on top of stack, got {}",
                    line_prefix, quot_type
                ));
            }
        };

        if quot_effect.has_yield() {
            return Err("dip: quotation must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // Pop the preserved value (below the quotation)
        let (rest_stack, preserved_type) = stack_after_quot.clone().pop().ok_or_else(|| {
            format!(
                "{}dip: stack underflow - expected a value below the quotation",
                line_prefix
            )
        })?;

        // Freshen and apply the quotation's effect to the stack below the preserved value
        let fresh_effect = self.freshen_effect(&quot_effect);
        let (result_stack, subst) =
            self.apply_effect(&fresh_effect, rest_stack, "dip (quotation)", span)?;

        // Push the preserved value back on top, applying substitution in case
        // preserved_type contains type variables resolved during unification
        let resolved_preserved = subst.apply_type(&preserved_type);
        let result_stack = result_stack.push(resolved_preserved);

        let propagated_effects = fresh_effect.effects.clone();
        Ok((result_stack, subst, propagated_effects))
    }

    /// Infer the stack effect of `keep`: ( ..a x quot -- ..b x )
    ///
    /// Run the quotation on the value (quotation receives x), then
    /// restore the original value on top. Like `over >aux call aux>`.
    pub(super) fn infer_keep(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop the quotation
        let (stack_after_quot, quot_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}keep: stack underflow - expected quotation on stack",
                line_prefix
            )
        })?;

        // Extract the quotation's effect
        let quot_effect = match &quot_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("keep")
                    .ok_or_else(|| "Unknown word: 'keep'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "keep", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}keep: expected quotation or closure on top of stack, got {}",
                    line_prefix, quot_type
                ));
            }
        };

        if quot_effect.has_yield() {
            return Err("keep: quotation must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // Peek at the preserved value type (it stays, we just need its type)
        let (_rest_stack, preserved_type) = stack_after_quot.clone().pop().ok_or_else(|| {
            format!(
                "{}keep: stack underflow - expected a value below the quotation",
                line_prefix
            )
        })?;

        // The quotation receives x on the stack (stack_after_quot still has x on top).
        // Apply the quotation's effect to the stack INCLUDING x.
        let fresh_effect = self.freshen_effect(&quot_effect);
        let (result_stack, subst) =
            self.apply_effect(&fresh_effect, stack_after_quot, "keep (quotation)", span)?;

        // Push the preserved value back on top, applying substitution in case
        // preserved_type contains type variables resolved during unification
        let resolved_preserved = subst.apply_type(&preserved_type);
        let result_stack = result_stack.push(resolved_preserved);

        let propagated_effects = fresh_effect.effects.clone();
        Ok((result_stack, subst, propagated_effects))
    }

    /// Infer the stack effect of `bi`: ( ..a x quot1 quot2 -- ..c )
    ///
    /// Apply two quotations to the same value. First quotation receives x,
    /// then second quotation receives x on top of the first quotation's results.
    pub(super) fn infer_bi(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop quot2 (top)
        let (stack1, quot2_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected second quotation on stack",
                line_prefix
            )
        })?;

        // Pop quot1
        let (stack2, quot1_type) = stack1.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected first quotation on stack",
                line_prefix
            )
        })?;

        // Extract both quotation effects
        let quot1_effect = match &quot1_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("bi")
                    .ok_or_else(|| "Unknown word: 'bi'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "bi", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}bi: expected quotation or closure as first quotation, got {}",
                    line_prefix, quot1_type
                ));
            }
        };

        let quot2_effect = match &quot2_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("bi")
                    .ok_or_else(|| "Unknown word: 'bi'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "bi", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}bi: expected quotation or closure as second quotation, got {}",
                    line_prefix, quot2_type
                ));
            }
        };

        if quot1_effect.has_yield() || quot2_effect.has_yield() {
            return Err("bi: quotations must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // stack2 has x on top (the value both quotations operate on)
        // Peek at x's type for the second application
        let (_rest, preserved_type) = stack2.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected a value below the quotations",
                line_prefix
            )
        })?;

        // Apply quot1 to stack including x
        let fresh_effect1 = self.freshen_effect(&quot1_effect);
        let (after_quot1, subst1) =
            self.apply_effect(&fresh_effect1, stack2, "bi (first quotation)", span)?;

        // Push x again for quot2, applying subst1 in case preserved_type
        // contains type variables that were resolved during quot1's unification
        let resolved_preserved = subst1.apply_type(&preserved_type);
        let with_x = after_quot1.push(resolved_preserved);

        // Apply quot2
        let fresh_effect2 = self.freshen_effect(&quot2_effect);
        let (result_stack, subst2) =
            self.apply_effect(&fresh_effect2, with_x, "bi (second quotation)", span)?;

        let subst = subst1.compose(&subst2);

        let mut effects = fresh_effect1.effects.clone();
        for e in fresh_effect2.effects.clone() {
            if !effects.contains(&e) {
                effects.push(e);
            }
        }

        Ok((result_stack, subst, effects))
    }
}
