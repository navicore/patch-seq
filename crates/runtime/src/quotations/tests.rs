use super::*;
use crate::arithmetic::push_int;
use crate::value::Value;

#[test]
fn test_spawn_registry_guard_cleanup() {
    // Test that the RAII guard cleans up the registry on drop
    let closure_id = 12345;

    // Create a test closure environment
    let env: Box<[Value]> = vec![Value::Int(42), Value::Int(99)].into_boxed_slice();
    let fn_ptr: usize = 0x1234;

    // Insert into registry
    {
        let mut registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        registry.insert(closure_id, (fn_ptr, env));
    }

    // Verify it's in the registry
    {
        let registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        assert!(registry.contains_key(&closure_id));
    }

    // Create a guard (without disarming) and let it drop
    {
        let _guard = SpawnRegistryGuard::new(closure_id);
        // Guard drops here, should clean up the registry
    }

    // Verify the registry was cleaned up
    {
        let registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        assert!(
            !registry.contains_key(&closure_id),
            "Guard should have cleaned up registry entry on drop"
        );
    }
}

#[test]
fn test_spawn_registry_guard_disarm() {
    // Test that disarming the guard prevents cleanup
    let closure_id = 54321;

    // Create a test closure environment
    let env: Box<[Value]> = vec![Value::Int(10), Value::Int(20)].into_boxed_slice();
    let fn_ptr: usize = 0x5678;

    // Insert into registry
    {
        let mut registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        registry.insert(closure_id, (fn_ptr, env));
    }

    // Create a guard, disarm it, and let it drop
    {
        let mut guard = SpawnRegistryGuard::new(closure_id);
        guard.disarm();
        // Guard drops here, but should NOT clean up because it's disarmed
    }

    // Verify the registry entry is still there
    {
        let registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        assert!(
            registry.contains_key(&closure_id),
            "Disarmed guard should not clean up registry entry"
        );

        // Manual cleanup for this test
        drop(registry);
        let mut registry = SPAWN_CLOSURE_REGISTRY.lock().unwrap();
        registry.remove(&closure_id);
    }
}

// Helper function for testing: a quotation that adds 1
unsafe extern "C" fn add_one_quot(stack: Stack) -> Stack {
    unsafe {
        let stack = push_int(stack, 1);
        crate::arithmetic::add(stack)
    }
}

#[test]
fn test_push_quotation() {
    unsafe {
        let stack: Stack = crate::stack::alloc_test_stack();

        // Push a quotation (for tests, wrapper and impl are the same C function)
        let fn_ptr = add_one_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Verify it's on the stack
        let (_stack, value) = pop(stack);
        assert!(matches!(value, Value::Quotation { .. }));
    }
}

#[test]
fn test_call_quotation() {
    unsafe {
        let stack: Stack = crate::stack::alloc_test_stack();

        // Push 5, then a quotation that adds 1
        let stack = push_int(stack, 5);
        let fn_ptr = add_one_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Call the quotation
        let stack = call(stack);

        // Result should be 6
        let (_stack, result) = pop(stack);
        assert_eq!(result, Value::Int(6));
    }
}

// Helper quotation for spawn test: does nothing, just completes
unsafe extern "C" fn noop_quot(stack: Stack) -> Stack {
    stack
}

#[test]
fn test_spawn_quotation() {
    unsafe {
        // Initialize scheduler
        crate::scheduler::scheduler_init();

        let stack: Stack = crate::stack::alloc_test_stack();

        // Push a quotation
        let fn_ptr = noop_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Spawn it
        let stack = spawn(stack);

        // Should have strand ID on stack
        let (_stack, result) = pop(stack);
        match result {
            Value::Int(strand_id) => {
                assert!(strand_id > 0, "Strand ID should be positive");
            }
            _ => panic!("Expected Int (strand ID), got {:?}", result),
        }

        // Wait for strand to complete
        crate::scheduler::wait_all_strands();
    }
}
