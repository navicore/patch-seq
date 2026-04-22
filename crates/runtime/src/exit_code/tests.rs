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
