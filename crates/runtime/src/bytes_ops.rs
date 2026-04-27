//! Byte construction primitives.
//!
//! Conversions from numeric values to fixed-width big-endian byte
//! strings, for binary protocol encoders written in Seq itself
//! (OSC, DNS, NTP, MessagePack, Protobuf, etc).
//!
//! Output Strings are byte-clean — `SeqString` carries arbitrary
//! bytes since the byte-cleanliness landing, so a Seq program can
//! pack and `concat` these into a complete wire payload and hand it
//! straight to `udp.send-to` / `tcp.write` / `file.spit`.
//!
//! These functions are exported with C ABI for LLVM codegen.

use crate::seqstring::global_bytes;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// Convert Int to 4-byte big-endian i32 String: ( Int -- String )
///
/// The value is narrowed to `i32` via `as i32`, which preserves the
/// low 32 bits. Out-of-range Ints wrap rather than fault — the same
/// semantics as Rust's `as` cast. OSC encoders pass values they
/// already know fit in i32; callers needing range checks should
/// validate before calling.
///
/// # Safety
/// Stack must have an Int value on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_int_to_bytes_i32_be(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "int.to-bytes-i32-be: stack is empty");
    let (stack, val) = unsafe { pop(stack) };

    match val {
        Value::Int(i) => {
            let bytes = (i as i32).to_be_bytes().to_vec();
            unsafe { push(stack, Value::String(global_bytes(bytes))) }
        }
        _ => panic!("int.to-bytes-i32-be: expected Int on stack, got {:?}", val),
    }
}

/// Convert Float to 4-byte big-endian f32 String: ( Float -- String )
///
/// Precision-converts the f64 to f32 via `as f32`, then emits the
/// IEEE-754 binary32 big-endian bytes. NaN/Infinity round-trip
/// through the standard IEEE-754 encoding.
///
/// # Safety
/// Stack must have a Float value on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_float_to_bytes_f32_be(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "float.to-bytes-f32-be: stack is empty");
    let (stack, val) = unsafe { pop(stack) };

    match val {
        Value::Float(f) => {
            let bytes = (f as f32).to_be_bytes().to_vec();
            unsafe { push(stack, Value::String(global_bytes(bytes))) }
        }
        _ => panic!(
            "float.to-bytes-f32-be: expected Float on stack, got {:?}",
            val
        ),
    }
}

pub use patch_seq_float_to_bytes_f32_be as float_to_bytes_f32_be;
pub use patch_seq_int_to_bytes_i32_be as int_to_bytes_i32_be;

#[cfg(test)]
mod tests;
