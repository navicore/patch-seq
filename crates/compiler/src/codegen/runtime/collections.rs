//! Runtime declarations for list and map operations.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // List operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_make(ptr)",
        category: Some("; List operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_push(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_get(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_set(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_map(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_filter(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_fold(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_each(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_length(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_empty(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_list_reverse(ptr)",
        category: None,
    },
    // Map operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_map(ptr)",
        category: Some("; Map operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_get(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_set(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_has(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_remove(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_keys(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_values(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_size(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_empty(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_each(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_map_fold(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // List operations
    ("list.make", "patch_seq_list_make"),
    ("list.push", "patch_seq_list_push"),
    ("list.get", "patch_seq_list_get"),
    ("list.set", "patch_seq_list_set"),
    ("list.map", "patch_seq_list_map"),
    ("list.filter", "patch_seq_list_filter"),
    ("list.fold", "patch_seq_list_fold"),
    ("list.each", "patch_seq_list_each"),
    ("list.length", "patch_seq_list_length"),
    ("list.empty?", "patch_seq_list_empty"),
    ("list.reverse", "patch_seq_list_reverse"),
    // Map operations
    ("map.make", "patch_seq_make_map"),
    ("map.get", "patch_seq_map_get"),
    ("map.set", "patch_seq_map_set"),
    ("map.has?", "patch_seq_map_has"),
    ("map.remove", "patch_seq_map_remove"),
    ("map.keys", "patch_seq_map_keys"),
    ("map.values", "patch_seq_map_values"),
    ("map.size", "patch_seq_map_size"),
    ("map.empty?", "patch_seq_map_empty"),
    ("map.each", "patch_seq_map_each"),
    ("map.fold", "patch_seq_map_fold"),
];
