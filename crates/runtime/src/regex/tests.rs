use super::*;
use seq_core::stack::alloc_stack;

#[test]
fn test_regex_match() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };
    let stack = unsafe { push(stack, Value::String(global_string("wo.ld".to_string()))) };

    let stack = unsafe { patch_seq_regex_match(stack) };
    let (_, value) = unsafe { pop(stack) };
    assert_eq!(value, Value::Bool(true));
}

#[test]
fn test_regex_match_no_match() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("hello".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("xyz".to_string()))) };

    let stack = unsafe { patch_seq_regex_match(stack) };
    let (_, value) = unsafe { pop(stack) };
    assert_eq!(value, Value::Bool(false));
}

#[test]
fn test_regex_find() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("a1 b2 c3".to_string()))) };
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("[a-z][0-9]".to_string())),
        )
    };

    let stack = unsafe { patch_seq_regex_find(stack) };
    let (stack, success) = unsafe { pop(stack) };
    let (_, matched) = unsafe { pop(stack) };

    assert_eq!(success, Value::Bool(true));
    if let Value::String(s) = matched {
        assert_eq!(s.as_str(), "a1");
    } else {
        panic!("expected String");
    }
}

#[test]
fn test_regex_find_all() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("a1 b2 c3".to_string()))) };
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("[a-z][0-9]".to_string())),
        )
    };

    let stack = unsafe { patch_seq_regex_find_all(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));
    let (_, list_val) = unsafe { pop(stack) };

    if let Value::Variant(v) = list_val {
        assert_eq!(v.fields.len(), 3);
        if let Value::String(s) = &v.fields[0] {
            assert_eq!(s.as_str(), "a1");
        }
        if let Value::String(s) = &v.fields[1] {
            assert_eq!(s.as_str(), "b2");
        }
        if let Value::String(s) = &v.fields[2] {
            assert_eq!(s.as_str(), "c3");
        }
    } else {
        panic!("expected Variant (List)");
    }
}

#[test]
fn test_regex_replace() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };
    let stack = unsafe { push(stack, Value::String(global_string("world".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("Seq".to_string()))) };

    let stack = unsafe { patch_seq_regex_replace(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));
    let (_, result) = unsafe { pop(stack) };

    if let Value::String(s) = result {
        assert_eq!(s.as_str(), "hello Seq");
    } else {
        panic!("expected String");
    }
}

#[test]
fn test_regex_replace_all() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("a1 b2 c3".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("[0-9]".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("X".to_string()))) };

    let stack = unsafe { patch_seq_regex_replace_all(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));
    let (_, result) = unsafe { pop(stack) };

    if let Value::String(s) = result {
        assert_eq!(s.as_str(), "aX bX cX");
    } else {
        panic!("expected String");
    }
}

#[test]
fn test_regex_captures() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("2024-01-15".to_string())),
        )
    };
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string(r"(\d+)-(\d+)-(\d+)".to_string())),
        )
    };

    let stack = unsafe { patch_seq_regex_captures(stack) };
    let (stack, success) = unsafe { pop(stack) };
    let (_, groups) = unsafe { pop(stack) };

    assert_eq!(success, Value::Bool(true));
    if let Value::Variant(v) = groups {
        assert_eq!(v.fields.len(), 3);
        if let Value::String(s) = &v.fields[0] {
            assert_eq!(s.as_str(), "2024");
        }
        if let Value::String(s) = &v.fields[1] {
            assert_eq!(s.as_str(), "01");
        }
        if let Value::String(s) = &v.fields[2] {
            assert_eq!(s.as_str(), "15");
        }
    } else {
        panic!("expected Variant (List)");
    }
}

#[test]
fn test_regex_split() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("a1b2c3".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("[0-9]".to_string()))) };

    let stack = unsafe { patch_seq_regex_split(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));
    let (_, result) = unsafe { pop(stack) };

    if let Value::Variant(v) = result {
        assert_eq!(v.fields.len(), 4); // "a", "b", "c", ""
        if let Value::String(s) = &v.fields[0] {
            assert_eq!(s.as_str(), "a");
        }
        if let Value::String(s) = &v.fields[1] {
            assert_eq!(s.as_str(), "b");
        }
        if let Value::String(s) = &v.fields[2] {
            assert_eq!(s.as_str(), "c");
        }
    } else {
        panic!("expected Variant (List)");
    }
}

#[test]
fn test_regex_valid() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("[a-z]+".to_string()))) };

    let stack = unsafe { patch_seq_regex_valid(stack) };
    let (_, result) = unsafe { pop(stack) };
    assert_eq!(result, Value::Bool(true));

    // Invalid regex
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("[invalid".to_string()))) };

    let stack = unsafe { patch_seq_regex_valid(stack) };
    let (_, result) = unsafe { pop(stack) };
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_invalid_regex_graceful() {
    // Invalid regex should return false, not panic
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string("test".to_string()))) };
    let stack = unsafe { push(stack, Value::String(global_string("[invalid".to_string()))) };

    let stack = unsafe { patch_seq_regex_match(stack) };
    let (_, result) = unsafe { pop(stack) };
    assert_eq!(result, Value::Bool(false));
}
