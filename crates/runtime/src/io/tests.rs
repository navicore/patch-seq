use super::*;
use crate::value::Value;
use std::ffi::CString;

#[test]
fn test_write_line() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("Hello, World!".into()));
        let _stack = write_line(stack);
    }
}

#[test]
fn test_write() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("no newline".into()));
        let _stack = write(stack);
    }
}

#[test]
fn test_push_string() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let test_str = CString::new("Test").unwrap();
        let stack = push_string(stack, test_str.as_ptr());

        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::String("Test".into()));
    }
}

#[test]
fn test_empty_string() {
    unsafe {
        // Empty string should be handled correctly
        let stack = crate::stack::alloc_test_stack();
        let empty_str = CString::new("").unwrap();
        let stack = push_string(stack, empty_str.as_ptr());

        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::String("".into()));

        // Write empty string should work without panic
        let stack = push(stack, Value::String("".into()));
        let _stack = write_line(stack);
    }
}

#[test]
fn test_unicode_strings() {
    unsafe {
        // Test that Unicode strings are handled correctly
        let stack = crate::stack::alloc_test_stack();
        let unicode_str = CString::new("Hello, 世界! 🌍").unwrap();
        let stack = push_string(stack, unicode_str.as_ptr());

        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::String("Hello, 世界! 🌍".into()));
    }
}

// =========================================================================
// read_n validation tests
// =========================================================================

#[test]
fn test_read_n_valid_input() {
    assert_eq!(super::validate_read_n_count(&Value::Int(0)), Ok(0));
    assert_eq!(super::validate_read_n_count(&Value::Int(100)), Ok(100));
    assert_eq!(
        super::validate_read_n_count(&Value::Int(1024 * 1024)), // 1MB
        Ok(1024 * 1024)
    );
}

#[test]
fn test_read_n_negative_input() {
    let result = super::validate_read_n_count(&Value::Int(-1));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be non-negative"));
}

#[test]
fn test_read_n_large_negative_input() {
    let result = super::validate_read_n_count(&Value::Int(i64::MIN));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be non-negative"));
}

#[test]
fn test_read_n_exceeds_max_bytes() {
    let result = super::validate_read_n_count(&Value::Int(super::READ_N_MAX_BYTES + 1));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exceeds maximum allowed"));
}

#[test]
fn test_read_n_at_max_bytes_ok() {
    // Exactly at the limit should be OK
    let result = super::validate_read_n_count(&Value::Int(super::READ_N_MAX_BYTES));
    assert_eq!(result, Ok(super::READ_N_MAX_BYTES as usize));
}

#[test]
fn test_read_n_wrong_type_string() {
    let result = super::validate_read_n_count(&Value::String("not an int".into()));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected Int"));
}

#[test]
fn test_read_n_wrong_type_bool() {
    let result = super::validate_read_n_count(&Value::Bool(true));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected Int"));
}
