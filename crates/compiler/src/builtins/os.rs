//! OS primitives, signal handling, and terminal operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // OS Operations
    // =========================================================================

    builtin!(sigs, "os.getenv", (a String -- a String Bool));
    builtin!(sigs, "os.home-dir", (a -- a String Bool));
    builtin!(sigs, "os.current-dir", (a -- a String Bool));
    builtin!(sigs, "os.path-exists", (a String -- a Bool));
    builtin!(sigs, "os.path-is-file", (a String -- a Bool));
    builtin!(sigs, "os.path-is-dir", (a String -- a Bool));
    builtin!(sigs, "os.path-join", (a String String -- a String));
    builtin!(sigs, "os.path-parent", (a String -- a String Bool));
    builtin!(sigs, "os.path-filename", (a String -- a String Bool));
    builtin!(sigs, "os.exit", (a Int -- a)); // Never returns, but typed as identity
    builtin!(sigs, "os.name", (a -- a String));
    builtin!(sigs, "os.arch", (a -- a String));

    // =========================================================================
    // Signal Handling (Unix signals)
    // =========================================================================

    builtin!(sigs, "signal.trap", (a Int -- a));
    builtin!(sigs, "signal.received?", (a Int -- a Bool));
    builtin!(sigs, "signal.pending?", (a Int -- a Bool));
    builtin!(sigs, "signal.default", (a Int -- a));
    builtin!(sigs, "signal.ignore", (a Int -- a));
    builtin!(sigs, "signal.clear", (a Int -- a));
    // Signal constants (platform-correct values)
    builtin!(sigs, "signal.SIGINT", (a -- a Int));
    builtin!(sigs, "signal.SIGTERM", (a -- a Int));
    builtin!(sigs, "signal.SIGHUP", (a -- a Int));
    builtin!(sigs, "signal.SIGPIPE", (a -- a Int));
    builtin!(sigs, "signal.SIGUSR1", (a -- a Int));
    builtin!(sigs, "signal.SIGUSR2", (a -- a Int));
    builtin!(sigs, "signal.SIGCHLD", (a -- a Int));
    builtin!(sigs, "signal.SIGALRM", (a -- a Int));
    builtin!(sigs, "signal.SIGCONT", (a -- a Int));

    // =========================================================================
    // Terminal Operations (raw mode, character I/O, dimensions)
    // =========================================================================

    builtin!(sigs, "terminal.raw-mode", (a Bool -- a));
    builtin!(sigs, "terminal.read-char", (a -- a Int));
    builtin!(sigs, "terminal.read-char?", (a -- a Int));
    builtin!(sigs, "terminal.width", (a -- a Int));
    builtin!(sigs, "terminal.height", (a -- a Int));
    builtin!(sigs, "terminal.flush", (a - -a));
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // OS Operations
    docs.insert(
        "os.getenv",
        "Get environment variable. Returns (String Bool) -- Bool is false if not set.",
    );
    docs.insert(
        "os.home-dir",
        "Get home directory. Returns (String Bool) -- Bool is false if unavailable.",
    );
    docs.insert(
        "os.current-dir",
        "Get current directory. Returns (String Bool) -- Bool is false if unavailable.",
    );
    docs.insert("os.path-exists", "Check if a path exists.");
    docs.insert("os.path-is-file", "Check if a path is a file.");
    docs.insert("os.path-is-dir", "Check if a path is a directory.");
    docs.insert("os.path-join", "Join two path segments.");
    docs.insert(
        "os.path-parent",
        "Get parent directory. Returns (String Bool) -- Bool is false for root.",
    );
    docs.insert(
        "os.path-filename",
        "Get filename component. Returns (String Bool) -- Bool is false if none.",
    );
    docs.insert("os.exit", "Exit the process with given exit code.");
    docs.insert("os.name", "Get OS name (e.g., \"macos\", \"linux\").");
    docs.insert(
        "os.arch",
        "Get CPU architecture (e.g., \"aarch64\", \"x86_64\").",
    );

    // Signal Handling
    docs.insert(
        "signal.trap",
        "Trap a signal: set internal flag on receipt instead of default action.",
    );
    docs.insert(
        "signal.received?",
        "Check if signal was received and clear the flag. Returns Bool.",
    );
    docs.insert(
        "signal.pending?",
        "Check if signal is pending without clearing the flag. Returns Bool.",
    );
    docs.insert(
        "signal.default",
        "Restore the default handler for a signal.",
    );
    docs.insert(
        "signal.ignore",
        "Ignore a signal entirely (useful for SIGPIPE in servers).",
    );
    docs.insert(
        "signal.clear",
        "Clear the pending flag for a signal without checking it.",
    );
    docs.insert("signal.SIGINT", "SIGINT constant (Ctrl+C interrupt).");
    docs.insert("signal.SIGTERM", "SIGTERM constant (termination request).");
    docs.insert("signal.SIGHUP", "SIGHUP constant (hangup detected).");
    docs.insert("signal.SIGPIPE", "SIGPIPE constant (broken pipe).");
    docs.insert(
        "signal.SIGUSR1",
        "SIGUSR1 constant (user-defined signal 1).",
    );
    docs.insert(
        "signal.SIGUSR2",
        "SIGUSR2 constant (user-defined signal 2).",
    );
    docs.insert("signal.SIGCHLD", "SIGCHLD constant (child status changed).");
    docs.insert("signal.SIGALRM", "SIGALRM constant (alarm clock).");
    docs.insert("signal.SIGCONT", "SIGCONT constant (continue if stopped).");

    // Terminal Operations
    docs.insert(
        "terminal.raw-mode",
        "Enable/disable raw terminal mode. In raw mode: no line buffering, no echo, Ctrl+C read as byte 3.",
    );
    docs.insert(
        "terminal.read-char",
        "Read a single byte from stdin (blocking). Returns 0-255 on success, -1 on EOF/error.",
    );
    docs.insert(
        "terminal.read-char?",
        "Read a single byte from stdin (non-blocking). Returns 0-255 if available, -1 otherwise.",
    );
    docs.insert(
        "terminal.width",
        "Get terminal width in columns. Returns 80 if unknown.",
    );
    docs.insert(
        "terminal.height",
        "Get terminal height in rows. Returns 24 if unknown.",
    );
    docs.insert(
        "terminal.flush",
        "Flush stdout. Use after writing escape sequences or partial lines.",
    );
}
