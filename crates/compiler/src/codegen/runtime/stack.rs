//! Runtime declarations for stack shuffle operations (dup, swap, rot, etc.)
//! and the generic `push_value` / `clone_value` helpers.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    RuntimeDecl {
        decl: "declare ptr @patch_seq_dup(ptr)",
        category: Some("; Stack operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_drop_op(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_swap(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_over(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_rot(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_nip(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare void @patch_seq_clone_value(ptr, ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tuck(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_2dup(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_pick_op(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_roll(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_push_value(ptr, %Value)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    ("dup", "patch_seq_dup"),
    ("swap", "patch_seq_swap"),
    ("over", "patch_seq_over"),
    ("rot", "patch_seq_rot"),
    ("nip", "patch_seq_nip"),
    ("tuck", "patch_seq_tuck"),
    ("2dup", "patch_seq_2dup"),
    ("drop", "patch_seq_drop_op"),
    ("pick", "patch_seq_pick_op"),
    ("roll", "patch_seq_roll"),
];
