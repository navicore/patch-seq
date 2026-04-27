use super::*;

#[test]
fn test_make_map() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Map(m) => assert!(m.is_empty()),
            _ => panic!("Expected Map"),
        }
    }
}

#[test]
fn test_map_set_and_get() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::String("name".into()));
        let stack = push(stack, Value::String("Alice".into()));
        let stack = map_set(stack);

        // Get the value back
        let stack = push(stack, Value::String("name".into()));
        let stack = map_get(stack);

        // map_get returns (value Bool)
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "Alice"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_map_set_with_int_key() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::Int(42));
        let stack = push(stack, Value::String("answer".into()));
        let stack = map_set(stack);

        let stack = push(stack, Value::Int(42));
        let stack = map_get(stack);

        // map_get returns (value Bool)
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "answer"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_map_has() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::String("key".into()));
        let stack = push(stack, Value::Int(100));
        let stack = map_set(stack);

        // Check existing key (dup map first since map_has consumes it)
        let stack = crate::stack::dup(stack);
        let stack = push(stack, Value::String("key".into()));
        let stack = map_has(stack);
        let (stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));

        // Check non-existing key (map is still on stack)
        let stack = push(stack, Value::String("missing".into()));
        let stack = map_has(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_map_remove() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::String("a".into()));
        let stack = push(stack, Value::Int(1));
        let stack = map_set(stack);
        let stack = push(stack, Value::String("b".into()));
        let stack = push(stack, Value::Int(2));
        let stack = map_set(stack);

        // Remove "a"
        let stack = push(stack, Value::String("a".into()));
        let stack = map_remove(stack);

        // Check "a" is gone (dup map first since map_has consumes it)
        let stack = crate::stack::dup(stack);
        let stack = push(stack, Value::String("a".into()));
        let stack = map_has(stack);
        let (stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));

        // Check "b" is still there (map is still on stack)
        let stack = push(stack, Value::String("b".into()));
        let stack = map_has(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_map_size() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        // Empty map
        let stack = map_size(stack);
        let (stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));

        // Add entries
        let stack = make_map(stack);
        let stack = push(stack, Value::String("a".into()));
        let stack = push(stack, Value::Int(1));
        let stack = map_set(stack);
        let stack = push(stack, Value::String("b".into()));
        let stack = push(stack, Value::Int(2));
        let stack = map_set(stack);

        let stack = map_size(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(2));
    }
}

#[test]
fn test_map_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        let stack = map_empty(stack);
        let (stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));

        // Non-empty
        let stack = make_map(stack);
        let stack = push(stack, Value::String("key".into()));
        let stack = push(stack, Value::Int(1));
        let stack = map_set(stack);

        let stack = map_empty(stack);
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_map_keys_and_values() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::String("x".into()));
        let stack = push(stack, Value::Int(10));
        let stack = map_set(stack);
        let stack = push(stack, Value::String("y".into()));
        let stack = push(stack, Value::Int(20));
        let stack = map_set(stack);

        // Get keys
        let stack = crate::stack::dup(stack); // Keep map for values test
        let stack = map_keys(stack);
        let (stack, keys_result) = pop(stack);
        match keys_result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 2);
                // Keys are "x" and "y" but order is not guaranteed
            }
            _ => panic!("Expected Variant"),
        }

        // Get values
        let stack = map_values(stack);
        let (_stack, values_result) = pop(stack);
        match values_result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 2);
                // Values are 10 and 20 but order is not guaranteed
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_map_get_found() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::String("key".into()));
        let stack = push(stack, Value::Int(42));
        let stack = map_set(stack);

        let stack = push(stack, Value::String("key".into()));
        let stack = map_get(stack);

        let (stack, flag) = pop(stack);
        let (_stack, value) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        assert_eq!(value, Value::Int(42));
    }
}

#[test]
fn test_map_get_not_found() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        let stack = push(stack, Value::String("missing".into()));
        let stack = map_get(stack);

        let (stack, flag) = pop(stack);
        let (_stack, _value) = pop(stack); // placeholder
        assert_eq!(flag, Value::Bool(false));
    }
}

#[test]
fn test_map_with_bool_key() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);
        let stack = push(stack, Value::Bool(true));
        let stack = push(stack, Value::String("yes".into()));
        let stack = map_set(stack);
        let stack = push(stack, Value::Bool(false));
        let stack = push(stack, Value::String("no".into()));
        let stack = map_set(stack);

        let stack = push(stack, Value::Bool(true));
        let stack = map_get(stack);
        // map_get returns (value Bool)
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "yes"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_map_key_overwrite() {
    // Test that map-set with existing key overwrites the value
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        // Set initial value
        let stack = push(stack, Value::String("key".into()));
        let stack = push(stack, Value::Int(100));
        let stack = map_set(stack);

        // Overwrite with new value
        let stack = push(stack, Value::String("key".into()));
        let stack = push(stack, Value::Int(200));
        let stack = map_set(stack);

        // Verify size is still 1 (not 2)
        let stack = crate::stack::dup(stack);
        let stack = map_size(stack);
        let (stack, size) = pop(stack);
        assert_eq!(size, Value::Int(1));

        // Verify value was updated
        let stack = push(stack, Value::String("key".into()));
        let stack = map_get(stack);
        // map_get returns (value Bool)
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(200));
    }
}

#[test]
fn test_map_mixed_key_types() {
    // Test that a single map can have different key types
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = make_map(stack);

        // Add string key
        let stack = push(stack, Value::String("name".into()));
        let stack = push(stack, Value::String("Alice".into()));
        let stack = map_set(stack);

        // Add integer key
        let stack = push(stack, Value::Int(42));
        let stack = push(stack, Value::String("answer".into()));
        let stack = map_set(stack);

        // Add boolean key
        let stack = push(stack, Value::Bool(true));
        let stack = push(stack, Value::String("yes".into()));
        let stack = map_set(stack);

        // Verify size is 3
        let stack = crate::stack::dup(stack);
        let stack = map_size(stack);
        let (stack, size) = pop(stack);
        assert_eq!(size, Value::Int(3));

        // Verify each key retrieves correct value
        // map_get returns (value Bool)
        let stack = crate::stack::dup(stack);
        let stack = push(stack, Value::String("name".into()));
        let stack = map_get(stack);
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "Alice"),
            _ => panic!("Expected String for name key"),
        }

        let stack = crate::stack::dup(stack);
        let stack = push(stack, Value::Int(42));
        let stack = map_get(stack);
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "answer"),
            _ => panic!("Expected String for int key"),
        }

        let stack = push(stack, Value::Bool(true));
        let stack = map_get(stack);
        let (stack, flag) = pop(stack);
        assert_eq!(flag, Value::Bool(true));
        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "yes"),
            _ => panic!("Expected String for bool key"),
        }
    }
}

// =========================================================================
// map.fold tests
// =========================================================================

#[test]
fn test_map_fold_empty() {
    // Folding an empty map should return the initial accumulator
    unsafe {
        use crate::quotations::push_quotation;

        let stack = crate::stack::alloc_test_stack();

        // Push empty map
        let stack = make_map(stack);

        // Push initial accumulator
        let stack = push(stack, Value::Int(99));

        // Push a dummy quotation (won't be called for empty map)
        unsafe extern "C" fn noop(stack: Stack) -> Stack {
            stack
        }
        let fn_ptr = noop as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        let stack = patch_seq_map_fold(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(99));
    }
}
