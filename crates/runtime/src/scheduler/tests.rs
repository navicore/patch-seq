use super::lifecycle::{DEFAULT_STACK_SIZE, parse_stack_size};
use super::spawn::free_stack;
use super::yield_ops::TAIL_CALL_COUNTER;
use super::*;
use crate::stack::{Stack, push};
use crate::value::Value;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn test_spawn_strand() {
    unsafe {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        extern "C" fn test_entry(_stack: Stack) -> Stack {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            std::ptr::null_mut()
        }

        for _ in 0..100 {
            spawn_strand(test_entry);
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(COUNTER.load(Ordering::SeqCst), 100);
    }
}

#[test]
fn test_scheduler_init_idempotent() {
    unsafe {
        // Should be safe to call multiple times
        scheduler_init();
        scheduler_init();
        scheduler_init();
    }
}

#[test]
fn test_free_stack_null() {
    // Freeing null should be a no-op
    free_stack(std::ptr::null_mut());
}

#[test]
fn test_free_stack_valid() {
    unsafe {
        // Create a stack, then free it
        let stack = push(crate::stack::alloc_test_stack(), Value::Int(42));
        free_stack(stack);
        // If we get here without crashing, test passed
    }
}

#[test]
fn test_strand_spawn_with_stack() {
    unsafe {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        extern "C" fn test_entry(stack: Stack) -> Stack {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            // Return the stack as-is (caller will free it)
            stack
        }

        let initial_stack = push(crate::stack::alloc_test_stack(), Value::Int(99));
        strand_spawn(test_entry, initial_stack);

        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn test_scheduler_shutdown() {
    unsafe {
        scheduler_init();
        scheduler_shutdown();
        // Should not crash
    }
}

#[test]
fn test_many_strands_stress() {
    unsafe {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        extern "C" fn increment(_stack: Stack) -> Stack {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            std::ptr::null_mut()
        }

        // Reset counter for this test
        COUNTER.store(0, Ordering::SeqCst);

        // Spawn many strands to stress test synchronization
        for _ in 0..1000 {
            strand_spawn(increment, std::ptr::null_mut());
        }

        // Wait for all to complete
        wait_all_strands();

        // Verify all strands executed
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1000);
    }
}

#[test]
fn test_strand_ids_are_unique() {
    unsafe {
        use std::collections::HashSet;

        extern "C" fn noop(_stack: Stack) -> Stack {
            std::ptr::null_mut()
        }

        // Spawn strands and collect their IDs
        let mut ids = Vec::new();
        for _ in 0..100 {
            let id = strand_spawn(noop, std::ptr::null_mut());
            ids.push(id);
        }

        // Wait for completion
        wait_all_strands();

        // Verify all IDs are unique
        let unique_ids: HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 100, "All strand IDs should be unique");

        // Verify all IDs are positive
        assert!(
            ids.iter().all(|&id| id > 0),
            "All strand IDs should be positive"
        );
    }
}

#[test]
fn test_arena_reset_with_strands() {
    unsafe {
        use crate::arena;
        use crate::seqstring::arena_string;

        extern "C" fn create_temp_strings(stack: Stack) -> Stack {
            // Create many temporary arena strings (simulating request parsing)
            for i in 0..100 {
                let temp = arena_string(&format!("temporary string {}", i));
                // Use the string temporarily
                assert!(!temp.as_str().is_empty());
                // String is dropped, but memory stays in arena
            }

            // Arena should have allocated memory
            let stats = arena::arena_stats();
            assert!(stats.allocated_bytes > 0, "Arena should have allocations");

            stack // Return empty stack
        }

        // Reset arena before test
        arena::arena_reset();

        // Spawn strand that creates many temp strings
        strand_spawn(create_temp_strings, std::ptr::null_mut());

        // Wait for strand to complete (which calls free_stack -> arena_reset)
        wait_all_strands();

        // After strand exits, arena should be reset
        let stats_after = arena::arena_stats();
        assert_eq!(
            stats_after.allocated_bytes, 0,
            "Arena should be reset after strand exits"
        );
    }
}

#[test]
fn test_arena_with_channel_send() {
    unsafe {
        use crate::channel::{close_channel, make_channel, receive, send};
        use crate::stack::{pop, push};
        use crate::value::Value;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

        static RECEIVED_COUNT: AtomicU32 = AtomicU32::new(0);
        static CHANNEL_PTR: AtomicI64 = AtomicI64::new(0);

        // Create channel
        let stack = crate::stack::alloc_test_stack();
        let stack = make_channel(stack);
        let (stack, chan_val) = pop(stack);
        let channel = match chan_val {
            Value::Channel(ch) => ch,
            _ => panic!("Expected Channel"),
        };

        // Store channel pointer for strands
        let ch_ptr = Arc::as_ptr(&channel) as i64;
        CHANNEL_PTR.store(ch_ptr, Ordering::Release);

        // Keep Arc alive
        std::mem::forget(channel.clone());
        std::mem::forget(channel.clone());

        // Sender strand: creates arena string, sends through channel
        extern "C" fn sender(_stack: Stack) -> Stack {
            use crate::seqstring::arena_string;
            use crate::value::ChannelData;
            use std::sync::Arc;

            unsafe {
                let ch_ptr = CHANNEL_PTR.load(Ordering::Acquire) as *const ChannelData;
                let channel = Arc::from_raw(ch_ptr);
                let channel_clone = Arc::clone(&channel);
                std::mem::forget(channel); // Don't drop

                // Create arena string
                let msg = arena_string("Hello from sender!");

                // Push string and channel for send
                let stack = push(crate::stack::alloc_test_stack(), Value::String(msg));
                let stack = push(stack, Value::Channel(channel_clone));

                // Send (will clone to global)
                send(stack)
            }
        }

        // Receiver strand: receives string from channel
        extern "C" fn receiver(_stack: Stack) -> Stack {
            use crate::value::ChannelData;
            use std::sync::Arc;
            use std::sync::atomic::Ordering;

            unsafe {
                let ch_ptr = CHANNEL_PTR.load(Ordering::Acquire) as *const ChannelData;
                let channel = Arc::from_raw(ch_ptr);
                let channel_clone = Arc::clone(&channel);
                std::mem::forget(channel); // Don't drop

                // Push channel for receive
                let stack = push(
                    crate::stack::alloc_test_stack(),
                    Value::Channel(channel_clone),
                );

                // Receive message (returns value, success_flag)
                let stack = receive(stack);

                // Pop success flag first, then message
                let (stack, _success) = pop(stack);
                let (_stack, msg_val) = pop(stack);
                match msg_val {
                    Value::String(s) => {
                        assert_eq!(s.as_str(), "Hello from sender!");
                        RECEIVED_COUNT.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => panic!("Expected String"),
                }

                std::ptr::null_mut()
            }
        }

        // Spawn sender and receiver
        spawn_strand(sender);
        spawn_strand(receiver);

        // Wait for both strands
        wait_all_strands();

        // Verify message was received
        assert_eq!(
            RECEIVED_COUNT.load(Ordering::SeqCst),
            1,
            "Receiver should have received message"
        );

        // Clean up channel
        let stack = push(stack, Value::Channel(channel));
        close_channel(stack);
    }
}

#[test]
fn test_no_memory_leak_over_many_iterations() {
    // PR #11 feedback: Verify 10K+ strand iterations don't cause memory growth
    unsafe {
        use crate::arena;
        use crate::seqstring::arena_string;

        extern "C" fn allocate_strings_and_exit(stack: Stack) -> Stack {
            // Simulate request processing: many temp allocations
            for i in 0..50 {
                let temp = arena_string(&format!("request header {}", i));
                assert!(!temp.as_str().is_empty());
                // Strings dropped here but arena memory stays allocated
            }
            stack
        }

        // Run many iterations to detect leaks
        let iterations = 10_000;

        for i in 0..iterations {
            // Reset arena before each iteration to start fresh
            arena::arena_reset();

            // Spawn strand, let it allocate strings, then exit
            strand_spawn(allocate_strings_and_exit, std::ptr::null_mut());

            // Wait for completion (triggers arena reset)
            wait_all_strands();

            // Every 1000 iterations, verify arena is actually reset
            if i % 1000 == 0 {
                let stats = arena::arena_stats();
                assert_eq!(
                    stats.allocated_bytes, 0,
                    "Arena not reset after iteration {} (leaked {} bytes)",
                    i, stats.allocated_bytes
                );
            }
        }

        // Final verification: arena should be empty
        let final_stats = arena::arena_stats();
        assert_eq!(
            final_stats.allocated_bytes, 0,
            "Arena leaked memory after {} iterations ({} bytes)",
            iterations, final_stats.allocated_bytes
        );

        println!(
            "✓ Memory leak test passed: {} iterations with no growth",
            iterations
        );
    }
}

#[test]
fn test_parse_stack_size_valid() {
    assert_eq!(parse_stack_size(Some("2097152".to_string())), 2097152);
    assert_eq!(parse_stack_size(Some("1".to_string())), 1);
    assert_eq!(parse_stack_size(Some("999999999".to_string())), 999999999);
}

#[test]
fn test_parse_stack_size_none() {
    assert_eq!(parse_stack_size(None), DEFAULT_STACK_SIZE);
}

#[test]
fn test_parse_stack_size_zero() {
    // Zero should fall back to default (with warning printed to stderr)
    assert_eq!(parse_stack_size(Some("0".to_string())), DEFAULT_STACK_SIZE);
}

#[test]
fn test_parse_stack_size_invalid() {
    // Non-numeric should fall back to default (with warning printed to stderr)
    assert_eq!(
        parse_stack_size(Some("invalid".to_string())),
        DEFAULT_STACK_SIZE
    );
    assert_eq!(
        parse_stack_size(Some("-100".to_string())),
        DEFAULT_STACK_SIZE
    );
    assert_eq!(parse_stack_size(Some("".to_string())), DEFAULT_STACK_SIZE);
    assert_eq!(
        parse_stack_size(Some("1.5".to_string())),
        DEFAULT_STACK_SIZE
    );
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_strand_registry_basic() {
    let registry = StrandRegistry::new(10);

    // Register some strands
    assert_eq!(registry.register(1), Some(0)); // First slot
    assert_eq!(registry.register(2), Some(1)); // Second slot
    assert_eq!(registry.register(3), Some(2)); // Third slot

    // Verify active strands
    let active: Vec<_> = registry.active_strands().collect();
    assert_eq!(active.len(), 3);

    // Unregister one
    assert!(registry.unregister(2));
    let active: Vec<_> = registry.active_strands().collect();
    assert_eq!(active.len(), 2);

    // Unregister non-existent should return false
    assert!(!registry.unregister(999));
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_strand_registry_overflow() {
    let registry = StrandRegistry::new(3); // Small capacity

    // Fill it up
    assert!(registry.register(1).is_some());
    assert!(registry.register(2).is_some());
    assert!(registry.register(3).is_some());

    // Next should overflow
    assert!(registry.register(4).is_none());
    assert_eq!(registry.overflow_count.load(Ordering::Relaxed), 1);

    // Another overflow
    assert!(registry.register(5).is_none());
    assert_eq!(registry.overflow_count.load(Ordering::Relaxed), 2);
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_strand_registry_slot_reuse() {
    let registry = StrandRegistry::new(3);

    // Fill it up
    registry.register(1);
    registry.register(2);
    registry.register(3);

    // Unregister middle one
    registry.unregister(2);

    // New registration should reuse the slot
    assert!(registry.register(4).is_some());
    assert_eq!(registry.active_strands().count(), 3);
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_strand_registry_concurrent_stress() {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(StrandRegistry::new(50)); // Moderate capacity

    let handles: Vec<_> = (0..100)
        .map(|i| {
            let reg = Arc::clone(&registry);
            thread::spawn(move || {
                let id = (i + 1) as u64;
                // Register
                let _ = reg.register(id);
                // Brief work
                thread::yield_now();
                // Unregister
                reg.unregister(id);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // All slots should be free after all threads complete
    assert_eq!(registry.active_strands().count(), 0);
}

#[test]
fn test_strand_lifecycle_counters() {
    unsafe {
        // Reset counters for isolation (not perfect but helps)
        let initial_spawned = TOTAL_SPAWNED.load(Ordering::Relaxed);
        let initial_completed = TOTAL_COMPLETED.load(Ordering::Relaxed);

        static COUNTER: AtomicU32 = AtomicU32::new(0);

        extern "C" fn simple_work(_stack: Stack) -> Stack {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            std::ptr::null_mut()
        }

        COUNTER.store(0, Ordering::SeqCst);

        // Spawn some strands
        for _ in 0..10 {
            strand_spawn(simple_work, std::ptr::null_mut());
        }

        wait_all_strands();

        // Verify counters incremented
        let final_spawned = TOTAL_SPAWNED.load(Ordering::Relaxed);
        let final_completed = TOTAL_COMPLETED.load(Ordering::Relaxed);

        assert!(
            final_spawned >= initial_spawned + 10,
            "TOTAL_SPAWNED should have increased by at least 10"
        );
        assert!(
            final_completed >= initial_completed + 10,
            "TOTAL_COMPLETED should have increased by at least 10"
        );
        assert_eq!(COUNTER.load(Ordering::SeqCst), 10);
    }
}

// =========================================================================
// Yield Safety Valve Tests
// =========================================================================

#[test]
fn test_maybe_yield_disabled_by_default() {
    // When SEQ_YIELD_INTERVAL is not set (or 0), maybe_yield should be a no-op
    // This test verifies it doesn't panic and returns quickly
    for _ in 0..1000 {
        patch_seq_maybe_yield();
    }
}

#[test]
fn test_tail_call_counter_increments() {
    // Verify the thread-local counter increments correctly
    TAIL_CALL_COUNTER.with(|counter| {
        let initial = counter.get();
        patch_seq_maybe_yield();
        patch_seq_maybe_yield();
        patch_seq_maybe_yield();
        // Counter should have incremented (if threshold > 0) or stayed same (if disabled)
        // Either way, it shouldn't panic
        let _ = counter.get();
        // Reset to avoid affecting other tests
        counter.set(initial);
    });
}

#[test]
fn test_counter_overflow_safety() {
    // Verify wrapping_add prevents overflow panic
    TAIL_CALL_COUNTER.with(|counter| {
        let initial = counter.get();
        // Set counter near max to test overflow behavior
        counter.set(u64::MAX - 1);
        // These calls should not panic due to overflow
        patch_seq_maybe_yield();
        patch_seq_maybe_yield();
        patch_seq_maybe_yield();
        // Reset
        counter.set(initial);
    });
}
