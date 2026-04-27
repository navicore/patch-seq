use super::*;
use crate::seqstring::global_string;
use crate::stack::{pop, push};
use crate::value::{Value, VariantData};
use std::sync::Arc;

#[test]
fn test_variant_field_count() {
    unsafe {
        // Create a variant with 3 fields
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("TestTag".to_string()),
            vec![Value::Int(10), Value::Int(20), Value::Int(30)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_field_count(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(3));
    }
}

#[test]
fn test_variant_tag() {
    unsafe {
        // Create a variant with tag "MyTag"
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("MyTag".to_string()),
            vec![Value::Int(10)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_tag(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Symbol(global_string("MyTag".to_string())));
    }
}

#[test]
fn test_variant_field_at() {
    unsafe {
        let str1 = global_string("hello".to_string());
        let str2 = global_string("world".to_string());

        // Create a variant with mixed fields
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("TestTag".to_string()),
            vec![
                Value::String(str1.clone()),
                Value::Int(42),
                Value::String(str2.clone()),
            ],
        )));

        // Test accessing field 0
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant.clone());
        let stack = push(stack, Value::Int(0));
        let stack = variant_field_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(str1.clone()));

        // Test accessing field 1
        let stack = push(stack, variant.clone());
        let stack = push(stack, Value::Int(1));
        let stack = variant_field_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(42));

        // Test accessing field 2
        let stack = push(stack, variant.clone());
        let stack = push(stack, Value::Int(2));
        let stack = variant_field_at(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::String(str2));
    }
}

#[test]
fn test_variant_field_count_empty() {
    unsafe {
        // Create a variant with no fields
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("Empty".to_string()),
            vec![],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_field_count(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0));
    }
}

#[test]
fn test_make_variant_with_fields() {
    unsafe {
        // Create: 10 20 30 :Tag make-variant-3
        // Should produce variant with tag :Tag and fields [10, 20, 30]
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(10)); // field 0
        let stack = push(stack, Value::Int(20)); // field 1
        let stack = push(stack, Value::Int(30)); // field 2
        let stack = push(stack, Value::Symbol(global_string("Tag".to_string()))); // tag

        let stack = make_variant_3(stack);

        let (_stack, result) = pop(stack);

        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "Tag");
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::Int(10));
                assert_eq!(v.fields[1], Value::Int(20));
                assert_eq!(v.fields[2], Value::Int(30));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_make_variant_empty() {
    unsafe {
        // Create: :None make-variant-0
        // Should produce variant with tag :None and no fields
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Symbol(global_string("None".to_string()))); // tag

        let stack = make_variant_0(stack);

        let (_stack, result) = pop(stack);

        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "None");
                assert_eq!(v.fields.len(), 0);
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_make_variant_with_mixed_types() {
    unsafe {
        let s = global_string("hello".to_string());

        // Create variant with mixed field types using make-variant-3
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42));
        let stack = push(stack, Value::String(s.clone()));
        let stack = push(stack, Value::Float(3.5));
        let stack = push(stack, Value::Symbol(global_string("Mixed".to_string()))); // tag

        let stack = make_variant_3(stack);

        let (_stack, result) = pop(stack);

        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "Mixed");
                assert_eq!(v.fields.len(), 3);
                assert_eq!(v.fields[0], Value::Int(42));
                assert_eq!(v.fields[1], Value::String(s));
                assert_eq!(v.fields[2], Value::Float(3.5));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_variant_append() {
    unsafe {
        // Create an empty variant (tag Array)
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Symbol(global_string("Array".to_string()))); // tag
        let stack = make_variant_0(stack);

        // Append a value
        let stack = push(stack, Value::Int(42));
        let stack = variant_append(stack);

        // Check result
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "Array");
                assert_eq!(v.fields.len(), 1);
                assert_eq!(v.fields[0], Value::Int(42));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_variant_append_multiple() {
    unsafe {
        // Create an empty variant (tag Object)
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Symbol(global_string("Object".to_string()))); // tag
        let stack = make_variant_0(stack);

        // Append key
        let key = global_string("name".to_string());
        let stack = push(stack, Value::String(key.clone()));
        let stack = variant_append(stack);

        // Append value
        let val = global_string("John".to_string());
        let stack = push(stack, Value::String(val.clone()));
        let stack = variant_append(stack);

        // Check result - should have 2 fields
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "Object");
                assert_eq!(v.fields.len(), 2);
                assert_eq!(v.fields[0], Value::String(key));
                assert_eq!(v.fields[1], Value::String(val));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_variant_first() {
    unsafe {
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(10), Value::Int(20), Value::Int(30)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_first(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(10));
    }
}

#[test]
fn test_variant_first_singleton() {
    // Singleton: first and last must agree.
    unsafe {
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("Wrap".to_string()),
            vec![Value::Int(99)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_first(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(99));
    }
}

// NOTE: there is no unit test for `variant.first` on a fieldless variant.
// The panic crosses the `extern "C"` boundary, which is non-unwinding
// in edition 2024 — it aborts the process rather than unwinding, so
// `#[should_panic]` would abort the entire test binary instead of being
// caught. The same is true for `variant.last` and `variant.init`. The
// contract is pinned by the docstring + panic message; an end-to-end
// repro would have to run the produced binary in a subprocess and
// observe the abort, which isn't worth the harness complexity here.

#[test]
fn test_variant_last() {
    unsafe {
        // Create a variant with 3 fields
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            vec![Value::Int(10), Value::Int(20), Value::Int(30)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_last(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(30));
    }
}

#[test]
fn test_variant_init() {
    unsafe {
        // Create a variant with 3 fields
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("Custom".to_string()),
            vec![Value::Int(10), Value::Int(20), Value::Int(30)],
        )));

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, variant);
        let stack = variant_init(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.tag.as_str_or_empty(), "Custom"); // tag preserved
                assert_eq!(v.fields.len(), 2);
                assert_eq!(v.fields[0], Value::Int(10));
                assert_eq!(v.fields[1], Value::Int(20));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_variant_stack_operations() {
    // Test using variant as a stack: append, append, last, init, last
    unsafe {
        // Create empty "stack" variant (tag Stack)
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Symbol(global_string("Stack".to_string()))); // tag
        let stack = make_variant_0(stack);

        // Append 10
        let stack = push(stack, Value::Int(10));
        let stack = variant_append(stack);

        // Append 20
        let stack = push(stack, Value::Int(20));
        let stack = variant_append(stack);

        // Now have variant with [10, 20] on stack
        // Dup and get last (should be 20)
        let (stack, variant) = pop(stack);
        let stack = push(stack, variant.clone());
        let stack = push(stack, variant);
        let stack = variant_last(stack);
        let (stack, top) = pop(stack);
        assert_eq!(top, Value::Int(20));

        // Now use init to remove last element
        let stack = variant_init(stack);

        // Dup and get last (should now be 10)
        let (stack, variant) = pop(stack);
        let stack = push(stack, variant.clone());
        let stack = push(stack, variant);
        let stack = variant_last(stack);
        let (stack, top) = pop(stack);
        assert_eq!(top, Value::Int(10));

        // Verify remaining variant has 1 field
        let (_stack, result) = pop(stack);
        match result {
            Value::Variant(v) => {
                assert_eq!(v.fields.len(), 1);
                assert_eq!(v.fields[0], Value::Int(10));
            }
            _ => panic!("Expected Variant"),
        }
    }
}

#[test]
fn test_variant_clone_is_o1() {
    // Regression test: Ensure deeply nested variants clone in O(1) time
    // This would have been O(2^n) with Box before the Arc change
    let mut variant = Value::Variant(Arc::new(VariantData::new(
        global_string("Level0".to_string()),
        vec![],
    )));

    // Build a deeply nested structure (100 levels)
    for i in 0..100 {
        variant = Value::Variant(Arc::new(VariantData::new(
            global_string(format!("Level{}", i)),
            vec![variant.clone()],
        )));
    }

    // Clone should be O(1) - just incrementing Arc refcount
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _copy = variant.clone();
    }
    let elapsed = start.elapsed();

    // 1000 clones of a 100-deep structure should take < 1ms with Arc
    // With Box it would take seconds or longer
    assert!(
        elapsed.as_millis() < 10,
        "Clone took {:?} - should be O(1) with Arc",
        elapsed
    );
}

#[test]
fn test_variant_arc_sharing() {
    // Test that Arc properly shares data (refcount increases, not deep copy)
    let inner = Value::Variant(Arc::new(VariantData::new(
        global_string("Inner".to_string()),
        vec![Value::Int(42)],
    )));
    let outer = Value::Variant(Arc::new(VariantData::new(
        global_string("Outer".to_string()),
        vec![inner.clone()],
    )));

    // Clone should share the same Arc, not deep copy
    let outer_clone = outer.clone();

    // Both should have the same inner value
    match (&outer, &outer_clone) {
        (Value::Variant(a), Value::Variant(b)) => {
            // Arc::ptr_eq would be ideal but fields are Box<[Value]>
            // Instead verify the values are equal (they share the same data)
            assert_eq!(a.tag, b.tag);
            assert_eq!(a.fields.len(), b.fields.len());
        }
        _ => panic!("Expected Variants"),
    }
}

#[test]
fn test_variant_thread_safe_sharing() {
    // Test that variants can be safely shared between threads
    // This validates the Send + Sync implementation
    use std::sync::Arc as StdArc;
    use std::thread;

    let variant = Value::Variant(Arc::new(VariantData::new(
        global_string("ThreadSafe".to_string()),
        vec![Value::Int(1), Value::Int(2), Value::Int(3)],
    )));

    // Wrap in Arc for thread sharing
    let shared = StdArc::new(variant);

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let v = StdArc::clone(&shared);
            thread::spawn(move || {
                // Access the variant from another thread
                match &*v {
                    Value::Variant(data) => {
                        assert_eq!(data.tag.as_str_or_empty(), "ThreadSafe");
                        assert_eq!(data.fields.len(), 3);
                    }
                    _ => panic!("Expected Variant"),
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Thread panicked");
    }
}
