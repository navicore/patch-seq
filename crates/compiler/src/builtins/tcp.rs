//! TCP socket operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // TCP Operations
    // =========================================================================

    // TCP operations return Bool for error handling
    builtin!(sigs, "tcp.listen", (a Int -- a Int Bool));
    builtin!(sigs, "tcp.accept", (a Int -- a Int Bool));
    builtin!(sigs, "tcp.read", (a Int -- a String Bool));
    builtin!(sigs, "tcp.write", (a String Int -- a Bool));
    builtin!(sigs, "tcp.close", (a Int -- a Bool));
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // TCP Operations
    docs.insert(
        "tcp.listen",
        "Start listening on a port. Returns (socket_id, success).",
    );
    docs.insert(
        "tcp.accept",
        "Accept a connection. Returns (client_id, success).",
    );
    docs.insert(
        "tcp.read",
        "Read data from a socket. Returns (string, success).",
    );
    docs.insert("tcp.write", "Write data to a socket. Returns success.");
    docs.insert("tcp.close", "Close a socket. Returns success.");

    // TCP Operations
    docs.insert(
        "tcp.listen",
        "Start listening on a port. Returns (fd Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.accept",
        "Accept a connection. Returns (fd Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.read",
        "Read from a connection. Returns (String Bool) -- Bool is false on failure.",
    );
    docs.insert(
        "tcp.write",
        "Write to a connection. Returns Bool -- false on failure.",
    );
    docs.insert(
        "tcp.close",
        "Close a connection. Returns Bool -- false on failure.",
    );
}
