//! AST normalization passes that run after type-checking and before
//! codegen / specialization.
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
//! When both branches are literal quotations with no captures and no
//! `>aux`/`aux>` operations, this triple is semantically identical to
//! the parser-level `cond if then-body else else-body then` form.
//! Rewriting the triple to a `Statement::If` here lets every downstream
//! pass — codegen, the type-specializer, lints — see the same shape it
//! sees today, which is the perf gate the design doc calls out.
//!
//! When either quotation has captures (it's a `Type::Closure`) or its
//! body uses `>aux`/`aux>` (the per-quotation aux frame is allocated by
//! the wrapper, which inlining skips), the triple is left alone and the
//! runtime `patch_seq_if` dispatch handles it.
//!
//! Removed once `if/else/then` keywords go away (phase 3) — at that
//! point this rewrite *is* how `if` becomes a parser-emitted control
//! structure.

use crate::ast::{Program, Statement};
use crate::types::Type;
use std::collections::HashMap;

/// Rewrite `[Quotation, Quotation, WordCall("__if__")]` triples to
/// `Statement::If` where it's safe (both quotations have no captures
/// and use no aux operations). All other `__if__` calls are left
/// untouched and go through the runtime dispatch path.
pub fn lower_literal_if_combinators(
    program: &mut Program,
    quotation_types: &HashMap<usize, Type>,
    quotation_aux_depths: &HashMap<usize, usize>,
) {
    for word in &mut program.words {
        rewrite_statements(&mut word.body, quotation_types, quotation_aux_depths);
    }
}

fn rewrite_statements(
    statements: &mut Vec<Statement>,
    quotation_types: &HashMap<usize, Type>,
    quotation_aux_depths: &HashMap<usize, usize>,
) {
    let mut i = 0;
    while i < statements.len() {
        if i + 2 < statements.len()
            && let Some((then_id, else_id)) = match_inline_triple(&statements[i..i + 3])
            && quotation_inlineable(then_id, quotation_types, quotation_aux_depths)
            && quotation_inlineable(else_id, quotation_types, quotation_aux_depths)
        {
            let if_span = match &statements[i + 2] {
                Statement::WordCall { span, .. } => span.clone(),
                _ => None,
            };
            statements.remove(i + 2);
            let mut else_quot = statements.remove(i + 1);
            let mut then_quot = statements.remove(i);

            let mut then_body = match &mut then_quot {
                Statement::Quotation { body, .. } => std::mem::take(body),
                _ => unreachable!("guarded by match_inline_triple"),
            };
            let mut else_body = match &mut else_quot {
                Statement::Quotation { body, .. } => std::mem::take(body),
                _ => unreachable!("guarded by match_inline_triple"),
            };
            rewrite_statements(&mut then_body, quotation_types, quotation_aux_depths);
            rewrite_statements(&mut else_body, quotation_types, quotation_aux_depths);

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
                rewrite_statements(then_branch, quotation_types, quotation_aux_depths);
                if let Some(eb) = else_branch.as_mut() {
                    rewrite_statements(eb, quotation_types, quotation_aux_depths);
                }
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    rewrite_statements(&mut arm.body, quotation_types, quotation_aux_depths);
                }
            }
            Statement::Quotation { body, .. } => {
                rewrite_statements(body, quotation_types, quotation_aux_depths);
            }
            _ => {}
        }
        i += 1;
    }
}

fn match_inline_triple(triple: &[Statement]) -> Option<(usize, usize)> {
    let (
        Statement::Quotation { id: then_id, .. },
        Statement::Quotation { id: else_id, .. },
        Statement::WordCall { name, .. },
    ) = (&triple[0], &triple[1], &triple[2])
    else {
        return None;
    };
    if name != "__if__" {
        return None;
    }
    Some((*then_id, *else_id))
}

fn quotation_inlineable(
    id: usize,
    quotation_types: &HashMap<usize, Type>,
    quotation_aux_depths: &HashMap<usize, usize>,
) -> bool {
    match quotation_types.get(&id) {
        Some(Type::Quotation(_)) => {}
        _ => return false,
    }
    matches!(quotation_aux_depths.get(&id).copied(), None | Some(0))
}
