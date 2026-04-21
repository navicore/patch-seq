//! Runtime declarations for float arithmetic, comparisons, and
//! int/float/string conversions.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    RuntimeDecl {
        decl: "declare ptr @patch_seq_push_float(ptr, double)",
        category: Some("; Float operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_add(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_subtract(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_multiply(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_divide(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_eq(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_lt(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_gt(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_lte(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_gte(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_f_neq(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_int_to_float(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_float_to_int(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_float_to_string(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_string_to_float(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // Float arithmetic
    ("f.add", "patch_seq_f_add"),
    ("f.subtract", "patch_seq_f_subtract"),
    ("f.multiply", "patch_seq_f_multiply"),
    ("f.divide", "patch_seq_f_divide"),
    // Terse float arithmetic aliases
    ("f.+", "patch_seq_f_add"),
    ("f.-", "patch_seq_f_subtract"),
    ("f.*", "patch_seq_f_multiply"),
    ("f./", "patch_seq_f_divide"),
    // Float comparison (symbol form)
    ("f.=", "patch_seq_f_eq"),
    ("f.<", "patch_seq_f_lt"),
    ("f.>", "patch_seq_f_gt"),
    ("f.<=", "patch_seq_f_lte"),
    ("f.>=", "patch_seq_f_gte"),
    ("f.<>", "patch_seq_f_neq"),
    // Float comparison (verbose form)
    ("f.eq", "patch_seq_f_eq"),
    ("f.lt", "patch_seq_f_lt"),
    ("f.gt", "patch_seq_f_gt"),
    ("f.lte", "patch_seq_f_lte"),
    ("f.gte", "patch_seq_f_gte"),
    ("f.neq", "patch_seq_f_neq"),
    // Float type conversions
    ("int->float", "patch_seq_int_to_float"),
    ("float->int", "patch_seq_float_to_int"),
    ("float->string", "patch_seq_float_to_string"),
    ("string->float", "patch_seq_string_to_float"),
];
