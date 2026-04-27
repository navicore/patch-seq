//! Encoding operations for Seq (Base64, Hex)
//!
//! These functions are exported with C ABI for LLVM codegen to call.
//!
//! # API
//!
//! ```seq
//! # Base64 encoding/decoding
//! "hello" encoding.base64-encode     # ( String -- String ) "aGVsbG8="
//! "aGVsbG8=" encoding.base64-decode  # ( String -- String Bool )
//!
//! # URL-safe Base64 (for JWTs, URLs)
//! data encoding.base64url-encode     # ( String -- String )
//! encoded encoding.base64url-decode  # ( String -- String Bool )
//!
//! # Hex encoding/decoding
//! "hello" encoding.hex-encode        # ( String -- String ) "68656c6c6f"
//! "68656c6c6f" encoding.hex-decode   # ( String -- String Bool )
//! ```

use crate::seqstring::global_string;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

use base64::prelude::*;

/// Encode a string to Base64 (standard alphabet with padding)
///
/// Stack effect: ( String -- String )
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_base64_encode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "base64-encode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            let encoded = BASE64_STANDARD.encode(s.as_bytes());
            unsafe { push(stack, Value::String(global_string(encoded))) }
        }
        _ => panic!("base64-encode: expected String on stack, got {:?}", value),
    }
}

/// Decode a Base64 string (standard alphabet)
///
/// Stack effect: ( String -- String Bool )
///
/// Returns the decoded string and true on success, empty string and false on failure.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_base64_decode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "base64-decode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => match BASE64_STANDARD.decode(s.as_bytes()) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(decoded) => {
                    let stack = unsafe { push(stack, Value::String(global_string(decoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                Err(_) => {
                    // Decoded bytes are not valid UTF-8
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            },
            Err(_) => {
                // Invalid Base64 input
                let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        _ => panic!("base64-decode: expected String on stack, got {:?}", value),
    }
}

/// Encode a string to URL-safe Base64 (no padding)
///
/// Stack effect: ( String -- String )
///
/// Uses URL-safe alphabet (- and _ instead of + and /) with no padding.
/// Suitable for JWTs, URLs, and filenames.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_base64url_encode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "base64url-encode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            let encoded = BASE64_URL_SAFE_NO_PAD.encode(s.as_bytes());
            unsafe { push(stack, Value::String(global_string(encoded))) }
        }
        _ => panic!(
            "base64url-encode: expected String on stack, got {:?}",
            value
        ),
    }
}

/// Decode a URL-safe Base64 string (no padding expected)
///
/// Stack effect: ( String -- String Bool )
///
/// Returns the decoded string and true on success, empty string and false on failure.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_base64url_decode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "base64url-decode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => match BASE64_URL_SAFE_NO_PAD.decode(s.as_bytes()) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(decoded) => {
                    let stack = unsafe { push(stack, Value::String(global_string(decoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                Err(_) => {
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            },
            Err(_) => {
                let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        _ => panic!(
            "base64url-decode: expected String on stack, got {:?}",
            value
        ),
    }
}

/// Encode a string to hexadecimal (lowercase)
///
/// Stack effect: ( String -- String )
///
/// Each byte becomes two hex characters.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_hex_encode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "hex-encode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            let encoded = hex::encode(s.as_bytes());
            unsafe { push(stack, Value::String(global_string(encoded))) }
        }
        _ => panic!("hex-encode: expected String on stack, got {:?}", value),
    }
}

/// Decode a hexadecimal string
///
/// Stack effect: ( String -- String Bool )
///
/// Returns the decoded string and true on success, empty string and false on failure.
/// Accepts both uppercase and lowercase hex characters.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_hex_decode(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "hex-decode: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => match hex::decode(s.as_str_or_empty()) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(decoded) => {
                    let stack = unsafe { push(stack, Value::String(global_string(decoded))) };
                    unsafe { push(stack, Value::Bool(true)) }
                }
                Err(_) => {
                    // Decoded bytes are not valid UTF-8
                    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                    unsafe { push(stack, Value::Bool(false)) }
                }
            },
            Err(_) => {
                // Invalid hex input
                let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        _ => panic!("hex-decode: expected String on stack, got {:?}", value),
    }
}

#[cfg(test)]
mod tests;
