use super::*;
use crate::seqstring::global_string;

// Helper quotation: double an integer
unsafe extern "C" fn double_quot(stack: Stack) -> Stack {
    unsafe {
        let (stack, val) = pop(stack);
        match val {
            Value::Int(n) => push(stack, Value::Int(n * 2)),
            _ => panic!("Expected Int"),
        }
    }
}

// Helper quotation: check if positive
unsafe extern "C" fn is_positive_quot(stack: Stack) -> Stack {
    unsafe {
        let (stack, val) = pop(stack);
        match val {
            Value::Int(n) => push(stack, Value::Bool(n > 0)),
            _ => panic!("Expected Int"),
        }
    }
}

// Helper quotation: add two integers
unsafe extern "C" fn add_quot(stack: Stack) -> Stack {
    unsafe {
        let (stack, b) = pop(stack);
        let (stack, a) = pop(stack);
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => push(stack, Value::Int(x + y)),
            _ => panic!("Expected two Ints"),
        }
    }
}

#[test]
fn test_list_map_double() {
    unsafe {
        // Create list [1, 2, 3]
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let fn_ptr = double_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_map(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::Int(2));
                assert_eq!(v.fields[1], Value::Int(4));
                assert_eq!(v.fields[2], Value::Int(6));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_filter_positive() {
    unsafe {
        // Create list [-1, 2, -3, 4, 0]
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![
                Value::Int(-1),
                Value::Int(2),
                Value::Int(-3),
                Value::Int(4),
                Value::Int(0),
            ],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let fn_ptr = is_positive_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_filter(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 2);
                assert_eq!(v.fields[0], Value::Int(2));
                assert_eq!(v.fields[1], Value::Int(4));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_fold_sum() {
    unsafe {
        // Create list [1, 2, 3, 4, 5]
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = push(stack, Value::Int(0)); // initial accumulator
        let fn_ptr = add_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_fold(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(15)); // 1+2+3+4+5 = 15
    }
}

#[test]
fn test_list_fold_empty() {
    unsafe {
        // Create empty list
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = push(stack, Value::Int(42)); // initial accumulator
        let fn_ptr = add_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_fold(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(42)); // Should return initial value
    }
}

#[test]
fn test_list_length() {
    unsafe {
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = list_length(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(3));
    }
}

#[test]
fn test_list_empty_true() {
    unsafe {
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = list_empty(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_list_empty_false() {
    unsafe {
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = list_empty(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_list_map_empty() {
    unsafe {
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let fn_ptr = double_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_map(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 0);
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_map_preserves_tag() {
    unsafe {
        // Create list with custom tag
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("CustomTag".to_string()),
            vec![Value::Int(1), Value::Int(2)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let fn_ptr = double_quot as *const () as usize;
        let stack = push(
            stack,
            Value::Quotation {
                wrapper: fn_ptr,
                impl_: fn_ptr,
            },
        );
        let stack = list_map(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str(), "CustomTag"); // Tag preserved
                assert_eq!(v.fields[0], Value::Int(2));
                assert_eq!(v.fields[1], Value::Int(4));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

// Helper closure function: adds captured value to element
// Closure receives: stack with element, env with [captured_value]
unsafe extern "C" fn add_captured_closure(
    stack: Stack,
    env: *const Value,
    _env_len: usize,
) -> Stack {
    unsafe {
        let (stack, val) = pop(stack);
        let captured = &*env; // First (and only) captured value
        match (val, captured) {
            (Value::Int(n), Value::Int(c)) => push(stack, Value::Int(n + c)),
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_list_map_with_closure() {
    unsafe {
        // Create list [1, 2, 3]
        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        )));

        // Create closure that adds 10 to each element
        let env: std::sync::Arc<[Value]> =
            std::sync::Arc::from(vec![Value::Int(10)].into_boxed_slice());
        let closure = Value::Closure {
            fn_ptr: add_captured_closure as *const () as usize,
            env,
        };

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = push(stack, closure);
        let stack = list_map(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::Int(11)); // 1 + 10
                assert_eq!(v.fields[1], Value::Int(12)); // 2 + 10
                assert_eq!(v.fields[2], Value::Int(13)); // 3 + 10
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_get_type_error_index() {
    unsafe {
        crate::error::clear_runtime_error();

        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = push(stack, Value::Bool(true)); // Wrong type - should be Int
        let stack = list_get(stack);

        // Should have set an error
        assert!(crate::error::has_runtime_error());
        let error = crate::error::take_runtime_error().unwrap();
        assert!(error.contains("expected Int"));

        // Should return (0, false)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::Int(0));
    }
}

#[test]
fn test_list_get_type_error_list() {
    unsafe {
        crate::error::clear_runtime_error();

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // Wrong type - should be Variant
        let stack = push(stack, Value::Int(0)); // index
        let stack = list_get(stack);

        // Should have set an error
        assert!(crate::error::has_runtime_error());
        let error = crate::error::take_runtime_error().unwrap();
        assert!(error.contains("expected Variant"));

        // Should return (0, false)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, value) = pop(stack);
        assert_eq!(value, Value::Int(0));
    }
}

#[test]
fn test_list_set_type_error_index() {
    unsafe {
        crate::error::clear_runtime_error();

        let list = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, list);
        let stack = push(stack, Value::Bool(true)); // Wrong type - should be Int
        let stack = push(stack, Value::Int(99)); // new value
        let stack = list_set(stack);

        // Should have set an error
        assert!(crate::error::has_runtime_error());
        let error = crate::error::take_runtime_error().unwrap();
        assert!(error.contains("expected Int"));

        // Should return (list, false)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, returned_list) = pop(stack);
        assert!(matches!(returned_list, Value::Variant(_)));
    }
}

#[test]
fn test_list_set_type_error_list() {
    unsafe {
        crate::error::clear_runtime_error();

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // Wrong type - should be Variant
        let stack = push(stack, Value::Int(0)); // index
        let stack = push(stack, Value::Int(99)); // new value
        let stack = list_set(stack);

        // Should have set an error
        assert!(crate::error::has_runtime_error());
        let error = crate::error::take_runtime_error().unwrap();
        assert!(error.contains("expected Variant"));

        // Should return (original value, false)
        let (stack, success) = pop(stack);
        assert_eq!(success, Value::Bool(false));
        let (_stack, returned) = pop(stack);
        assert_eq!(returned, Value::Int(42)); // Original value returned
    }
}

// =========================================================================
// list.reverse tests
// =========================================================================

#[test]
fn test_list_reverse_empty() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        // Push empty list
        let empty_list = Value::Variant(Arc::new(VariantData::new(
            crate::seqstring::global_string("List".to_string()),
            vec![],
        )));
        let stack = push(stack, empty_list);
        let stack = list_reverse(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 0);
                assert_eq!(v.tag.as_str(), "List");
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_reverse_single() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            crate::seqstring::global_string("List".to_string()),
            vec![Value::Int(42)],
        )));
        let stack = push(stack, list);
        let stack = list_reverse(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 1);
                assert_eq!(v.fields[0], Value::Int(42));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_list_reverse_multiple() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let list = Value::Variant(Arc::new(VariantData::new(
            crate::seqstring::global_string("List".to_string()),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        )));
        let stack = push(stack, list);
        let stack = list_reverse(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::Int(3));
                assert_eq!(v.fields[1], Value::Int(2));
                assert_eq!(v.fields[2], Value::Int(1));
                assert_eq!(v.tag.as_str(), "List"); // tag preserved
            }
            _ => panic!("Expected Variant"),
        }
    }
}
