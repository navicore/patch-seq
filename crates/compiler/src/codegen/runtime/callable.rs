//! Runtime declarations for first-class callables: quotations, dataflow
//! combinators (dip, keep, bi, __if__), strand spawning, `cond`, and the
//! peek helpers used by the codegen to inspect quotation values.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Quotation operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_push_quotation(ptr, i64, i64)",
        category: Some("; Quotation operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_call(ptr)",
        category: None,
    },
    // Dataflow combinators
    RuntimeDecl {
        decl: "declare ptr @patch_seq_dip(ptr)",
        category: Some("; Dataflow combinators"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_keep(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_bi(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_if(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_peek_is_quotation(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_peek_quotation_fn_ptr(ptr)",
        category: None,
    },
    // Strand / weave operations (quotation-consuming)
    RuntimeDecl {
        decl: "declare ptr @patch_seq_spawn(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_weave(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_resume(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_weave_cancel(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_yield(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_cond(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    ("call", "patch_seq_call"),
    ("dip", "patch_seq_dip"),
    ("keep", "patch_seq_keep"),
    ("bi", "patch_seq_bi"),
    ("__if__", "patch_seq_if"),
    ("strand.spawn", "patch_seq_spawn"),
    ("strand.weave", "patch_seq_weave"),
    ("strand.resume", "patch_seq_resume"),
    ("strand.weave-cancel", "patch_seq_weave_cancel"),
    ("yield", "patch_seq_yield"),
    ("cond", "patch_seq_cond"),
];
