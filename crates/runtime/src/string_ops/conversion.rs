//! Conversions and escape/compare helpers that aren't plain string-to-string.

use crate::error::set_runtime_error;
use crate::seqstring::global_string;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_symbol_equal(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "symbol_equal: stack is empty");

    let (stack, sym2_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "symbol_equal: need two symbols");
    let (stack, sym1_val) = unsafe { pop(stack) };

    match (sym1_val, sym2_val) {
        (Value::Symbol(s1), Value::Symbol(s2)) => {
            // Fast path: both interned symbols -> O(1) pointer comparison
            let equal = if s1.is_interned() && s2.is_interned() {
                s1.as_ptr() == s2.as_ptr()
            } else {
                // Fallback: byte-level comparison for runtime-created
                // symbols. Must be `as_bytes()`, not `as_str_or_empty()` —
                // otherwise two distinct non-UTF-8 symbols both collapse
                // to "" and are reported equal. (Symbols are normally
                // ASCII identifiers so this rarely matters in practice,
                // but the contract should be byte-precise.)
                s1.as_bytes() == s2.as_bytes()
            };
            unsafe { push(stack, Value::Bool(equal)) }
        }
        _ => panic!("symbol_equal: expected two symbols on stack"),
    }
}

/// Escape a string for JSON output
///
/// Stack effect: ( str -- str )
///
/// Escapes special characters according to JSON spec:
/// - `"` → `\"`
/// - `\` → `\\`
/// - newline → `\n`
/// - carriage return → `\r`
/// - tab → `\t`
/// - backspace → `\b`
/// - form feed → `\f`
/// - Control characters (0x00-0x1F) → `\uXXXX`
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_json_escape(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "json_escape: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            let input = s.as_str_or_empty();
            let mut result = String::with_capacity(input.len() + 16);

            for ch in input.chars() {
                match ch {
                    '"' => result.push_str("\\\""),
                    '\\' => result.push_str("\\\\"),
                    '\n' => result.push_str("\\n"),
                    '\r' => result.push_str("\\r"),
                    '\t' => result.push_str("\\t"),
                    '\x08' => result.push_str("\\b"), // backspace
                    '\x0C' => result.push_str("\\f"), // form feed
                    // Control characters (0x00-0x1F except those handled above)
                    // RFC 8259 uses uppercase hex in examples for Unicode escapes
                    c if c.is_control() => {
                        result.push_str(&format!("\\u{:04X}", c as u32));
                    }
                    c => result.push(c),
                }
            }

            unsafe { push(stack, Value::String(global_string(result))) }
        }
        _ => panic!("json_escape: expected String on stack"),
    }
}

/// Convert String to Int: ( String -- Int Bool )
/// Returns the parsed int and true on success, or 0 and false on failure.
/// Accepts integers in range [-9223372036854775808, 9223372036854775807] (i64).
/// Trims leading/trailing whitespace before parsing.
/// Leading zeros are accepted (e.g., "007" parses to 7).
///
/// # Error Handling
/// - Empty stack: Sets runtime error, returns unchanged stack
/// - Type mismatch: Sets runtime error, returns 0 and false
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_to_int(stack: Stack) -> Stack {
    if stack.is_null() {
        set_runtime_error("string->int: stack is empty");
        return stack;
    }
    let (stack, val) = unsafe { pop(stack) };

    match val {
        Value::String(s) => match s.as_str_or_empty().trim().parse::<i64>() {
            Ok(i) => {
                let stack = unsafe { push(stack, Value::Int(i)) };
                unsafe { push(stack, Value::Bool(true)) }
            }
            Err(_) => {
                let stack = unsafe { push(stack, Value::Int(0)) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        _ => {
            set_runtime_error("string->int: expected String on stack");
            let stack = unsafe { push(stack, Value::Int(0)) };
            unsafe { push(stack, Value::Bool(false)) }
        }
    }
}
