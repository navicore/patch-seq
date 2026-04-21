//! Runtime declarations for assorted utilities that come after the main
//! categories: stack introspection, SON serialization, regex, compression,
//! peek helpers used by conditionals, raw tagged-stack access, and the
//! at-exit report hook.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Stack introspection
    RuntimeDecl {
        decl: "declare ptr @patch_seq_stack_dump(ptr)",
        category: Some("; Stack introspection"),
    },
    // SON serialization
    RuntimeDecl {
        decl: "declare ptr @patch_seq_son_dump(ptr)",
        category: Some("; SON serialization"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_son_dump_pretty(ptr)",
        category: None,
    },
    // Regex operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_match(ptr)",
        category: Some("; Regex operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_find(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_find_all(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_replace(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_replace_all(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_captures(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_split(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_regex_valid(ptr)",
        category: None,
    },
    // Compression operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_gzip(ptr)",
        category: Some("; Compression operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_gzip_level(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_gunzip(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_zstd(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_zstd_level(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_compress_unzstd(ptr)",
        category: None,
    },
    // Helpers for conditionals
    RuntimeDecl {
        decl: "declare i64 @patch_seq_peek_int_value(ptr)",
        category: Some("; Helpers for conditionals"),
    },
    RuntimeDecl {
        decl: "declare i1 @patch_seq_peek_bool_value(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_pop_stack(ptr)",
        category: None,
    },
    // Tagged stack operations
    RuntimeDecl {
        decl: "declare ptr @seq_stack_new_default()",
        category: Some("; Tagged stack operations"),
    },
    RuntimeDecl {
        decl: "declare void @seq_stack_free(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @seq_stack_base(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @seq_stack_sp(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare void @seq_stack_set_sp(ptr, i64)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare void @seq_stack_grow(ptr, i64)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare void @patch_seq_set_stack_base(ptr)",
        category: None,
    },
    // Report operations
    RuntimeDecl {
        decl: "declare void @patch_seq_report()",
        category: Some("; Report operations"),
    },
    RuntimeDecl {
        decl: "declare void @patch_seq_report_init(ptr, ptr, i64)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // Regex operations
    ("regex.match?", "patch_seq_regex_match"),
    ("regex.find", "patch_seq_regex_find"),
    ("regex.find-all", "patch_seq_regex_find_all"),
    ("regex.replace", "patch_seq_regex_replace"),
    ("regex.replace-all", "patch_seq_regex_replace_all"),
    ("regex.captures", "patch_seq_regex_captures"),
    ("regex.split", "patch_seq_regex_split"),
    ("regex.valid?", "patch_seq_regex_valid"),
    // Compression operations
    ("compress.gzip", "patch_seq_compress_gzip"),
    ("compress.gzip-level", "patch_seq_compress_gzip_level"),
    ("compress.gunzip", "patch_seq_compress_gunzip"),
    ("compress.zstd", "patch_seq_compress_zstd"),
    ("compress.zstd-level", "patch_seq_compress_zstd_level"),
    ("compress.unzstd", "patch_seq_compress_unzstd"),
    // SON serialization
    ("son.dump", "patch_seq_son_dump"),
    ("son.dump-pretty", "patch_seq_son_dump_pretty"),
    // Stack introspection
    ("stack.dump", "patch_seq_stack_dump"),
];
