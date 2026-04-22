//! Quotations, dataflow combinators, and control-flow builtins.

use std::collections::HashMap;

use crate::types::{Effect, SideEffect, StackType, Type};

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

    // strand.spawn: ( a Quotation -- a Int ) - spawn a concurrent strand
    // The quotation can have any stack effect - it runs independently
    sigs.insert(
        "strand.spawn".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Quotation(Box::new(Effect::new(
                StackType::RowVar("spawn_in".to_string()),
                StackType::RowVar("spawn_out".to_string()),
            )))),
            StackType::RowVar("a".to_string()).push(Type::Int),
        ),
    );

    // strand.weave: ( a Quotation -- a handle ) - create a woven strand (generator)
    // The quotation receives (WeaveCtx, first_resume_value) and must thread WeaveCtx through.
    // Returns a handle (WeaveCtx) for use with strand.resume.
    sigs.insert(
        "strand.weave".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Quotation(Box::new(Effect::new(
                StackType::RowVar("weave_in".to_string()),
                StackType::RowVar("weave_out".to_string()),
            )))),
            StackType::RowVar("a".to_string()).push(Type::Var("handle".to_string())),
        ),
    );

    // strand.resume: ( a handle b -- a handle b Bool ) - resume weave with value
    // Takes handle and value to send, returns (handle, yielded_value, has_more)
    sigs.insert(
        "strand.resume".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("handle".to_string()))
                .push(Type::Var("b".to_string())),
            StackType::RowVar("a".to_string())
                .push(Type::Var("handle".to_string()))
                .push(Type::Var("b".to_string()))
                .push(Type::Bool),
        ),
    );

    // yield: ( a ctx b -- a ctx b | Yield b ) - yield value and receive resume value
    // The WeaveCtx must be passed explicitly and threaded through.
    // The Yield effect indicates this word produces yield semantics.
    sigs.insert(
        "yield".to_string(),
        Effect::with_effects(
            StackType::RowVar("a".to_string())
                .push(Type::Var("ctx".to_string()))
                .push(Type::Var("b".to_string())),
            StackType::RowVar("a".to_string())
                .push(Type::Var("ctx".to_string()))
                .push(Type::Var("b".to_string())),
            vec![SideEffect::Yield(Box::new(Type::Var("b".to_string())))],
        ),
    );

    // strand.weave-cancel: ( a handle -- a ) - cancel a weave and release its resources
    // Use this to clean up a weave that won't be resumed to completion.
    // This prevents resource leaks from abandoned weaves.
    sigs.insert(
        "strand.weave-cancel".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Var("handle".to_string())),
            StackType::RowVar("a".to_string()),
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
