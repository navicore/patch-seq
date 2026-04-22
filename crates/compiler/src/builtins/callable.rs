//! Quotations, dataflow combinators, and control-flow builtins.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // Quotation/Control Flow Operations
    // =========================================================================

    // call: Polymorphic - accepts Quotation or Closure
    // Uses type variable Q to represent "something callable"
    sigs.insert(
        "call".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()),
        ),
    );

    // =========================================================================
    // Dataflow Combinators
    // =========================================================================

    // dip: ( ..a x Quotation[..a -- ..b] -- ..b x )
    // Hide top value, run quotation on rest, restore value.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Same placeholder shape as keep — both take (value, quotation) and preserve value.
    sigs.insert(
        "dip".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
        ),
    );

    // keep: ( ..a x Quotation[..a x -- ..b] -- ..b x )
    // Run quotation on value, but preserve the original.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Same placeholder shape as dip — both take (value, quotation) and preserve value.
    sigs.insert(
        "keep".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
        ),
    );

    // bi: ( ..a x Quotation[..a x -- ..b] Quotation[..b x -- ..c] -- ..c )
    // Apply two quotations to the same value.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Q1/Q2 are distinct type vars — the two quotations may have different types.
    sigs.insert(
        "bi".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q1".to_string()))
                .push(Type::Var("Q2".to_string())),
            StackType::RowVar("b".to_string()),
        ),
    );

    // cond: Multi-way conditional (variable arity)
    sigs.insert(
        "cond".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()),
            StackType::RowVar("b".to_string()),
        ),
    );
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // Control Flow
    docs.insert("call", "Call a quotation or closure.");
    docs.insert(
        "cond",
        "Multi-way conditional: test clauses until one succeeds.",
    );

    // Dataflow Combinators
    docs.insert(
        "dip",
        "Hide top value, run quotation on rest of stack, restore value. ( ..a x [..a -- ..b] -- ..b x )",
    );
    docs.insert(
        "keep",
        "Run quotation on top value, but preserve the original. ( ..a x [..a x -- ..b] -- ..b x )",
    );
    docs.insert(
        "bi",
        "Apply two quotations to the same value. ( ..a x [q1] [q2] -- ..c )",
    );
}
