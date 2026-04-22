//! Bitwise operations: `band`, `bor`, `bxor`, `bnot`, `shl`, `shr`,
//! plus bit-counting intrinsics (`popcount`, `clz`, `ctz`) and
//! `int_bits` (raw bit pattern as integer).

use crate::stack::{Stack, pop, pop_two, push};
use crate::value::Value;

// ============================================================================
// Bitwise Operations
// ============================================================================

/// Bitwise AND
///
/// Stack effect: ( a b -- a&b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_band(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "band") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe { push(rest, Value::Int(a_val & b_val)) },
        _ => panic!("band: expected two integers on stack"),
    }
}

/// Bitwise OR
///
/// Stack effect: ( a b -- a|b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_bor(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "bor") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe { push(rest, Value::Int(a_val | b_val)) },
        _ => panic!("bor: expected two integers on stack"),
    }
}

/// Bitwise XOR
///
/// Stack effect: ( a b -- a^b )
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_bxor(stack: Stack) -> Stack {
    let (rest, a, b) = unsafe { pop_two(stack, "bxor") };
    match (a, b) {
        (Value::Int(a_val), Value::Int(b_val)) => unsafe { push(rest, Value::Int(a_val ^ b_val)) },
        _ => panic!("bxor: expected two integers on stack"),
    }
}

/// Bitwise NOT (one's complement)
///
/// Stack effect: ( a -- !a )
///
/// # Safety
/// Stack must have one Int value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_bnot(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "bnot: stack is empty");
    let (rest, a) = unsafe { pop(stack) };
    match a {
        Value::Int(a_val) => unsafe { push(rest, Value::Int(!a_val)) },
        _ => panic!("bnot: expected integer on stack"),
    }
}

/// Shift left
///
/// Stack effect: ( value count -- result )
/// Shifts value left by count bits. Negative count or count >= 64 returns 0.
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_shl(stack: Stack) -> Stack {
    let (rest, value, count) = unsafe { pop_two(stack, "shl") };
    match (value, count) {
        (Value::Int(v), Value::Int(c)) => {
            // Use checked_shl to avoid undefined behavior for out-of-range shifts
            // Negative counts become large u32 values, which correctly return None
            let result = if c < 0 {
                0
            } else {
                v.checked_shl(c as u32).unwrap_or(0)
            };
            unsafe { push(rest, Value::Int(result)) }
        }
        _ => panic!("shl: expected two integers on stack"),
    }
}

/// Logical shift right (zero-fill)
///
/// Stack effect: ( value count -- result )
/// Shifts value right by count bits, filling with zeros.
/// Negative count or count >= 64 returns 0.
///
/// # Safety
/// Stack must have two Int values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_shr(stack: Stack) -> Stack {
    let (rest, value, count) = unsafe { pop_two(stack, "shr") };
    match (value, count) {
        (Value::Int(v), Value::Int(c)) => {
            // Use checked_shr to avoid undefined behavior for out-of-range shifts
            // Cast to u64 for logical (zero-fill) shift behavior
            let result = if c < 0 {
                0
            } else {
                (v as u64).checked_shr(c as u32).unwrap_or(0) as i64
            };
            unsafe { push(rest, Value::Int(result)) }
        }
        _ => panic!("shr: expected two integers on stack"),
    }
}

/// Population count (count number of 1 bits)
///
/// Stack effect: ( n -- count )
///
/// # Safety
/// Stack must have one Int value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_popcount(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "popcount: stack is empty");
    let (rest, a) = unsafe { pop(stack) };
    match a {
        Value::Int(v) => unsafe { push(rest, Value::Int(v.count_ones() as i64)) },
        _ => panic!("popcount: expected integer on stack"),
    }
}

/// Count leading zeros
///
/// Stack effect: ( n -- count )
///
/// # Safety
/// Stack must have one Int value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_clz(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "clz: stack is empty");
    let (rest, a) = unsafe { pop(stack) };
    match a {
        Value::Int(v) => unsafe { push(rest, Value::Int(v.leading_zeros() as i64)) },
        _ => panic!("clz: expected integer on stack"),
    }
}

/// Count trailing zeros
///
/// Stack effect: ( n -- count )
///
/// # Safety
/// Stack must have one Int value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_ctz(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "ctz: stack is empty");
    let (rest, a) = unsafe { pop(stack) };
    match a {
        Value::Int(v) => unsafe { push(rest, Value::Int(v.trailing_zeros() as i64)) },
        _ => panic!("ctz: expected integer on stack"),
    }
}

/// Push the bit width of Int (64)
///
/// Stack effect: ( -- 64 )
///
/// # Safety
/// Always safe to call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_int_bits(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Int(64)) }
}
