//! Runtime declarations for the test framework and time operations.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Test framework operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_init(ptr)",
        category: Some("; Test framework operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_finish(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_has_failures(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_assert(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_assert_not(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_assert_eq(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_assert_eq_str(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_fail(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_pass_count(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_test_fail_count(ptr)",
        category: None,
    },
    // Source-line diagnostic hook: direct FFI, no stack thread.
    RuntimeDecl {
        decl: "declare void @patch_seq_test_set_line(i64)",
        category: None,
    },
    // Time operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_time_now(ptr)",
        category: Some("; Time operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_time_nanos(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_time_sleep_ms(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // Test framework operations
    ("test.init", "patch_seq_test_init"),
    ("test.finish", "patch_seq_test_finish"),
    ("test.has-failures", "patch_seq_test_has_failures"),
    ("test.assert", "patch_seq_test_assert"),
    ("test.assert-not", "patch_seq_test_assert_not"),
    ("test.assert-eq", "patch_seq_test_assert_eq"),
    ("test.assert-eq-str", "patch_seq_test_assert_eq_str"),
    ("test.fail", "patch_seq_test_fail"),
    ("test.pass-count", "patch_seq_test_pass_count"),
    ("test.fail-count", "patch_seq_test_fail_count"),
    // Time operations
    ("time.now", "patch_seq_time_now"),
    ("time.nanos", "patch_seq_time_nanos"),
    ("time.sleep-ms", "patch_seq_time_sleep_ms"),
];
