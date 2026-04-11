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
///
/// `Release` ordering is sufficient: the write happens-before the C `main`
/// reads the value with `Acquire`, after `scheduler_run` has joined all
/// strands. There is no other reader.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_set_exit_code(code: i64) {
    EXIT_CODE.store(code, Ordering::Release);
}

/// Get the process exit code.
///
/// Called by the generated C `main` function after `scheduler_run` returns.
/// Returns 0 if `patch_seq_set_exit_code` was never called (void main).
///
/// `Acquire` pairs with the `Release` store in `patch_seq_set_exit_code`.
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_get_exit_code() -> i64 {
    EXIT_CODE.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Tests share a global AtomicI64, so they must run serially to avoid
    // interleaving writes from one test with reads from another.

    #[test]
    #[serial]
    fn test_default_is_zero() {
        patch_seq_set_exit_code(0);
        assert_eq!(patch_seq_get_exit_code(), 0);
    }

    #[test]
    #[serial]
    fn test_set_and_get() {
        patch_seq_set_exit_code(42);
        assert_eq!(patch_seq_get_exit_code(), 42);
        patch_seq_set_exit_code(0);
    }

    #[test]
    #[serial]
    fn test_negative_exit_code() {
        patch_seq_set_exit_code(-1);
        assert_eq!(patch_seq_get_exit_code(), -1);
        patch_seq_set_exit_code(0);
    }
}
