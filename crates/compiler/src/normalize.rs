//! AST normalization passes that run after parse + include resolution
//! and before type-checking.
//!
//! ## `__if__` literal-quotation lowering
//!
//! The 6.0 migration introduces `__if__` as the conditional combinator
//! (renamed to `if` once `if/else/then` are removed from the parser).
//! At the source level a typical use is:
//!
//! ```seq
//! cond [ then-body ] [ else-body ] __if__
//! ```
//!
//! When both operands are literal `[ ... ]` quotations at the call site,
//! this triple is semantically identical to the (now-removed)
//! parser-level `cond if then-body else else-body then` form. Rewriting
//! it to `Statement::If` here — *before* the type-checker runs — means
//! every downstream pass (typechecker, codegen, lints, the
//! type-specializer) sees the same shape it sees for the keyword form.
//!
//! Doing this purely syntactically (no consultation of inferred types)
//! is sound: lifting a literal quotation's body into the enclosing word
//! is semantically equivalent to invoking the quotation, because
//! captures resolve in the same lexical scope and `>aux`/`aux>` slot
//! allocation falls naturally to the enclosing word's slot table once
//! the body is no longer wrapped in its own quotation function.
//!
//! When at least one operand is *not* a literal `[ ... ]` (e.g. it
//! comes from a word argument or is constructed at runtime), the
//! triple is left alone and the runtime `patch_seq_if` dispatch
//! handles the call.
//!
//! Removed once the `__if__` → `if` rename completes (final phase of
//! the 6.0 cutover).

use crate::ast::{Program, Statement};

/// Rewrite every `[Quotation, Quotation, WordCall("__if__")]` triple in
/// the program to a `Statement::If`. Runs after parse + include
/// resolution, before type-checking.
pub fn lower_literal_if_combinators(program: &mut Program) {
    for word in &mut program.words {
        rewrite_statements(&mut word.body);
    }
}

fn rewrite_statements(statements: &mut Vec<Statement>) {
    let mut i = 0;
    while i < statements.len() {
        if i + 2 < statements.len() && is_inline_triple(&statements[i..i + 3]) {
            let if_span = match &statements[i + 2] {
                Statement::WordCall { span, .. } => span.clone(),
                _ => None,
            };
            statements.remove(i + 2);
            let mut else_quot = statements.remove(i + 1);
            let mut then_quot = statements.remove(i);

            let mut then_body = match &mut then_quot {
                Statement::Quotation { body, .. } => std::mem::take(body),
                _ => unreachable!("guarded by is_inline_triple"),
            };
            let mut else_body = match &mut else_quot {
                Statement::Quotation { body, .. } => std::mem::take(body),
                _ => unreachable!("guarded by is_inline_triple"),
            };
            rewrite_statements(&mut then_body);
            rewrite_statements(&mut else_body);

            statements.insert(
                i,
                Statement::If {
                    then_branch: then_body,
                    else_branch: Some(else_body),
                    span: if_span,
                },
            );
            i += 1;
            continue;
        }

        match &mut statements[i] {
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                rewrite_statements(then_branch);
                if let Some(eb) = else_branch.as_mut() {
                    rewrite_statements(eb);
                }
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    rewrite_statements(&mut arm.body);
                }
            }
            Statement::Quotation { body, .. } => {
                rewrite_statements(body);
            }
            _ => {}
        }
        i += 1;
    }
}

fn is_inline_triple(triple: &[Statement]) -> bool {
    matches!(
        (&triple[0], &triple[1], &triple[2]),
        (
            Statement::Quotation { .. },
            Statement::Quotation { .. },
            Statement::WordCall { name, .. },
        ) if name == "__if__"
    )
}
