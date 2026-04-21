//! Runtime declarations for symbols and variant (algebraic-data-type)
//! construction and access, including the `wrap-N` aliases used by SON
//! serialization.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Symbol operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_symbol_equal(ptr)",
        category: Some("; Symbol operations"),
    },
    // Variant operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_field_count(ptr)",
        category: Some("; Variant operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_tag(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_field_at(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_append(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_last(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_variant_init(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_0(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_1(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_2(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_3(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_4(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_5(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_6(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_7(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_8(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_9(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_10(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_11(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_variant_12(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_unpack_variant(ptr, i64)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_symbol_eq_cstr(ptr, ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // Symbol operations
    ("symbol.=", "patch_seq_symbol_equal"),
    // Variant operations
    ("variant.field-count", "patch_seq_variant_field_count"),
    ("variant.tag", "patch_seq_variant_tag"),
    ("variant.field-at", "patch_seq_variant_field_at"),
    ("variant.append", "patch_seq_variant_append"),
    ("variant.last", "patch_seq_variant_last"),
    ("variant.init", "patch_seq_variant_init"),
    ("variant.make-0", "patch_seq_make_variant_0"),
    ("variant.make-1", "patch_seq_make_variant_1"),
    ("variant.make-2", "patch_seq_make_variant_2"),
    ("variant.make-3", "patch_seq_make_variant_3"),
    ("variant.make-4", "patch_seq_make_variant_4"),
    ("variant.make-5", "patch_seq_make_variant_5"),
    ("variant.make-6", "patch_seq_make_variant_6"),
    ("variant.make-7", "patch_seq_make_variant_7"),
    ("variant.make-8", "patch_seq_make_variant_8"),
    ("variant.make-9", "patch_seq_make_variant_9"),
    ("variant.make-10", "patch_seq_make_variant_10"),
    ("variant.make-11", "patch_seq_make_variant_11"),
    ("variant.make-12", "patch_seq_make_variant_12"),
    // wrap-N aliases for dynamic variant construction (SON)
    ("wrap-0", "patch_seq_make_variant_0"),
    ("wrap-1", "patch_seq_make_variant_1"),
    ("wrap-2", "patch_seq_make_variant_2"),
    ("wrap-3", "patch_seq_make_variant_3"),
    ("wrap-4", "patch_seq_make_variant_4"),
    ("wrap-5", "patch_seq_make_variant_5"),
    ("wrap-6", "patch_seq_make_variant_6"),
    ("wrap-7", "patch_seq_make_variant_7"),
    ("wrap-8", "patch_seq_make_variant_8"),
    ("wrap-9", "patch_seq_make_variant_9"),
    ("wrap-10", "patch_seq_make_variant_10"),
    ("wrap-11", "patch_seq_make_variant_11"),
    ("wrap-12", "patch_seq_make_variant_12"),
];
