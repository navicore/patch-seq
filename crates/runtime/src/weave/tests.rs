use super::*;
use crate::quotations::push_quotation;
use crate::scheduler::{scheduler_init, wait_all_strands};
use crate::stack::{alloc_test_stack, pop, push};
use crate::value::Value;
use std::sync::atomic::{AtomicBool, Ordering};

// =========================================================================
// Test Helper Quotations
// =========================================================================

/// A quotation that yields once and completes
/// Stack effect: ( WeaveCtx resume_value -- WeaveCtx )
/// Yields: resume_value + 100
unsafe extern "C" fn yield_once_quot(stack: Stack) -> Stack {
    unsafe {
        // Pop resume value
        let (stack, resume_val) = pop(stack);
        let n = match resume_val {
            Value::Int(i) => i,
            _ => 0,
        };

        // Push value to yield (resume_value + 100)
        let stack = push(stack, Value::Int(n + 100));

        // Yield - WeaveCtx is below our value
        let stack = weave_yield(stack);

        // After yield, we have (WeaveCtx, new_resume_value)
        // Pop the new resume value and complete
        let (stack, _new_resume) = pop(stack);

        // Return with WeaveCtx on stack (signals completion)
        stack
    }
}

/// A quotation that yields multiple times (3 times)
/// Stack effect: ( WeaveCtx resume_value -- WeaveCtx )
/// Yields: 1, 2, 3 then completes
unsafe extern "C" fn yield_three_times_quot(stack: Stack) -> Stack {
    unsafe {
        // Pop initial resume value (we ignore it)
        let (stack, _) = pop(stack);

        // Yield 1
        let stack = push(stack, Value::Int(1));
        let stack = weave_yield(stack);
        let (stack, _) = pop(stack); // pop resume value

        // Yield 2
        let stack = push(stack, Value::Int(2));
        let stack = weave_yield(stack);
        let (stack, _) = pop(stack); // pop resume value

        // Yield 3
        let stack = push(stack, Value::Int(3));
        let stack = weave_yield(stack);
        let (stack, _) = pop(stack); // pop resume value

        // Complete - return with WeaveCtx on stack
        stack
    }
}

/// A quotation that never yields (completes immediately)
/// Stack effect: ( WeaveCtx resume_value -- WeaveCtx )
unsafe extern "C" fn no_yield_quot(stack: Stack) -> Stack {
    unsafe {
        // Pop resume value and complete immediately
        let (stack, _) = pop(stack);
        stack
    }
}

/// A quotation that echoes the resume value back
/// Stack effect: ( WeaveCtx resume_value -- WeaveCtx )
/// Yields the same value it receives, loops until receives negative
unsafe extern "C" fn echo_quot(stack: Stack) -> Stack {
    unsafe {
        let (mut stack, mut resume_val) = pop(stack);

        loop {
            let n = match resume_val {
                Value::Int(i) => i,
                _ => -1,
            };

            // If negative, complete
            if n < 0 {
                break;
            }

            // Echo the value back
            stack = push(stack, Value::Int(n));
            stack = weave_yield(stack);
            let (new_stack, new_val) = pop(stack);
            stack = new_stack;
            resume_val = new_val;
        }

        stack
    }
}

// =========================================================================
// Basic Weave Tests
// =========================================================================

#[test]
fn test_weave_create() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_once_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Should have WeaveHandle on stack
        let (_, handle) = pop(stack);
        assert!(
            matches!(handle, Value::WeaveCtx { .. }),
            "Expected WeaveCtx (handle), got {:?}",
            handle
        );
    }
}

#[test]
fn test_weave_single_yield() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_once_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Resume with value 42
        let stack = push(stack, Value::Int(42));
        let stack = resume(stack);

        // Should get (handle, yielded_value, true)
        let (stack, has_more) = pop(stack);
        let (stack, yielded) = pop(stack);
        let (_, _handle) = pop(stack);

        assert_eq!(has_more, Value::Bool(true), "Should have more");
        assert_eq!(yielded, Value::Int(142), "Should yield 42 + 100 = 142");

        wait_all_strands();
    }
}

#[test]
fn test_weave_completion() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_once_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // First resume - gets yielded value
        let stack = push(stack, Value::Int(10));
        let stack = resume(stack);
        let (stack, has_more1) = pop(stack);
        let (stack, _yielded) = pop(stack);
        assert_eq!(has_more1, Value::Bool(true));

        // Second resume - weave completes
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more2) = pop(stack);
        let (_stack, _placeholder) = pop(stack);

        assert_eq!(has_more2, Value::Bool(false), "Weave should be complete");

        wait_all_strands();
    }
}

#[test]
fn test_weave_no_yield() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = no_yield_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Resume - weave completes immediately without yielding
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);

        let (stack, has_more) = pop(stack);
        let (_stack, _placeholder) = pop(stack);

        assert_eq!(
            has_more,
            Value::Bool(false),
            "Weave should complete immediately"
        );

        wait_all_strands();
    }
}

#[test]
fn test_weave_multiple_yields() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_three_times_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Resume 1 - should yield 1
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more1) = pop(stack);
        let (stack, yielded1) = pop(stack);
        assert_eq!(has_more1, Value::Bool(true));
        assert_eq!(yielded1, Value::Int(1));

        // Resume 2 - should yield 2
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more2) = pop(stack);
        let (stack, yielded2) = pop(stack);
        assert_eq!(has_more2, Value::Bool(true));
        assert_eq!(yielded2, Value::Int(2));

        // Resume 3 - should yield 3
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more3) = pop(stack);
        let (stack, yielded3) = pop(stack);
        assert_eq!(has_more3, Value::Bool(true));
        assert_eq!(yielded3, Value::Int(3));

        // Resume 4 - weave completes
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more4) = pop(stack);
        let (_stack, _) = pop(stack);
        assert_eq!(has_more4, Value::Bool(false));

        wait_all_strands();
    }
}

#[test]
fn test_weave_echo() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = echo_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Echo 42
        let stack = push(stack, Value::Int(42));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (stack, yielded) = pop(stack);
        assert_eq!(has_more, Value::Bool(true));
        assert_eq!(yielded, Value::Int(42));

        // Echo 99
        let stack = push(stack, Value::Int(99));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (stack, yielded) = pop(stack);
        assert_eq!(has_more, Value::Bool(true));
        assert_eq!(yielded, Value::Int(99));

        // Send negative to complete
        let stack = push(stack, Value::Int(-1));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (_stack, _) = pop(stack);
        assert_eq!(has_more, Value::Bool(false));

        wait_all_strands();
    }
}

// =========================================================================
// Cancellation Tests
// =========================================================================

#[test]
fn test_weave_cancel_before_resume() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_three_times_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave but don't resume
        let stack = weave(stack);

        // Cancel immediately
        let _stack = weave_cancel(stack);

        // Should not block - weave was dormant
        wait_all_strands();
    }
}

#[test]
fn test_weave_cancel_after_yield() {
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_three_times_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave
        let stack = weave(stack);

        // Resume once to get first yield
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, _) = pop(stack); // has_more
        let (stack, _) = pop(stack); // yielded value

        // Cancel instead of continuing
        let _stack = weave_cancel(stack);

        wait_all_strands();
    }
}

// =========================================================================
// Dormant Strand Tests (Issue #287)
// =========================================================================

#[test]
fn test_dormant_weave_doesnt_block_shutdown() {
    // This tests that creating a weave without resuming it doesn't
    // prevent the program from exiting (the weave is "dormant")
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_three_times_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        // Create weave but never resume it
        let _stack = weave(stack);

        // This should return immediately since the weave is dormant
        // (not counted in ACTIVE_STRANDS)
        wait_all_strands();

        // If we get here, the test passed - dormant weave didn't block
    }
}

#[test]
fn test_multiple_dormant_weaves() {
    unsafe {
        scheduler_init();

        // Create multiple weaves without resuming any
        for _ in 0..10 {
            let stack = alloc_test_stack();
            let fn_ptr = yield_three_times_quot as *const () as usize;
            let stack = push_quotation(stack, fn_ptr, fn_ptr);
            let _stack = weave(stack);
        }

        // Should return immediately
        wait_all_strands();
    }
}

// =========================================================================
// Error Handling Tests
// =========================================================================

#[test]
fn test_resume_wrong_type() {
    // resume with non-WeaveHandle should abort, but we can't test abort
    // This test documents the expected behavior via comments
    //
    // If called with wrong type:
    // - eprintln!("strand.resume: expected WeaveHandle, got ...")
    // - std::process::abort()
    //
    // We don't test this directly because abort() terminates the process
}

// =========================================================================
// Integration Tests
// =========================================================================

#[test]
fn test_weave_with_active_strands() {
    // Test that weaves work correctly alongside regular strands
    unsafe {
        use crate::scheduler::strand_spawn;

        scheduler_init();

        static STRAND_COMPLETED: AtomicBool = AtomicBool::new(false);

        extern "C" fn simple_strand(_stack: Stack) -> Stack {
            STRAND_COMPLETED.store(true, Ordering::SeqCst);
            std::ptr::null_mut()
        }

        // Spawn a regular strand
        strand_spawn(simple_strand, std::ptr::null_mut());

        // Create and use a weave
        let stack = alloc_test_stack();
        let fn_ptr = yield_once_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);
        let stack = weave(stack);

        // Resume weave
        let stack = push(stack, Value::Int(5));
        let stack = resume(stack);
        let (stack, _) = pop(stack);
        let (stack, _) = pop(stack);

        // Complete weave
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, _) = pop(stack);
        let (_stack, _) = pop(stack);

        wait_all_strands();

        assert!(
            STRAND_COMPLETED.load(Ordering::SeqCst),
            "Regular strand should have completed"
        );
    }
}

#[test]
fn test_weave_generator_pattern() {
    // Test the common generator pattern: iterate until completion
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_three_times_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        let stack = weave(stack);

        let mut collected = Vec::new();
        let mut current_stack = stack;

        // Generator loop: resume until has_more is false
        loop {
            current_stack = push(current_stack, Value::Int(0));
            current_stack = resume(current_stack);

            let (s, has_more) = pop(current_stack);
            let (s, value) = pop(s);
            current_stack = s;

            match has_more {
                Value::Bool(true) => {
                    if let Value::Int(n) = value {
                        collected.push(n);
                    }
                }
                Value::Bool(false) => {
                    // Pop handle and exit
                    let (_s, _handle) = pop(current_stack);
                    break;
                }
                _ => panic!("Unexpected has_more value"),
            }
        }

        assert_eq!(collected, vec![1, 2, 3]);

        wait_all_strands();
    }
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
fn test_weave_yields_zero() {
    // Ensure yielding 0 works (not confused with completion)
    unsafe {
        scheduler_init();

        // Use echo_quot which echoes whatever we send
        let stack = alloc_test_stack();
        let fn_ptr = echo_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        let stack = weave(stack);

        // Send 0 - should echo 0, not be confused with completion
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (stack, yielded) = pop(stack);

        assert_eq!(has_more, Value::Bool(true), "Should still have more");
        assert_eq!(yielded, Value::Int(0), "Should yield 0");

        // Complete with negative
        let stack = push(stack, Value::Int(-1));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (_stack, _) = pop(stack);
        assert_eq!(has_more, Value::Bool(false));

        wait_all_strands();
    }
}

#[test]
fn test_weave_yields_negative() {
    // Ensure yielding negative values works
    unsafe {
        scheduler_init();

        let stack = alloc_test_stack();
        let fn_ptr = yield_once_quot as *const () as usize;
        let stack = push_quotation(stack, fn_ptr, fn_ptr);

        let stack = weave(stack);

        // Resume with -50, should yield -50 + 100 = 50
        let stack = push(stack, Value::Int(-50));
        let stack = resume(stack);
        let (stack, has_more) = pop(stack);
        let (stack, yielded) = pop(stack);

        assert_eq!(has_more, Value::Bool(true));
        assert_eq!(yielded, Value::Int(50));

        // Complete
        let stack = push(stack, Value::Int(0));
        let stack = resume(stack);
        let (stack, _) = pop(stack);
        let (_stack, _) = pop(stack);

        wait_all_strands();
    }
}

// Note: Tests for panic/abort conditions (null stack, type mismatch) are documented
// but not executed because extern "C" functions cannot unwind and abort() terminates
// the process. The expected behavior is documented in the function comments:
//
// - strand.weave with null stack: eprintln + abort
// - strand.weave with non-Quotation: eprintln + abort
// - strand.resume with null stack: eprintln + abort
// - strand.resume with non-WeaveHandle: eprintln + abort
// - yield with null stack: cleanup + block_forever
// - yield with non-WeaveCtx: eprintln + cleanup + block_forever
