use super::*;
use crate::arithmetic::push_int;
use crate::scheduler::scheduler_init;

/// Helper: bind a UDP socket on `0.0.0.0:port`, return `(socket_id, bound_port)`.
/// Asserts success.
unsafe fn bind_succeeds(port: i64) -> (i64, i64) {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, port);
        let stack = udp_bind(stack);

        let (stack, success) = pop(stack);
        assert!(
            matches!(success, Value::Bool(true)),
            "udp.bind({}) should succeed",
            port
        );
        let (stack, bound_port_v) = pop(stack);
        let bound_port = match bound_port_v {
            Value::Int(p) => p,
            other => panic!("expected bound-port Int, got {:?}", other),
        };
        let (_, socket_v) = pop(stack);
        let socket_id = match socket_v {
            Value::Int(s) => s,
            other => panic!("expected socket Int, got {:?}", other),
        };
        (socket_id, bound_port)
    }
}

#[test]
fn test_udp_bind_returns_assigned_port() {
    unsafe {
        scheduler_init();

        // port=0 lets the OS pick — we must get back a non-zero bound port.
        let (socket_id, bound_port) = bind_succeeds(0);
        assert!(socket_id >= 0, "socket id should be non-negative");
        assert!(
            bound_port > 0,
            "OS-assigned bound port should be non-zero, got {}",
            bound_port
        );
    }
}

#[test]
fn test_udp_bind_invalid_port_negative() {
    unsafe {
        scheduler_init();
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, -1);
        let stack = udp_bind(stack);

        // (0, 0, false)
        let (stack, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));
        let (stack, bound_port) = pop(stack);
        assert!(matches!(bound_port, Value::Int(0)));
        let (_, socket) = pop(stack);
        assert!(matches!(socket, Value::Int(0)));
    }
}

#[test]
fn test_udp_bind_invalid_port_too_high() {
    unsafe {
        scheduler_init();
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 65536);
        let stack = udp_bind(stack);

        let (stack, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));
        let (stack, bound_port) = pop(stack);
        assert!(matches!(bound_port, Value::Int(0)));
        let (_, socket) = pop(stack);
        assert!(matches!(socket, Value::Int(0)));
    }
}

#[test]
fn test_udp_loopback_round_trip() {
    // Bind socket A on 127.0.0.1:0 (OS-assigned port).
    // Bind socket B on 127.0.0.1:0 (sender side).
    // From B, send a payload to 127.0.0.1:<A's bound port>.
    // From A, receive — assert byte-exact match including source port == B's port.
    unsafe {
        scheduler_init();

        let (sock_a, port_a) = bind_succeeds(0);
        let (sock_b, port_b) = bind_succeeds(0);
        assert_ne!(port_a, port_b, "A and B should have different ports");

        // udp.send-to: ( bytes host port socket -- Bool )
        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("hello".into()));
        let stack = push(stack, Value::String("127.0.0.1".into()));
        let stack = push_int(stack, port_a);
        let stack = push_int(stack, sock_b);
        let stack = udp_send_to(stack);
        let (_, send_success) = pop(stack);
        assert!(
            matches!(send_success, Value::Bool(true)),
            "send-to should succeed"
        );

        // udp.receive-from: ( socket -- bytes host port Bool )
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock_a);
        let stack = udp_receive_from(stack);

        let (stack, recv_success) = pop(stack);
        assert!(
            matches!(recv_success, Value::Bool(true)),
            "receive-from should succeed"
        );
        let (stack, src_port) = pop(stack);
        assert!(
            matches!(src_port, Value::Int(p) if p == port_b),
            "source port should be B's bound port {}, got {:?}",
            port_b,
            src_port
        );
        let (stack, src_host) = pop(stack);
        match src_host {
            Value::String(s) => assert_eq!(s.as_str(), "127.0.0.1"),
            other => panic!("expected source host, got {:?}", other),
        }
        let (_, payload) = pop(stack);
        match payload {
            Value::String(s) => assert_eq!(s.as_str(), "hello"),
            other => panic!("expected payload, got {:?}", other),
        }
    }
}

#[test]
fn test_udp_send_to_invalid_socket() {
    unsafe {
        scheduler_init();

        let stack = crate::stack::alloc_test_stack();
        let stack = push(stack, Value::String("hi".into()));
        let stack = push(stack, Value::String("127.0.0.1".into()));
        let stack = push_int(stack, 9999);
        let stack = push_int(stack, 99_999); // invalid socket id
        let stack = udp_send_to(stack);

        let (_, success) = pop(stack);
        assert!(
            matches!(success, Value::Bool(false)),
            "send-to on invalid socket should return false"
        );
    }
}

#[test]
fn test_udp_receive_from_invalid_socket() {
    unsafe {
        scheduler_init();

        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 99_999);
        let stack = udp_receive_from(stack);

        // ("", "", 0, false)
        let (stack, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));
        let (stack, port) = pop(stack);
        assert!(matches!(port, Value::Int(0)));
        let (stack, host) = pop(stack);
        match host {
            Value::String(s) => assert_eq!(s.as_str(), ""),
            other => panic!("expected empty host, got {:?}", other),
        }
        let (_, bytes) = pop(stack);
        match bytes {
            Value::String(s) => assert_eq!(s.as_str(), ""),
            other => panic!("expected empty bytes, got {:?}", other),
        }
    }
}

#[test]
fn test_udp_close_idempotent() {
    unsafe {
        scheduler_init();

        // Bind, then close — should succeed.
        let (sock, _) = bind_succeeds(0);
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(matches!(success, Value::Bool(true)));

        // Closing an already-closed/invalid socket returns false.
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 99_999);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));
    }
}

#[test]
fn test_udp_socket_registry_constants() {
    // Documents the limits — same shape as `tcp::tests`.
    assert_eq!(MAX_SOCKETS, 10_000);
    assert_eq!(MAX_READ_SIZE, 1_048_576);
}
