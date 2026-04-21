//! Runtime declarations for OS, signal handling, and terminal operations.

use super::RuntimeDecl;

pub(super) static DECLS: &[RuntimeDecl] = &[
    // OS operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_getenv(ptr)",
        category: Some("; OS operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_home_dir(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_current_dir(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_exists(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_is_file(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_is_dir(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_join(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_parent(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_path_filename(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_exit(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_os_name(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_os_arch(ptr)",
        category: None,
    },
    // Signal handling
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_trap(ptr)",
        category: Some("; Signal handling"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_received(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_pending(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_default(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_ignore(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_clear(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigint(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigterm(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sighup(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigpipe(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigusr1(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigusr2(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigchld(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigalrm(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_signal_sigcont(ptr)",
        category: None,
    },
    // Terminal operations
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_raw_mode(ptr)",
        category: Some("; Terminal operations"),
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_read_char(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_read_char_nonblock(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_width(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_height(ptr)",
        category: None,
    },
    RuntimeDecl {
        decl: "declare ptr @patch_seq_terminal_flush(ptr)",
        category: None,
    },
];

pub(super) static SYMBOLS: &[(&str, &str)] = &[
    // OS operations
    ("os.getenv", "patch_seq_getenv"),
    ("os.home-dir", "patch_seq_home_dir"),
    ("os.current-dir", "patch_seq_current_dir"),
    ("os.path-exists", "patch_seq_path_exists"),
    ("os.path-is-file", "patch_seq_path_is_file"),
    ("os.path-is-dir", "patch_seq_path_is_dir"),
    ("os.path-join", "patch_seq_path_join"),
    ("os.path-parent", "patch_seq_path_parent"),
    ("os.path-filename", "patch_seq_path_filename"),
    ("os.exit", "patch_seq_exit"),
    ("os.name", "patch_seq_os_name"),
    ("os.arch", "patch_seq_os_arch"),
    // Signal handling
    ("signal.trap", "patch_seq_signal_trap"),
    ("signal.received?", "patch_seq_signal_received"),
    ("signal.pending?", "patch_seq_signal_pending"),
    ("signal.default", "patch_seq_signal_default"),
    ("signal.ignore", "patch_seq_signal_ignore"),
    ("signal.clear", "patch_seq_signal_clear"),
    ("signal.SIGINT", "patch_seq_signal_sigint"),
    ("signal.SIGTERM", "patch_seq_signal_sigterm"),
    ("signal.SIGHUP", "patch_seq_signal_sighup"),
    ("signal.SIGPIPE", "patch_seq_signal_sigpipe"),
    ("signal.SIGUSR1", "patch_seq_signal_sigusr1"),
    ("signal.SIGUSR2", "patch_seq_signal_sigusr2"),
    ("signal.SIGCHLD", "patch_seq_signal_sigchld"),
    ("signal.SIGALRM", "patch_seq_signal_sigalrm"),
    ("signal.SIGCONT", "patch_seq_signal_sigcont"),
    // Terminal operations
    ("terminal.raw-mode", "patch_seq_terminal_raw_mode"),
    ("terminal.read-char", "patch_seq_terminal_read_char"),
    (
        "terminal.read-char?",
        "patch_seq_terminal_read_char_nonblock",
    ),
    ("terminal.width", "patch_seq_terminal_width"),
    ("terminal.height", "patch_seq_terminal_height"),
    ("terminal.flush", "patch_seq_terminal_flush"),
];
