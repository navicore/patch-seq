//! Test framework, time, SON serialization, and stack introspection.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
    // =========================================================================
    // Test Framework
    // =========================================================================

    builtin!(sigs, "test.init", (a String -- a));
    // Identity effect ( a -- a ). See note in concurrency.rs on chan.yield.
    sigs.insert(
        "test.finish".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()),
            StackType::RowVar("a".to_string()),
        ),
    );
    builtin!(sigs, "test.has-failures", (a -- a Bool));
    builtin!(sigs, "test.assert", (a Bool -- a));
    builtin!(sigs, "test.assert-not", (a Bool -- a));
    builtin!(sigs, "test.assert-eq", (a Int Int -- a));
    builtin!(sigs, "test.assert-eq-str", (a String String -- a));
    builtin!(sigs, "test.fail", (a String -- a));
    builtin!(sigs, "test.pass-count", (a -- a Int));
    builtin!(sigs, "test.fail-count", (a -- a Int));

    // Time operations
    builtin!(sigs, "time.now", (a -- a Int));
    builtin!(sigs, "time.nanos", (a -- a Int));
    builtin!(sigs, "time.sleep-ms", (a Int -- a));

    // SON serialization
    builtin!(sigs, "son.dump", (a T -- a String));
    builtin!(sigs, "son.dump-pretty", (a T -- a String));

    // Stack introspection (for REPL)
    // stack.dump prints all values and clears the stack
    sigs.insert(
        "stack.dump".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()), // Consumes any stack
            StackType::RowVar("b".to_string()), // Returns empty stack (different row var)
        ),
    );
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // Test Framework
    docs.insert(
        "test.init",
        "Initialize the test framework with a test name.",
    );
    docs.insert("test.finish", "Finish testing and print results.");
    docs.insert("test.has-failures", "Check if any tests have failed.");
    docs.insert("test.assert", "Assert that a boolean is true.");
    docs.insert("test.assert-not", "Assert that a boolean is false.");
    docs.insert("test.assert-eq", "Assert that two integers are equal.");
    docs.insert("test.assert-eq-str", "Assert that two strings are equal.");
    docs.insert("test.fail", "Mark a test as failed with a message.");
    docs.insert("test.pass-count", "Get the number of passed assertions.");
    docs.insert("test.fail-count", "Get the number of failed assertions.");

    // Time Operations
    docs.insert("time.now", "Get current Unix timestamp in seconds.");
    docs.insert(
        "time.nanos",
        "Get high-resolution monotonic time in nanoseconds.",
    );
    docs.insert("time.sleep-ms", "Sleep for N milliseconds.");

    // Serialization
    docs.insert("son.dump", "Serialize any value to SON format (compact).");
    docs.insert(
        "son.dump-pretty",
        "Serialize any value to SON format (pretty-printed).",
    );

    // Stack Introspection
    docs.insert(
        "stack.dump",
        "Print all stack values and clear the stack (REPL).",
    );
}
