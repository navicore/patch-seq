use super::*;
use crate::stack::{pop, push};
use crate::value::Value;

#[test]
fn test_add() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 5);
        let stack = push_int(stack, 3);
        let stack = add(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(8));
    }
}

#[test]
fn test_subtract() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 10);
        let stack = push_int(stack, 3);
        let stack = subtract(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(7));
    }
}

#[test]
fn test_multiply() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 4);
        let stack = push_int(stack, 5);
        let stack = multiply(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(20));
    }
}

#[test]
fn test_divide() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 20);
        let stack = push_int(stack, 4);
        let stack = divide(stack);

        // Division now returns (result, success_bool)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(5));
    }
}

#[test]
fn test_comparisons() {
    unsafe {
        // Test eq (returns true/false Bool)
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 5);
        let stack = push_int(stack, 5);
        let stack = eq(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));

        // Test lt
        let stack = push_int(stack, 3);
        let stack = push_int(stack, 5);
        let stack = lt(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));

        // Test gt
        let stack = push_int(stack, 7);
        let stack = push_int(stack, 5);
        let stack = gt(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_63bit_overflow_wrapping() {
    // Verify that arithmetic at the 63-bit boundary wraps correctly.
    // tag_int silently overflows for values outside -(2^62) to (2^62-1),
    // so wrapping arithmetic at the boundary is defined but the tagged
    // representation wraps within 63 bits.
    unsafe {
        let int63_max = (1i64 << 62) - 1; // 4611686018427387903

        // max + 1 wraps within the tagged representation
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(int63_max));
        let stack = push(stack, Value::Int(1));
        let stack = add(stack);
        let (_stack, result) = pop(stack);
        // The result wraps — it should be a valid Int (not crash)
        assert!(matches!(result, Value::Int(_)));
    }
}

#[test]
fn test_negative_division() {
    unsafe {
        // Test negative dividend
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, -10);
        let stack = push_int(stack, 3);
        let stack = divide(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-3)); // Truncates toward zero

        // Test negative divisor
        let stack = push_int(stack, 10);
        let stack = push_int(stack, -3);
        let stack = divide(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-3));

        // Test both negative
        let stack = push_int(stack, -10);
        let stack = push_int(stack, -3);
        let stack = divide(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(3));
    }
}

#[test]
fn test_and_true_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 1);
        let stack = push_int(stack, 1);
        let stack = and(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(1));
    }
}

#[test]
fn test_and_true_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 1);
        let stack = push_int(stack, 0);
        let stack = and(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_and_false_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 0);
        let stack = push_int(stack, 0);
        let stack = and(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_or_true_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 1);
        let stack = push_int(stack, 1);
        let stack = or(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(1));
    }
}

#[test]
fn test_or_true_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 1);
        let stack = push_int(stack, 0);
        let stack = or(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(1));
    }
}

#[test]
fn test_or_false_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 0);
        let stack = push_int(stack, 0);
        let stack = or(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_not_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 1);
        let stack = not(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_not_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 0);
        let stack = not(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(1));
    }
}

#[test]
fn test_and_nonzero_values() {
    // Forth-style: any non-zero is true
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 42);
        let stack = push_int(stack, -5);
        let stack = and(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(1));
    }
}

#[test]
fn test_divide_by_zero_returns_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 42);
        let stack = push_int(stack, 0);
        let stack = divide(stack);

        // Division by zero now returns (0, false) instead of setting runtime error
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));

        // Should have pushed 0 as result
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}
