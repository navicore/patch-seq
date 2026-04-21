//! Runtime declarations for TCP socket operations.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tcp_listen(ptr)",
        category: Some("; TCP operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tcp_accept(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tcp_read(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tcp_write(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_tcp_close(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    ("tcp.listen", "patch_seq_tcp_listen"),
    ("tcp.accept", "patch_seq_tcp_accept"),
    ("tcp.read", "patch_seq_tcp_read"),
    ("tcp.write", "patch_seq_tcp_write"),
    ("tcp.close", "patch_seq_tcp_close"),
];
