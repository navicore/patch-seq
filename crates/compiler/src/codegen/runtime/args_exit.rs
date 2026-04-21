//! Runtime declarations for process exit code and command-line arguments.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Exit code handling
    RuntimeDecl {
        decl: "declare void @patch_seq_set_exit_code(i64)",
        category: Some("; Exit code handling"),
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_get_exit_code()",
        category: None,
    },
    // Command-line argument operations
    RuntimeDecl {
        decl: "declare void @patch_seq_args_init(i32, ptr)",
        category: Some("; Command-line argument operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_arg_count(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_arg_at(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    ("args.count", "patch_seq_arg_count"),
    ("args.at", "patch_seq_arg_at"),
];
