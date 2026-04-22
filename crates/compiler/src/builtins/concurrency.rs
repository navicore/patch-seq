//! Channels and strand-related concurrency primitives.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

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
    builtin!(sigs, "chan.yield", (a - -a));
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
