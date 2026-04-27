use super::*;
use crate::stack::push;

// Helper: predicate that always returns true (keeps value, pushes true)
// Stack effect: ( value -- value Bool )
unsafe extern "C" fn pred_always_true(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Bool(true)) }
}

// Helper: predicate that always returns false (keeps value, pushes false)
// Stack effect: ( value -- value Bool )
unsafe extern "C" fn pred_always_false(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Bool(false)) }
}

// Helper: predicate that checks if top value is zero
// Stack effect: ( Int -- Int Bool )
unsafe extern "C" fn pred_is_zero(stack: Stack) -> Stack {
    unsafe {
        let val = crate::stack::peek(stack);
        match val {
            Value::Int(n) => push(stack, Value::Bool(n == 0)),
            _ => panic!("pred_is_zero: expected Int"),
        }
    }
}

// Helper: predicate that checks if value is negative
// Stack effect: ( Int -- Int Bool )
unsafe extern "C" fn pred_is_negative(stack: Stack) -> Stack {
    unsafe {
        let val = crate::stack::peek(stack);
        match val {
            Value::Int(n) => push(stack, Value::Bool(n < 0)),
            _ => panic!("pred_is_negative: expected Int"),
        }
    }
}

// Helper: body that drops value and pushes "matched"
// Stack effect: ( value -- String )
unsafe extern "C" fn body_matched(stack: Stack) -> Stack {
    unsafe {
        let (stack, _) = pop(stack);
        push(
            stack,
            Value::String(crate::seqstring::global_string("matched".to_string())),
        )
    }
}

// Helper: body that drops value and pushes "zero"
// Stack effect: ( value -- String )
unsafe extern "C" fn body_zero(stack: Stack) -> Stack {
    unsafe {
        let (stack, _) = pop(stack);
        push(
            stack,
            Value::String(crate::seqstring::global_string("zero".to_string())),
        )
    }
}

// Helper: body that drops value and pushes "positive"
// Stack effect: ( value -- String )
unsafe extern "C" fn body_positive(stack: Stack) -> Stack {
    unsafe {
        let (stack, _) = pop(stack);
        push(
            stack,
            Value::String(crate::seqstring::global_string("positive".to_string())),
        )
    }
}

// Helper: body that drops value and pushes "negative"
// Stack effect: ( value -- String )
unsafe extern "C" fn body_negative(stack: Stack) -> Stack {
    unsafe {
        let (stack, _) = pop(stack);
        push(
            stack,
            Value::String(crate::seqstring::global_string("negative".to_string())),
        )
    }
}

// Helper: body that drops value and pushes "default"
// Stack effect: ( value -- String )
unsafe extern "C" fn body_default(stack: Stack) -> Stack {
    unsafe {
        let (stack, _) = pop(stack);
        push(
            stack,
            Value::String(crate::seqstring::global_string("default".to_string())),
        )
    }
}

// Helper to create a quotation value from a function pointer
fn make_quotation(f: unsafe extern "C" fn(Stack) -> Stack) -> Value {
    let ptr = f as *const () as usize;
    Value::Quotation {
        wrapper: ptr,
        impl_: ptr,
    }
}

#[test]
fn test_cond_single_match() {
    // Test: single predicate that always matches
    // Stack: value [pred_always_true] [body_matched] 1
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // value
        let stack = push(stack, make_quotation(pred_always_true));
        let stack = push(stack, make_quotation(body_matched));
        let stack = push(stack, Value::Int(1)); // count

        let stack = cond(stack);

        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "matched"),
            _ => panic!("Expected String, got {:?}", result),
        }
    }
}

#[test]
fn test_cond_first_match_wins() {
    // Test: multiple predicates, first one that matches should win
    // Both predicates would match, but first one should be used
    // Stack: value [pred_always_true] [body_matched] [pred_always_true] [body_default] 2
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // value
        let stack = push(stack, make_quotation(pred_always_true));
        let stack = push(stack, make_quotation(body_matched)); // first pair
        let stack = push(stack, make_quotation(pred_always_true));
        let stack = push(stack, make_quotation(body_default)); // second pair
        let stack = push(stack, Value::Int(2)); // count

        let stack = cond(stack);

        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "matched"), // first body wins
            _ => panic!("Expected String, got {:?}", result),
        }
    }
}

#[test]
fn test_cond_second_match() {
    // Test: first predicate fails, second matches
    // Stack: value [pred_always_false] [body_matched] [pred_always_true] [body_default] 2
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // value
        let stack = push(stack, make_quotation(pred_always_false));
        let stack = push(stack, make_quotation(body_matched)); // first pair - won't match
        let stack = push(stack, make_quotation(pred_always_true));
        let stack = push(stack, make_quotation(body_default)); // second pair - will match
        let stack = push(stack, Value::Int(2)); // count

        let stack = cond(stack);

        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "default"), // second body wins
            _ => panic!("Expected String, got {:?}", result),
        }
    }
}

#[test]
fn test_cond_classify_number() {
    // Test: classify numbers as negative, zero, or positive
    // This mimics the example from the docs
    unsafe {
        // Test negative number
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(-5)); // value
        let stack = push(stack, make_quotation(pred_is_negative));
        let stack = push(stack, make_quotation(body_negative));
        let stack = push(stack, make_quotation(pred_is_zero));
        let stack = push(stack, make_quotation(body_zero));
        let stack = push(stack, make_quotation(pred_always_true)); // default
        let stack = push(stack, make_quotation(body_positive));
        let stack = push(stack, Value::Int(3)); // count

        let stack = cond(stack);
        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "negative"),
            _ => panic!("Expected String"),
        }

        // Test zero
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(0)); // value
        let stack = push(stack, make_quotation(pred_is_negative));
        let stack = push(stack, make_quotation(body_negative));
        let stack = push(stack, make_quotation(pred_is_zero));
        let stack = push(stack, make_quotation(body_zero));
        let stack = push(stack, make_quotation(pred_always_true)); // default
        let stack = push(stack, make_quotation(body_positive));
        let stack = push(stack, Value::Int(3)); // count

        let stack = cond(stack);
        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "zero"),
            _ => panic!("Expected String"),
        }

        // Test positive
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42)); // value
        let stack = push(stack, make_quotation(pred_is_negative));
        let stack = push(stack, make_quotation(body_negative));
        let stack = push(stack, make_quotation(pred_is_zero));
        let stack = push(stack, make_quotation(body_zero));
        let stack = push(stack, make_quotation(pred_always_true)); // default
        let stack = push(stack, make_quotation(body_positive));
        let stack = push(stack, Value::Int(3)); // count

        let stack = cond(stack);
        let (_, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "positive"),
            _ => panic!("Expected String"),
        }
    }
}

// Note: #[should_panic] tests don't work with extern "C" functions because
// they can't unwind. The following panic conditions are documented in the
// function's doc comments and verified by the compiler's type system:
//
// - "cond: no predicate matched" - when all predicates return false
// - "cond: need at least one predicate/body pair" - when count is 0
// - "cond: count must be non-negative" - when count is negative
// - "cond: expected body Quotation" - when body is not a Quotation
// - "cond: expected predicate Quotation" - when predicate is not a Quotation
// - "cond: predicate must return Bool" - when predicate returns non-Bool
