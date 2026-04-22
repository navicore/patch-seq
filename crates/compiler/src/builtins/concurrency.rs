//! Channels and strand-related concurrency primitives.

use std::collections::HashMap;

use crate::types::{Effect, SideEffect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // Channel Operations (CSP-style concurrency)
    // Errors are values, not crashes - all ops return success flags
    // =========================================================================

    builtin!(sigs, "chan.make", (a -- a Channel));
    builtin!(sigs, "chan.send", (a T Channel -- a Bool)); // returns success flag
    builtin!(sigs, "chan.receive", (a Channel -- a T Bool)); // returns value and success flag
    builtin!(sigs, "chan.close", (a Channel -- a));
    // Identity effect ( a -- a ). Spelled out because the `builtin!` macro's
    // (a -- a) arm round-trips through rustfmt as (a - -a), which is
    // surprising to read.
    sigs.insert(
        "chan.yield".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()),
            StackType::RowVar("a".to_string()),
        ),
    );

    // =========================================================================
    // Strand / Weave (co-located with their docs)
    // =========================================================================

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
    // Channel Operations
    docs.insert(
        "chan.make",
        "Create a new channel for inter-strand communication.",
    );
    docs.insert(
        "chan.send",
        "Send a value on a channel. Returns Bool -- false if channel is closed.",
    );
    docs.insert(
        "chan.receive",
        "Receive a value from a channel. Returns (value Bool) -- Bool is false if channel is closed.",
    );
    docs.insert("chan.close", "Close a channel.");
    docs.insert("chan.yield", "Yield control to the scheduler.");

    // Concurrency
    docs.insert(
        "strand.spawn",
        "Spawn a concurrent strand. Returns strand ID.",
    );
    docs.insert(
        "strand.weave",
        "Create a generator/coroutine. Returns handle.",
    );
    docs.insert(
        "strand.resume",
        "Resume a weave with a value. Returns (handle, value, has_more).",
    );
    docs.insert(
        "yield",
        "Yield a value from a weave and receive resume value.",
    );
    docs.insert(
        "strand.weave-cancel",
        "Cancel a weave and release its resources.",
    );
}
