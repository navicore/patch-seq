use super::*;
use crate::stack::pop;
use std::time::Instant;

#[test]
fn test_time_now_returns_positive() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = patch_seq_time_now(stack);
        let (_, value) = pop(stack);

        match value {
            Value::Int(micros) => {
                // Should be a reasonable timestamp (after year 2020)
                assert!(micros > 1_577_836_800_000_000); // 2020-01-01
            }
            _ => panic!("Expected Int"),
        }
    }
}

#[test]
fn test_time_nanos_monotonic() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = patch_seq_time_nanos(stack);
        let (_, value1) = pop(stack);

        // Small delay
        std::thread::sleep(Duration::from_micros(100));

        let stack = crate::stack::alloc_test_stack();
        let stack = patch_seq_time_nanos(stack);
        let (_, value2) = pop(stack);

        match (value1, value2) {
            (Value::Int(t1), Value::Int(t2)) => {
                assert!(t2 > t1, "time.nanos should be monotonically increasing");
            }
            _ => panic!("Expected Int values"),
        }
    }
}

#[test]
fn test_time_nanos_cross_thread() {
    // Verify raw_monotonic_nanos is consistent across threads
    use std::sync::mpsc;
    use std::thread;

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    // Get time on main thread
    let t1 = raw_monotonic_nanos();

    // Spawn thread, get time there
    let handle = thread::spawn(move || {
        let t2 = raw_monotonic_nanos();
        tx1.send(t2).unwrap();
        rx2.recv().unwrap() // wait for main to continue
    });

    let t2 = rx1.recv().unwrap();

    // Get time on main thread again
    let t3 = raw_monotonic_nanos();
    tx2.send(()).unwrap();
    handle.join().unwrap();

    // All times should be monotonically increasing
    assert!(t2 > t1, "t2 ({}) should be > t1 ({})", t2, t1);
    assert!(t3 > t2, "t3 ({}) should be > t2 ({})", t3, t2);
}

#[test]
fn test_time_sleep_ms() {
    unsafe {
        // Push 1ms sleep duration
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::Int(1));

        let start = Instant::now();
        let _stack = patch_seq_time_sleep_ms(stack);
        let elapsed = start.elapsed();

        // Should sleep at least 1ms
        assert!(elapsed >= Duration::from_millis(1));
        // Stack should be empty after popping the duration
    }
}
