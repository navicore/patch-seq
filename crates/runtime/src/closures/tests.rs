use super::*;

#[test]
fn test_create_env() {
    let env = create_env(3);
    assert!(!env.is_null());

    // Clean up
    unsafe {
        let _ = Box::from_raw(env);
    }
}

#[test]
fn test_env_set_and_get() {
    let env = create_env(3);

    // Set values
    unsafe {
        env_set(env, 0, Value::Int(42));
        env_set(env, 1, Value::Bool(true));
        env_set(env, 2, Value::Int(99));
    }

    // Get values (convert to data pointer + length)
    unsafe {
        let env_slice = &*env;
        let env_data = env_slice.as_ptr();
        let env_len = env_slice.len();
        assert_eq!(env_get(env_data, env_len, 0), Value::Int(42));
        assert_eq!(env_get(env_data, env_len, 1), Value::Bool(true));
        assert_eq!(env_get(env_data, env_len, 2), Value::Int(99));
    }

    // Clean up
    unsafe {
        let _ = Box::from_raw(env);
    }
}

#[test]
fn test_make_closure() {
    let env = create_env(2);

    unsafe {
        env_set(env, 0, Value::Int(5));
        env_set(env, 1, Value::Int(10));

        let closure = make_closure(0x1234, env);

        match closure {
            Value::Closure { fn_ptr, env } => {
                assert_eq!(fn_ptr, 0x1234);
                assert_eq!(env.len(), 2);
                assert_eq!(env[0], Value::Int(5));
                assert_eq!(env[1], Value::Int(10));
            }
            _ => panic!("Expected Closure value"),
        }
    }
}

// Note: We don't test panic behavior for FFI functions as they use
// extern "C" which cannot unwind. The functions will still panic at runtime
// if called incorrectly, but we can't test that behavior with #[should_panic].

#[test]
fn test_push_closure() {
    use crate::stack::{pop, push};
    use crate::value::Value;

    // Create a stack with some values
    let mut stack = crate::stack::alloc_test_stack();
    stack = unsafe { push(stack, Value::Int(10)) };
    stack = unsafe { push(stack, Value::Int(5)) };

    // Create a closure that captures both values
    let fn_ptr = 0x1234;
    stack = unsafe { push_closure(stack, fn_ptr, 2) };

    // Pop the closure
    let (_stack, closure_value) = unsafe { pop(stack) };

    // Verify it's a closure with correct captures.
    // Env is stored bottom-to-top: env[0] is the caller's deepest capture
    // (pushed first — Int(10)), env[N-1] is the shallowest (Int(5)).
    // This matches the typechecker's capture-type ordering and preserves
    // the caller's visual stack order inside the closure body.
    match closure_value {
        Value::Closure { fn_ptr: fp, env } => {
            assert_eq!(fp, fn_ptr as usize);
            assert_eq!(env.len(), 2);
            assert_eq!(env[0], Value::Int(10)); // deepest caller capture
            assert_eq!(env[1], Value::Int(5)); // shallowest (was on top)
        }
        _ => panic!("Expected Closure value, got {:?}", closure_value),
    }

    // Stack should be empty now
}

#[test]
fn test_push_closure_zero_captures() {
    use crate::stack::pop;
    use crate::value::Value;

    // Create empty stack
    let stack = crate::stack::alloc_test_stack();

    // Create a closure with no captures
    let fn_ptr = 0x5678;
    let stack = unsafe { push_closure(stack, fn_ptr, 0) };

    // Pop the closure
    let (_stack, closure_value) = unsafe { pop(stack) };

    // Verify it's a closure with no captures
    match closure_value {
        Value::Closure { fn_ptr: fp, env } => {
            assert_eq!(fp, fn_ptr as usize);
            assert_eq!(env.len(), 0);
        }
        _ => panic!("Expected Closure value, got {:?}", closure_value),
    }

    // Stack should be empty
}

#[test]
fn test_env_get_bool() {
    let env = create_env(2);

    unsafe {
        env_set(env, 0, Value::Bool(true));
        env_set(env, 1, Value::Bool(false));

        let env_slice = &*env;
        let env_data = env_slice.as_ptr();
        let env_len = env_slice.len();

        assert_eq!(env_get_bool(env_data, env_len, 0), 1);
        assert_eq!(env_get_bool(env_data, env_len, 1), 0);

        let _ = Box::from_raw(env);
    }
}

#[test]
fn test_env_get_float() {
    let env = create_env(2);

    unsafe {
        env_set(env, 0, Value::Float(1.234));
        env_set(env, 1, Value::Float(-5.678));

        let env_slice = &*env;
        let env_data = env_slice.as_ptr();
        let env_len = env_slice.len();

        assert!((env_get_float(env_data, env_len, 0) - 1.234).abs() < 0.0001);
        assert!((env_get_float(env_data, env_len, 1) - (-5.678)).abs() < 0.0001);

        let _ = Box::from_raw(env);
    }
}

#[test]
fn test_env_get_quotation() {
    let env = create_env(1);
    let wrapper: usize = 0xDEADBEEF;
    let impl_: usize = 0xCAFEBABE;

    unsafe {
        env_set(env, 0, Value::Quotation { wrapper, impl_ });

        let env_slice = &*env;
        let env_data = env_slice.as_ptr();
        let env_len = env_slice.len();

        // env_get_quotation returns the impl_ pointer for TCO
        assert_eq!(env_get_quotation(env_data, env_len, 0), impl_ as i64);

        let _ = Box::from_raw(env);
    }
}
