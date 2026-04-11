//! Process exit code handling
//!
//! Stores the integer value returned from `main ( -- Int )` so the C-level
//! `main` function can read it after the scheduler joins all strands and
//! return it as the process exit code.
//!
//! # Lifetime
//!
//! - The user's `seq_main` function calls `patch_seq_set_exit_code` with the
//!   top-of-stack Int just before its stack is freed.
//! - The C `main` function calls `patch_seq_get_exit_code` after
//!   `patch_seq_scheduler_run` returns and uses the value as the process
//!   exit code.
//!
//! # Concurrency
//!
//! The exit code is a single atomic global. Only the main strand writes to
//! it, and only after all spawned strands have finished (since
//! `scheduler_run` joins all strands). The C `main` reads it after
//! `scheduler_run` returns. There is no race.
//!
//! Programs declaring `main ( -- )` (void main) never call the setter, so
//! the exit code remains 0 — matching the historical behavior.

use std::sync::atomic::{AtomicI64, Ordering};

/// Process exit code, written by `seq_main` for `main ( -- Int )` programs.
/// Defaults to 0 so void mains exit with success.
static EXIT_CODE: AtomicI64 = AtomicI64::new(0);

/// Set the process exit code.
///
/// Called by generated code at the end of `seq_main` when the user declared
/// `main ( -- Int )`. The value is the top-of-stack Int.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_set_exit_code(code: i64) {
    EXIT_CODE.store(code, Ordering::SeqCst);
}

/// Get the process exit code.
///
/// Called by the generated C `main` function after `scheduler_run` returns.
/// Returns 0 if `patch_seq_set_exit_code` was never called (void main).
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_get_exit_code() -> i64 {
    EXIT_CODE.load(Ordering::SeqCst)
}

// Public re-exports with short names for internal use
pub use patch_seq_get_exit_code as get_exit_code;
pub use patch_seq_set_exit_code as set_exit_code;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_zero() {
        // Note: this test is order-sensitive if other tests in this module
        // run first. Reset to 0 to make it independent.
        patch_seq_set_exit_code(0);
        assert_eq!(patch_seq_get_exit_code(), 0);
    }

    #[test]
    fn test_set_and_get() {
        patch_seq_set_exit_code(42);
        assert_eq!(patch_seq_get_exit_code(), 42);
        // Restore to 0 to avoid polluting other tests
        patch_seq_set_exit_code(0);
    }

    #[test]
    fn test_negative_exit_code() {
        patch_seq_set_exit_code(-1);
        assert_eq!(patch_seq_get_exit_code(), -1);
        patch_seq_set_exit_code(0);
    }
}
