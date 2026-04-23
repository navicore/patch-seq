//! Subprocess helpers for the REPL. Runs a compiled Seq program with a
//! bounded timeout so the REPL can't hang on a blocked strand.

use std::io::Read as _;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Default execution timeout in seconds (can be overridden via SEQ_REPL_TIMEOUT)
/// Set higher now that weaves don't block (issue #287 fix) - this is a safety net
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Result of running a command with timeout
#[allow(dead_code)] // Fields kept for API completeness
pub(crate) enum RunResult {
    /// Command completed successfully
    Success { stdout: String, stderr: String },
    /// Command failed with non-zero exit
    Failed {
        stdout: String,
        stderr: String,
        status: ExitStatus,
    },
    /// Command timed out and was killed
    Timeout { timeout_secs: u64 },
    /// Command failed to start
    Error(String),
}

/// Run a compiled program with a timeout
///
/// This prevents the REPL from hanging indefinitely when a program blocks
/// (e.g., creating a weave without resuming it).
pub(crate) fn run_with_timeout(path: &Path) -> RunResult {
    let timeout_secs = std::env::var("SEQ_REPL_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    let timeout = Duration::from_secs(timeout_secs);

    // Spawn the child process
    let mut child = match Command::new(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return RunResult::Error(format!("Failed to start: {}", e)),
    };

    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    // Poll for completion with timeout
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited - collect output
                let stdout = child
                    .stdout
                    .take()
                    .map(|mut s| {
                        let mut buf = String::new();
                        let _ = s.read_to_string(&mut buf);
                        buf
                    })
                    .unwrap_or_default();

                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = String::new();
                        let _ = s.read_to_string(&mut buf);
                        buf
                    })
                    .unwrap_or_default();

                if status.success() {
                    return RunResult::Success { stdout, stderr };
                } else {
                    return RunResult::Failed {
                        stdout,
                        stderr,
                        status,
                    };
                }
            }
            Ok(None) => {
                // Still running - check timeout
                if start.elapsed() >= timeout {
                    // Timeout - kill the process
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie
                    return RunResult::Timeout { timeout_secs };
                }
                // Brief sleep before next poll
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return RunResult::Error(format!("Wait error: {}", e));
            }
        }
    }
}
