//! Integer arithmetic: push literals + `add`, `subtract`, `multiply`,
//! `divide`, `modulo`. All use wrapping semantics.

use crate::stack::{Stack, pop_two, push};
use crate::value::Value;

/// Push an integer literal onto the stack (for compiler-generated code)
///
/// Stack effect: ( -- n )
///
/// # Safety
/// Always safe to call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_push_int(stack: Stack, value: i64) -> Stack {
    unsafe { push(stack, Value::Int(value)) }
}

/// Push a boolean literal onto the stack (for compiler-generated code)
///
/// Stack effect: ( -- bool )
///
/// # Safety
/// Always safe to call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_push_bool(stack: Stack, value: bool) -> Stack {
    unsafe { push(stack, Value::Bool(value)) }
}

/// Add two integers
///
/// Stack effect: ( a b -- a+b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_add(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "add") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Int(a_val.wrapping_add(b_val)))
        },
        _ => panic!("add: expected two integers on stack"),
    }
}

/// Subtract two integers (a - b)
///
/// Stack effect: ( a b -- a-b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_subtract(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "subtract") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Int(a_val.wrapping_sub(b_val)))
        },
        _ => panic!("subtract: expected two integers on stack"),
    }
}

/// Multiply two integers
///
/// Stack effect: ( a b -- a*b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_multiply(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "multiply") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Int(a_val.wrapping_mul(b_val)))
        },
        _ => panic!("multiply: expected two integers on stack"),
    }
}

/// Divide two integers (a / b)
///
/// Stack effect: ( a b -- result success )
///
/// Returns the quotient and a Bool success flag.
/// On division by zero, returns (0, false).
/// On success, returns (a/b, true).
///
/// # Safety
/// Stack must have at least two values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_divide(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "divide") };
    match (a, b) {
        (Value::Int(_a_val), Value::Int(0)) => {
            // Division by zero - return 0 and false
            let stack = unsafe { push(rest, Value::Int(0)) };
            unsafe { push(stack, Value::Bool(false)) }
        }
        (Value::Int(a_val), Value::Int(b_val)) => {
            // Use wrapping_div to handle i64::MIN / -1 overflow edge case
            let stack = unsafe { push(rest, Value::Int(a_val.wrapping_div(b_val))) };
            unsafe { push(stack, Value::Bool(true)) }
        }
        _ => {
            // Type error - should not happen with type-checked code
            panic!("divide: expected two integers on stack");
        }
    }
}

/// Modulo (remainder) of two integers (a % b)
///
/// Stack effect: ( a b -- result success )
///
/// Returns the remainder and a Bool success flag.
/// On division by zero, returns (0, false).
/// On success, returns (a%b, true).
///
/// # Safety
/// Stack must have at least two values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_modulo(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "modulo") };
    match (a, b) {
        (Value::Int(_a_val), Value::Int(0)) => {
            // Division by zero - return 0 and false
            let stack = unsafe { push(rest, Value::Int(0)) };
            unsafe { push(stack, Value::Bool(false)) }
        }
        (Value::Int(a_val), Value::Int(b_val)) => {
            // Use wrapping_rem to handle i64::MIN % -1 overflow edge case
            let stack = unsafe { push(rest, Value::Int(a_val.wrapping_rem(b_val))) };
            unsafe { push(stack, Value::Bool(true)) }
        }
        _ => {
            // Type error - should not happen with type-checked code
            panic!("modulo: expected two integers on stack");
        }
    }
}
