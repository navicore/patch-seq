//! Runtime declarations for channels and the strand scheduler.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // Channel operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_make_channel(ptr)",
        category: Some("; Concurrency operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_chan_send(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_chan_receive(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_close_channel(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_yield_strand(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare void @patch_seq_maybe_yield()",
        category: None,
    },
    // Scheduler operations
    RuntimeDecl {
        decl: "declare void @patch_seq_scheduler_init()",
        category: Some("; Scheduler operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_scheduler_run()",
        category: None,
    },
    RuntimeDecl {
        decl: "declare i64 @patch_seq_strand_spawn(ptr, ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    ("chan.make", "patch_seq_make_channel"),
    ("chan.send", "patch_seq_chan_send"),
    ("chan.receive", "patch_seq_chan_receive"),
    ("chan.close", "patch_seq_close_channel"),
    ("chan.yield", "patch_seq_yield_strand"),
];
