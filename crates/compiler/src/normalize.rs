//! AST normalization passes that run after parse + include resolution
//! and before type-checking.
//!
//! ## `if` literal-quotation lowering
//!
//! `if` is a stack-consuming conditional combinator. At the source level
//! a typical use is:
//!
//! ```seq
//! cond [ then-body ] [ else-body ] if
//! ```
//!
//! When both operands are literal `[ ... ]` quotations at the call site,
//! this triple is semantically identical to a parser-level if/else
//! conditional. Rewriting it to `Statement::If` here — *before* the
//! type-checker runs — means every downstream pass (typechecker,
//! codegen, lints, the type-specializer) sees the same shape it sees
//! for any other inline conditional.
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
//! ## Pipeline invariant
//!
//! Every code path that runs the type-checker on user source must
//! call [`lower_literal_if_combinators`] first. The dynamic-dispatch
//! path through `infer_if_combinator` is strictly less expressive than
//! `Statement::If` (no aux-stack consistency check, no divergent-branch
//! short-circuit), so a lint or analysis pass that skips this pre-pass
//! will reject programs the build accepts. Today the call sites are
//! `crates/compiler/src/lib.rs` (both `compile_*` paths) and
//! `crates/compiler/src/main/lint.rs`. Add new pipelines to that list.

use crate::ast::{Program, Statement};

/// Rewrite every `[Quotation, Quotation, WordCall("if")]` triple in
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
                _ => panic!("normalize: is_inline_triple guard accepted a non-Quotation"),
            };
            let mut else_body = match &mut else_quot {
                Statement::Quotation { body, .. } => std::mem::take(body),
                _ => panic!("normalize: is_inline_triple guard accepted a non-Quotation"),
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
        ) if name == "if"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Program, WordDef};

    fn quot(id: usize, body: Vec<Statement>) -> Statement {
        Statement::Quotation {
            id,
            body,
            span: None,
        }
    }

    fn word_call(name: &str) -> Statement {
        Statement::WordCall {
            name: name.to_string(),
            span: None,
        }
    }

    fn lower_body(body: Vec<Statement>) -> Vec<Statement> {
        let mut program = Program {
            includes: vec![],
            unions: vec![],
            words: vec![WordDef {
                name: "test".to_string(),
                effect: None,
                body,
                source: None,
                allowed_lints: vec![],
            }],
        };
        lower_literal_if_combinators(&mut program);
        program.words.into_iter().next().unwrap().body
    }

    #[test]
    fn rewrites_literal_triple_to_statement_if() {
        let body = vec![
            Statement::BoolLiteral(true),
            quot(0, vec![Statement::IntLiteral(1)]),
            quot(1, vec![Statement::IntLiteral(2)]),
            word_call("if"),
        ];
        let lowered = lower_body(body);
        assert_eq!(lowered.len(), 2);
        assert!(matches!(lowered[0], Statement::BoolLiteral(true)));
        match &lowered[1] {
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                assert_eq!(then_branch, &vec![Statement::IntLiteral(1)]);
                assert_eq!(
                    else_branch.as_deref(),
                    Some(&[Statement::IntLiteral(2)][..])
                );
            }
            other => panic!("expected Statement::If, got {:?}", other),
        }
    }

    #[test]
    fn rewrites_nested_triples() {
        // outer: cond [ inner ] [ ... ] if
        // inner: cond' [ a ] [ b ] if
        let inner_triple = vec![
            Statement::BoolLiteral(true),
            quot(2, vec![Statement::IntLiteral(10)]),
            quot(3, vec![Statement::IntLiteral(20)]),
            word_call("if"),
        ];
        let body = vec![
            Statement::BoolLiteral(false),
            quot(0, inner_triple),
            quot(1, vec![Statement::IntLiteral(99)]),
            word_call("if"),
        ];
        let lowered = lower_body(body);
        assert_eq!(lowered.len(), 2);
        match &lowered[1] {
            Statement::If { then_branch, .. } => {
                assert_eq!(then_branch.len(), 2);
                assert!(matches!(then_branch[0], Statement::BoolLiteral(true)));
                assert!(matches!(then_branch[1], Statement::If { .. }));
            }
            other => panic!("expected outer Statement::If, got {:?}", other),
        }
    }

    #[test]
    fn leaves_dynamic_dispatch_alone() {
        // [ a ] my-word if   — second operand is a WordCall, not a Quotation literal.
        let body = vec![
            Statement::BoolLiteral(true),
            quot(0, vec![Statement::IntLiteral(1)]),
            word_call("my-word"),
            word_call("if"),
        ];
        let original = body.clone();
        let lowered = lower_body(body);
        assert_eq!(lowered, original);
    }

    #[test]
    fn leaves_non_if_word_call_alone() {
        // [ a ] [ b ] times   — a real (different) combinator.
        let body = vec![
            Statement::IntLiteral(3),
            quot(0, vec![Statement::IntLiteral(1)]),
            quot(1, vec![Statement::IntLiteral(2)]),
            word_call("times"),
        ];
        let original = body.clone();
        let lowered = lower_body(body);
        assert_eq!(lowered, original);
    }

    #[test]
    fn recurses_into_quotation_body() {
        // [ cond [ a ] [ b ] if ] my-word — the outer is left alone,
        // but the inner triple inside the quotation body should be rewritten.
        let inner_triple = vec![
            Statement::BoolLiteral(true),
            quot(1, vec![Statement::IntLiteral(1)]),
            quot(2, vec![Statement::IntLiteral(2)]),
            word_call("if"),
        ];
        let body = vec![quot(0, inner_triple), word_call("my-word")];
        let lowered = lower_body(body);
        match &lowered[0] {
            Statement::Quotation { body, .. } => {
                assert_eq!(body.len(), 2);
                assert!(matches!(body[1], Statement::If { .. }));
            }
            other => panic!("expected outer Quotation, got {:?}", other),
        }
    }
}
