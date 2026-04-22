use super::*;

#[test]
fn test_push_float() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_float(stack, 3.5);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(3.5));
    }
}

#[test]
fn test_f_add() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(1.5));
        let stack = push(stack, Value::Float(2.5));

        let stack = f_add(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(4.0));
    }
}

#[test]
fn test_f_subtract() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(5.0));
        let stack = push(stack, Value::Float(2.0));

        let stack = f_subtract(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(3.0));
    }
}

#[test]
fn test_f_multiply() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(3.0));
        let stack = push(stack, Value::Float(4.0));

        let stack = f_multiply(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(12.0));
    }
}

#[test]
fn test_f_divide() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(10.0));
        let stack = push(stack, Value::Float(4.0));

        let stack = f_divide(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(2.5));
    }
}

#[test]
fn test_f_divide_by_zero() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(1.0));
        let stack = push(stack, Value::Float(0.0));

        let stack = f_divide(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Float(f) => assert!(f.is_infinite()),
            _ => panic!("Expected Float"),
        }
    }
}

#[test]
fn test_f_eq_true() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(3.5));
        let stack = push(stack, Value::Float(3.5));

        let stack = f_eq(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true));
    }
}

#[test]
fn test_f_eq_false() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(3.5));
        let stack = push(stack, Value::Float(2.5));

        let stack = f_eq(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(false));
    }
}

#[test]
fn test_f_lt() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(1.5));
        let stack = push(stack, Value::Float(2.5));

        let stack = f_lt(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true)); // 1.5 < 2.5
    }
}

#[test]
fn test_f_gt() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(2.5));
        let stack = push(stack, Value::Float(1.5));

        let stack = f_gt(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true)); // 2.5 > 1.5
    }
}

#[test]
fn test_f_lte() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(2.5));
        let stack = push(stack, Value::Float(2.5));

        let stack = f_lte(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true)); // 2.5 <= 2.5
    }
}

#[test]
fn test_f_gte() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(2.5));
        let stack = push(stack, Value::Float(2.5));

        let stack = f_gte(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true)); // 2.5 >= 2.5
    }
}

#[test]
fn test_f_neq() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(1.0));
        let stack = push(stack, Value::Float(2.0));

        let stack = f_neq(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Bool(true)); // 1.0 <> 2.0
    }
}

#[test]
fn test_int_to_float() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(42));

        let stack = int_to_float(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Float(42.0));
    }
}

#[test]
fn test_float_to_int() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(3.7));

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(3)); // Truncates toward zero
    }
}

#[test]
fn test_float_to_int_negative() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(-3.7));

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(-3)); // Truncates toward zero
    }
}

#[test]
fn test_float_to_int_nan() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(f64::NAN));

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(0)); // NaN becomes 0
    }
}

#[test]
fn test_float_to_int_clamps_to_63bit_max() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(1e20)); // Much larger than 63-bit max

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        let int63_max = (1i64 << 62) - 1;
        assert_eq!(result, Value::Int(int63_max));
    }
}

#[test]
fn test_float_to_int_clamps_to_63bit_min() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(-1e20)); // Much smaller than 63-bit min

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        let int63_min = -(1i64 << 62);
        assert_eq!(result, Value::Int(int63_min));
    }
}

#[test]
fn test_float_to_int_infinity_clamps() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(f64::INFINITY));

        let stack = float_to_int(stack);

        let (_stack, result) = pop(stack);
        let int63_max = (1i64 << 62) - 1;
        assert_eq!(result, Value::Int(int63_max));
    }
}

#[test]
fn test_float_to_string() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(3.5));

        let stack = float_to_string(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "3.5"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_float_to_string_whole_number() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(42.0));

        let stack = float_to_string(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::String(s) => assert_eq!(s.as_str(), "42"),
            _ => panic!("Expected String"),
        }
    }
}

#[test]
fn test_nan_propagation() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(f64::NAN));
        let stack = push(stack, Value::Float(1.0));

        let stack = f_add(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Float(f) => assert!(f.is_nan()),
            _ => panic!("Expected Float"),
        }
    }
}

#[test]
fn test_infinity() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Float(f64::INFINITY));
        let stack = push(stack, Value::Float(1.0));

        let stack = f_add(stack);

        let (_stack, result) = pop(stack);
        match result {
            Value::Float(f) => assert!(f.is_infinite()),
            _ => panic!("Expected Float"),
        }
    }
}
