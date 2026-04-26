//! UDP socket operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // UDP Operations
    // =========================================================================
    //
    // Datagram-oriented; sockets are Int handles. Every word ends with a
    // success Bool on top so callers can `[ ... ] [ ... ] if`.
    //
    // `udp.bind` returns three values: (socket, bound-port, success).
    // The bound-port differs from the requested port only when the user
    // passed 0 (let the OS pick); for non-zero requests the returned
    // port equals the request.
    builtin!(sigs, "udp.bind", (a Int -- a Int Int Bool));
    builtin!(sigs, "udp.send-to", (a String String Int Int -- a Bool));
    builtin!(sigs, "udp.receive-from", (a Int -- a String String Int Bool));
    builtin!(sigs, "udp.close", (a Int -- a Bool));
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    docs.insert(
        "udp.bind",
        "Bind a UDP socket to a local port. ( port -- socket bound-port Bool ). \
         port=0 lets the OS pick; bound-port is the actual assigned port. \
         On failure pushes (0, 0, false).",
    );
    docs.insert(
        "udp.send-to",
        "Send a datagram to host:port from a bound socket. \
         ( bytes host port socket -- Bool ).",
    );
    docs.insert(
        "udp.receive-from",
        "Receive one datagram (yields the strand). \
         ( socket -- bytes host port Bool ). \
         On failure pushes (\"\", \"\", 0, false).",
    );
    docs.insert("udp.close", "Release a UDP socket. ( socket -- Bool ).");
}
