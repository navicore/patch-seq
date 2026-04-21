//! Runtime declarations for closure creation and captured-environment access.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    RuntimeDecl {
        decl: "declare ptr @patch_seq_create_env(i32)",
        category: Some("; Closure operations"),
    },
    RuntimeDecl {
        decl: "declare void @patch_seq_env_set(ptr, i32, %Value)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare %Value @patch_seq_env_get(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_env_get_int(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_env_get_bool(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare double @patch_seq_env_get_float(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_env_get_quotation(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_env_get_string(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_env_push_string(ptr, ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_env_push_value(ptr, ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare %Value @patch_seq_make_closure(i64, ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_push_closure(ptr, i64, i32)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_push_seqstring(ptr, ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[];
