use super::*;
use crate::seqstring::global_bytes;
use crate::stack::pop;

#[test]
fn test_base64_encode() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = patch_seq_base64_encode(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "aGVsbG8="),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_base64_decode_success() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("aGVsbG8=".to_string())));
        let stack = patch_seq_base64_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(true)) => assert_eq!(s.as_str_or_empty(), "hello"),
            _ => panic!("Expected (String, true)"),
        }
    }
}

#[test]
fn test_base64_decode_failure() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("not valid base64!!!".to_string())),
        );
        let stack = patch_seq_base64_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(false)) => assert_eq!(s.as_str_or_empty(), ""),
            _ => panic!("Expected (empty String, false)"),
        }
    }
}

#[test]
fn test_base64url_encode() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        // Use input that produces + and / in standard base64
        let stack = push(stack, Value::String(global_string("hello??".to_string())));
        let stack = patch_seq_base64url_encode(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => {
                // Should not contain + or / or =
                assert!(!s.as_str_or_empty().contains('+'));
                assert!(!s.as_str_or_empty().contains('/'));
                assert!(!s.as_str_or_empty().contains('='));
            }
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_base64url_roundtrip() {
    unsafe {
        let original = "hello world! 123";
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string(original.to_string())));
        let stack = patch_seq_base64url_encode(stack);
        let stack = patch_seq_base64url_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(true)) => assert_eq!(s.as_str_or_empty(), original),
            _ => panic!("Expected (String, true)"),
        }
    }
}

#[test]
fn test_hex_encode() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_string())));
        let stack = patch_seq_hex_encode(stack);
        let (_, value) = pop(stack);

        match value {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "68656c6c6f"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_hex_decode_success() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("68656c6c6f".to_string())),
        );
        let stack = patch_seq_hex_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(true)) => assert_eq!(s.as_str_or_empty(), "hello"),
            _ => panic!("Expected (String, true)"),
        }
    }
}

#[test]
fn test_hex_decode_uppercase() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("68656C6C6F".to_string())),
        );
        let stack = patch_seq_hex_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(true)) => assert_eq!(s.as_str_or_empty(), "hello"),
            _ => panic!("Expected (String, true)"),
        }
    }
}

#[test]
fn test_hex_decode_failure() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("not hex!".to_string())));
        let stack = patch_seq_hex_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(false)) => assert_eq!(s.as_str_or_empty(), ""),
            _ => panic!("Expected (empty String, false)"),
        }
    }
}

#[test]
fn test_hex_roundtrip() {
    unsafe {
        let original = "Hello, World! 123";
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string(original.to_string())));
        let stack = patch_seq_hex_encode(stack);
        let stack = patch_seq_hex_decode(stack);

        let (stack, success) = pop(stack);
        let (_, decoded) = pop(stack);

        match (decoded, success) {
            (Value::String(s), Value::Bool(true)) => assert_eq!(s.as_str_or_empty(), original),
            _ => panic!("Expected (String, true)"),
        }
    }
}

// ----------------------------------------------------------------------------
// Byte-cleanliness regression tests for encoding round-trips.
//
// base64 and hex are encodings *for* arbitrary bytes — the canonical use
// case is binary-as-text. Round-tripping non-UTF-8 bytes through encode →
// decode must produce byte-identical output.
// ----------------------------------------------------------------------------

const ENC_BIN: &[u8] = &[0x00, 0xDC, b'x', 0xFF, 0xC3, b'!', 0x80];

#[test]
fn byte_clean_base64_round_trips_binary() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_bytes(ENC_BIN.to_vec())));
        let stack = patch_seq_base64_encode(stack);
        let (stack, encoded) = pop(stack);
        let encoded = match encoded {
            Value::String(s) => s,
            _ => panic!("expected encoded String"),
        };
        // base64 output is always ASCII text.
        let _ = encoded.as_str().expect("base64 output must be valid UTF-8");

        let stack = push(stack, Value::String(encoded));
        let stack = patch_seq_base64_decode(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_, decoded) = pop(stack);
        match decoded {
            Value::String(s) => assert_eq!(s.as_bytes(), ENC_BIN),
            _ => panic!("expected decoded String"),
        }
    }
}

#[test]
fn byte_clean_hex_round_trips_binary() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_bytes(ENC_BIN.to_vec())));
        let stack = patch_seq_hex_encode(stack);
        let (stack, encoded) = pop(stack);
        let encoded = match encoded {
            Value::String(s) => s,
            _ => panic!("expected encoded String"),
        };

        let stack = push(stack, Value::String(encoded));
        let stack = patch_seq_hex_decode(stack);
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        let (_, decoded) = pop(stack);
        match decoded {
            Value::String(s) => assert_eq!(s.as_bytes(), ENC_BIN),
            _ => panic!("expected decoded String"),
        }
    }
}
