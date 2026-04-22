use super::*;

#[test]
fn test_string_split_simple() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("a b c".to_owned())));
        let stack = push(stack, Value::String(global_string(" ".to_owned())));

        let stack = string_split(stack);

        // Should have a Variant with 3 fields: "a", "b", "c"
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str(), "List");
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::String(global_string("a".to_owned())));
                assert_eq!(v.fields[1], Value::String(global_string("b".to_owned())));
                assert_eq!(v.fields[2], Value::String(global_string("c".to_owned())));
            }
            _ => panic!("Expected Variant, got {:?}", result),
        }
    }
}

#[test]
fn test_string_split_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));
        let stack = push(stack, Value::String(global_string(" ".to_owned())));

        let stack = string_split(stack);

        // Empty string splits to one empty part
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str(), "List");
                assert_eq!(v.fields.len(), 1);
                assert_eq!(v.fields[0], Value::String(global_string("".to_owned())));
            }
            _ => panic!("Expected Variant, got {:?}", result),
        }
    }
}

#[test]
fn test_string_empty_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));

        let stack = string_empty(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_string_empty_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));

        let stack = string_empty(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_string_contains_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("world".to_owned())));

        let stack = string_contains(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_string_contains_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("foo".to_owned())));

        let stack = string_contains(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_string_starts_with_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("hello".to_owned())));

        let stack = string_starts_with(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_string_starts_with_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("world".to_owned())));

        let stack = string_starts_with(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_http_request_line_parsing() {
    // Real-world use case: Parse "GET /api/users HTTP/1.1"
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("GET /api/users HTTP/1.1".to_owned())),
        );
        let stack = push(stack, Value::String(global_string(" ".to_owned())));

        let stack = string_split(stack);

        // Should have a Variant with 3 fields: "GET", "/api/users", "HTTP/1.1"
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str(), "List");
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::String(global_string("GET".to_owned())));
                assert_eq!(
                    v.fields[1],
                    Value::String(global_string("/api/users".to_owned()))
                );
                assert_eq!(
                    v.fields[2],
                    Value::String(global_string("HTTP/1.1".to_owned()))
                );
            }
            _ => panic!("Expected Variant, got {:?}", result),
        }
    }
}

#[test]
fn test_path_routing() {
    // Real-world use case: Check if path starts with "/api/"
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("/api/users".to_owned())));
        let stack = push(stack, Value::String(global_string("/api/".to_owned())));

        let stack = string_starts_with(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_string_concat() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("Hello, ".to_owned())));
        let stack = push(stack, Value::String(global_string("World!".to_owned())));

        let stack = string_concat(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("Hello, World!".to_owned()))
        );
    }
}

#[test]
fn test_string_length() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("Hello".to_owned())));

        let stack = string_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(5));
    }
}

#[test]
fn test_string_length_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));

        let stack = string_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_string_trim() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("  Hello, World!  ".to_owned())),
        );

        let stack = string_trim(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("Hello, World!".to_owned()))
        );
    }
}

#[test]
fn test_string_to_upper() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("Hello, World!".to_owned())),
        );

        let stack = string_to_upper(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("HELLO, WORLD!".to_owned()))
        );
    }
}

#[test]
fn test_string_to_lower() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("Hello, World!".to_owned())),
        );

        let stack = string_to_lower(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("hello, world!".to_owned()))
        );
    }
}

#[test]
fn test_http_header_content_length() {
    // Real-world use case: Build "Content-Length: 42" header
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("Content-Length: ".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("42".to_owned())));

        let stack = string_concat(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("Content-Length: 42".to_owned()))
        );
    }
}

#[test]
fn test_string_equal_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::String(global_string("hello".to_owned())));

        let stack = string_equal(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_string_equal_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::String(global_string("world".to_owned())));

        let stack = string_equal(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_string_equal_empty_strings() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));
        let stack = push(stack, Value::String(global_string("".to_owned())));

        let stack = string_equal(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

// UTF-8 String Primitives Tests

#[test]
fn test_string_length_utf8() {
    // "héllo" has 5 characters but 6 bytes (é is 2 bytes in UTF-8)
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("héllo".to_owned())));

        let stack = string_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(5)); // Characters, not bytes
    }
}

#[test]
fn test_string_length_emoji() {
    // Emoji is one code point but multiple bytes
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hi🎉".to_owned())));

        let stack = string_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(3)); // 'h', 'i', and emoji
    }
}

#[test]
fn test_string_byte_length_ascii() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));

        let stack = string_byte_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(5)); // Same as char length for ASCII
    }
}

#[test]
fn test_string_byte_length_utf8() {
    // "héllo" has 5 characters but 6 bytes
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("héllo".to_owned())));

        let stack = string_byte_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(6)); // Bytes, not characters
    }
}

#[test]
fn test_string_char_at_ascii() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(0));

        let stack = string_char_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(104)); // 'h' = 104
    }
}

#[test]
fn test_string_char_at_utf8() {
    // Get the é character at index 1 in "héllo"
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("héllo".to_owned())));
        let stack = push(stack, Value::Int(1));

        let stack = string_char_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(233)); // 'é' = U+00E9 = 233
    }
}

#[test]
fn test_string_char_at_out_of_bounds() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(10)); // Out of bounds

        let stack = string_char_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-1));
    }
}

#[test]
fn test_string_char_at_negative() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(-1));

        let stack = string_char_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-1));
    }
}

#[test]
fn test_string_substring_simple() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(1)); // start
        let stack = push(stack, Value::Int(3)); // len

        let stack = string_substring(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("ell".to_owned())));
    }
}

#[test]
fn test_string_substring_utf8() {
    // "héllo" - get "éll" (characters 1-3)
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("héllo".to_owned())));
        let stack = push(stack, Value::Int(1)); // start
        let stack = push(stack, Value::Int(3)); // len

        let stack = string_substring(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("éll".to_owned())));
    }
}

#[test]
fn test_string_substring_clamp() {
    // Request more than available - should clamp
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(2)); // start
        let stack = push(stack, Value::Int(100)); // len (way too long)

        let stack = string_substring(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("llo".to_owned())));
    }
}

#[test]
fn test_string_substring_beyond_end() {
    // Start beyond end - returns empty
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::Int(10)); // start (beyond end)
        let stack = push(stack, Value::Int(3)); // len

        let stack = string_substring(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("".to_owned())));
    }
}

#[test]
fn test_char_to_string_ascii() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(65)); // 'A'

        let stack = char_to_string(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("A".to_owned())));
    }
}

#[test]
fn test_char_to_string_utf8() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(233)); // 'é' = U+00E9

        let stack = char_to_string(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("é".to_owned())));
    }
}

#[test]
fn test_char_to_string_newline() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(10)); // '\n'

        let stack = char_to_string(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("\n".to_owned())));
    }
}

#[test]
fn test_char_to_string_invalid() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(-1)); // Invalid

        let stack = char_to_string(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("".to_owned())));
    }
}

#[test]
fn test_string_find_found() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("world".to_owned())));

        let stack = string_find(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(6)); // "world" starts at index 6
    }
}

#[test]
fn test_string_find_not_found() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello world".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("xyz".to_owned())));

        let stack = string_find(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-1));
    }
}

#[test]
fn test_string_find_first_match() {
    // Should return first occurrence
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("hello".to_owned())));
        let stack = push(stack, Value::String(global_string("l".to_owned())));

        let stack = string_find(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(2)); // First 'l' is at index 2
    }
}

#[test]
fn test_string_find_utf8() {
    // Find in UTF-8 string - returns character index, not byte index
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("héllo wörld".to_owned())),
        );
        let stack = push(stack, Value::String(global_string("wörld".to_owned())));

        let stack = string_find(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(6)); // Character index, not byte index
    }
}

// JSON Escape Tests

#[test]
fn test_json_escape_quotes() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("hello \"world\"".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("hello \\\"world\\\"".to_owned()))
        );
    }
}

#[test]
fn test_json_escape_backslash() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("path\\to\\file".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("path\\\\to\\\\file".to_owned()))
        );
    }
}

#[test]
fn test_json_escape_newline_tab() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("line1\nline2\ttabbed".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("line1\\nline2\\ttabbed".to_owned()))
        );
    }
}

#[test]
fn test_json_escape_carriage_return() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("line1\r\nline2".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("line1\\r\\nline2".to_owned()))
        );
    }
}

#[test]
fn test_json_escape_control_chars() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        // Test backspace (0x08) and form feed (0x0C)
        let stack = push(
            stack,
            Value::String(global_string("a\x08b\x0Cc".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("a\\bb\\fc".to_owned())));
    }
}

#[test]
fn test_json_escape_unicode_control() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        // Test null character (0x00) - should be escaped as \u0000 (uppercase hex per RFC 8259)
        let stack = push(stack, Value::String(global_string("a\x00b".to_owned())));

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("a\\u0000b".to_owned())));
    }
}

#[test]
fn test_json_escape_mixed_special_chars() {
    // Test combination of multiple special characters
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("Line 1\nLine \"2\"\ttab\r\n".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string(
                "Line 1\\nLine \\\"2\\\"\\ttab\\r\\n".to_owned()
            ))
        );
    }
}

#[test]
fn test_json_escape_no_change() {
    // Normal string without special chars should pass through unchanged
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("Hello, World!".to_owned())),
        );

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(
            result,
            Value::String(global_string("Hello, World!".to_owned()))
        );
    }
}

#[test]
fn test_json_escape_empty_string() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));

        let stack = json_escape(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(global_string("".to_owned())));
    }
}

// string->int tests

#[test]
fn test_string_to_int_success() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("42".to_owned())));

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        assert_eq!(value, Value::Int(42));
    }
}

#[test]
fn test_string_to_int_negative() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("-99".to_owned())));

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        assert_eq!(value, Value::Int(-99));
    }
}

#[test]
fn test_string_to_int_with_whitespace() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("  123  ".to_owned())));

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        assert_eq!(value, Value::Int(123));
    }
}

#[test]
fn test_string_to_int_failure() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(
            stack,
            Value::String(global_string("not a number".to_owned())),
        );

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        assert_eq!(value, Value::Int(0));
    }
}

#[test]
fn test_string_to_int_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("".to_owned())));

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        assert_eq!(value, Value::Int(0));
    }
}

#[test]
fn test_string_to_int_leading_zeros() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String(global_string("007".to_owned())));

        let stack = string_to_int(stack);

        let (stack, success) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(success, Value::Bool(true));
        assert_eq!(value, Value::Int(7));
    }
}

#[test]
fn test_string_to_int_type_error() {
    unsafe {
        crate::error::clear_runtime_error();

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // Wrong type - should be String

        let stack = string_to_int(stack);

        // Should have set an error
        assert!(crate::error::has_runtime_error());
        let error = crate::error::take_runtime_error().unwrap();
        assert!(error.contains("expected String"));

        // Should return (0, false)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::Int(0));
    }
}

// =========================================================================
// string.join tests
// =========================================================================

#[test]
fn test_string_join_strings() {
    unsafe {
        use crate::value::VariantData;
        use std::sync::Arc;

        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![
                Value::String(global_string("a".to_string())),
                Value::String(global_string("b".to_string())),
                Value::String(global_string("c".to_string())),
            ],
        )));
        let stack = push(stack, list);
        let stack = push(stack, Value::String(global_string(", ".to_string())));
        let stack = patch_seq_string_join(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "a, b, c"),
            _ => panic!("Expected String, got {:?}", result),
        }
    }
}

#[test]
fn test_string_join_empty_list() {
    unsafe {
        use crate::value::VariantData;
        use std::sync::Arc;

        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![],
        )));
        let stack = push(stack, list);
        let stack = push(stack, Value::String(global_string(", ".to_string())));
        let stack = patch_seq_string_join(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), ""),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_string_join_single_element() {
    unsafe {
        use crate::value::VariantData;
        use std::sync::Arc;

        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::String(global_string("only".to_string()))],
        )));
        let stack = push(stack, list);
        let stack = push(stack, Value::String(global_string(", ".to_string())));
        let stack = patch_seq_string_join(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "only"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_string_join_mixed_types() {
    unsafe {
        use crate::value::VariantData;
        use std::sync::Arc;

        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![
                Value::Int(1),
                Value::Bool(true),
                Value::String(global_string("x".to_string())),
            ],
        )));
        let stack = push(stack, list);
        let stack = push(stack, Value::String(global_string(" ".to_string())));
        let stack = patch_seq_string_join(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "1 true x"),
            _ => panic!("Expected String"),
        }
    }
}
