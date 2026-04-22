//! Peek/pop helpers the compiler uses for conditional branches: extract
//! the top Int or Bool without moving ownership (`peek_*_value`), then
//! free the slot (`pop_stack`).

use crate::stack::{Stack, peek, pop};
use crate::value::Value;

/// # Safety
/// Stack must not be null and top must be an Int.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_peek_int_value(stack: Stack) -> i64 {
    assert!(!stack.is_null(), "peek_int_value: stack is empty");

    // Peek and extract — use pop/push-free path via peek()
    let val = unsafe { peek(stack) };
    match val {
        Value::Int(i) => i,
        other => panic!("peek_int_value: expected Int on stack, got {:?}", other),
    }
}

/// Peek at a bool value on top of stack without popping (for pattern matching)
///
/// # Safety
/// Stack must have a Bool value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_peek_bool_value(stack: Stack) -> bool {
    assert!(!stack.is_null(), "peek_bool_value: stack is empty");

    let val = unsafe { peek(stack) };
    match val {
        Value::Bool(b) => b,
        other => panic!("peek_bool_value: expected Bool on stack, got {:?}", other),
    }
}

/// Helper for popping without extracting the value (for conditionals)
///
/// Pops the top stack node and returns the updated stack pointer.
/// Used after peek_int_value to free the condition value's stack node.
///
/// Stack effect: ( n -- )
///
/// # Safety
/// Stack must not be empty
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_pop_stack(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "pop_stack: stack is empty");
    let (rest, _value) = unsafe { pop(stack) };
    rest
}
