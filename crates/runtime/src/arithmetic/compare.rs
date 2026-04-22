//! Integer comparison operators + boolean logic: `eq`, `lt`, `gt`, `lte`,
//! `gte`, `neq` plus short-circuit-free `and`/`or`/`not`.

use crate::stack::{Stack, pop, pop_two, push};
use crate::value::Value;

/// # Safety
/// Stack must have two Int values on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_eq(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "=") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Bool(a_val == b_val))
        },
        _ => panic!("=: expected two integers on stack"),
    }
}

/// Less than: <
///
/// Returns Bool (true if a < b, false otherwise)
/// Stack effect: ( a b -- Bool )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_lt(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "<") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe { push(rest, Value::Bool(a_val < b_val)) },
        _ => panic!("<: expected two integers on stack"),
    }
}

/// Greater than: >
///
/// Returns Bool (true if a > b, false otherwise)
/// Stack effect: ( a b -- Bool )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_gt(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, ">") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe { push(rest, Value::Bool(a_val > b_val)) },
        _ => panic!(">: expected two integers on stack"),
    }
}

/// Less than or equal: <=
///
/// Returns Bool (true if a <= b, false otherwise)
/// Stack effect: ( a b -- Bool )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_lte(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "<=") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Bool(a_val <= b_val))
        },
        _ => panic!("<=: expected two integers on stack"),
    }
}

/// Greater than or equal: >=
///
/// Returns Bool (true if a >= b, false otherwise)
/// Stack effect: ( a b -- Bool )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_gte(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, ">=") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Bool(a_val >= b_val))
        },
        _ => panic!(">=: expected two integers on stack"),
    }
}

/// Not equal: <>
///
/// Returns Bool (true if a != b, false otherwise)
/// Stack effect: ( a b -- Bool )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_neq(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "<>") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(rest, Value::Bool(a_val != b_val))
        },
        _ => panic!("<>: expected two integers on stack"),
    }
}

/// Logical AND operation (Forth-style: multiply for boolean values)
///
/// Stack effect: ( a b -- result )
/// where 0 is false, non-zero is true
/// Returns 1 if both are true (non-zero), 0 otherwise
///
/// # Safety
/// Stack must have at least two Int values
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_and(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "and") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(
                rest,
                Value::Int(if a_val != 0 && b_val != 0 { 1 } else { 0 }),
            )
        },
        _ => panic!("and: expected two integers on stack"),
    }
}

/// Logical OR operation (Forth-style)
///
/// Stack effect: ( a b -- result )
/// where 0 is false, non-zero is true
/// Returns 1 if either is true (non-zero), 0 otherwise
///
/// # Safety
/// Stack must have at least two Int values
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_or(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "or") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe {
            push(
                rest,
                Value::Int(if a_val != 0 || b_val != 0 { 1 } else { 0 }),
            )
        },
        _ => panic!("or: expected two integers on stack"),
    }
}

/// Logical NOT operation
///
/// Stack effect: ( a -- result )
/// where 0 is false, non-zero is true
/// Returns 1 if false (0), 0 otherwise
///
/// # Safety
/// Stack must have at least one Int value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_not(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "not: stack is empty");
    let (rest, a) = unsafe { pop(stack) };

    match a {
        Value::Int(a_val) => unsafe { push(rest, Value::Int(if a_val == 0 { 1 } else { 0 })) },
        _ => panic!("not: expected integer on stack"),
    }
}
