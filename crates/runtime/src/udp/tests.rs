use super::*;
use crate::arithmetic::push_int;
use crate::scheduler::scheduler_init;
use may::net::UdpSocket as MayUdpSocket;
use std::sync::Mutex;

/// Serializes tests whose assertions depend on `SOCKETS` registry
/// stability across multiple operations (e.g. "double-close returns
/// false" — a parallel test calling `bind` between our two `close`
/// calls would recycle the freed id, making the second close find a
/// different socket and return `true`). The id-reuse behaviour is
/// intentional in the registry; this lock just keeps the tests that
/// observe it deterministic. Tests that only care about a single
/// allocate-or-free-cycle don't need it.
static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

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
    let _guard = REGISTRY_LOCK.lock().unwrap();
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
    let _guard = REGISTRY_LOCK.lock().unwrap();
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
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "127.0.0.1"),
            other => panic!("expected source host, got {:?}", other),
        }
        let (_, payload) = pop(stack);
        match payload {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "hello"),
            other => panic!("expected payload, got {:?}", other),
        }
    }
}

#[test]
fn test_udp_send_to_invalid_socket() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
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
    let _guard = REGISTRY_LOCK.lock().unwrap();
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
            Value::String(s) => assert_eq!(s.as_str_or_empty(), ""),
            other => panic!("expected empty host, got {:?}", other),
        }
        let (_, bytes) = pop(stack);
        match bytes {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), ""),
            other => panic!("expected empty bytes, got {:?}", other),
        }
    }
}

#[test]
fn test_udp_close_double_close() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
    unsafe {
        scheduler_init();

        // Bind, close — should succeed.
        let (sock, _) = bind_succeeds(0);
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(
            matches!(success, Value::Bool(true)),
            "first close on a valid handle should succeed"
        );

        // Closing the *same* handle a second time returns false. The id
        // may eventually be reused for a different socket via the free
        // list, but until that happens the slot is None and close is a
        // no-op.
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(
            matches!(success, Value::Bool(false)),
            "double-close on the same handle should return false"
        );
    }
}

#[test]
fn test_udp_close_invalid_handle() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
    unsafe {
        scheduler_init();

        // A handle that was never allocated returns false.
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, 99_999);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));

        // Negative id is rejected before the `as usize` cast (would
        // otherwise wrap to usize::MAX and benignly miss).
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, -1);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(matches!(success, Value::Bool(false)));
    }
}

#[test]
fn test_udp_receive_from_preserves_non_utf8_bytes() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
    // After the SeqString byte-cleanliness change, non-UTF-8 datagrams
    // round-trip exactly. This is the load-bearing test for binary
    // protocols (OSC int32 / float32 args, DNS records, NTP packets,
    // multicast TLV) — every wire byte must come back intact.
    //
    // Inject raw bytes via `may::net::UdpSocket` directly (rather than
    // `udp_send_to`, which already supports any bytes via the new
    // byte-clean SeqString) and verify the receive-side bytes match.
    unsafe {
        scheduler_init();

        let (recv_sock_id, recv_port) = bind_succeeds(0);

        let sender = MayUdpSocket::bind("0.0.0.0:0").expect("sender bind");
        // Mix of high-bit, NUL, valid-ASCII, and a partial UTF-8 lead.
        // None of this is valid UTF-8 as a whole.
        let payload: &[u8] = &[0xFF, 0xFE, 0x00, b'x', 0xC0, 0x42];
        sender
            .send_to(payload, format!("127.0.0.1:{}", recv_port))
            .expect("raw send");

        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, recv_sock_id);
        let stack = udp_receive_from(stack);

        // Success tuple: ( bytes host port true )
        let (stack, success) = pop(stack);
        assert!(
            matches!(success, Value::Bool(true)),
            "non-UTF-8 datagram should now succeed"
        );
        let (stack, port) = pop(stack);
        assert!(matches!(port, Value::Int(p) if p > 0));
        let (stack, host) = pop(stack);
        match host {
            Value::String(s) => assert_eq!(s.as_str_or_empty(), "127.0.0.1"),
            other => panic!("expected 127.0.0.1 host, got {:?}", other),
        }
        // Critical: the payload bytes round-trip exactly.
        let (_, bytes) = pop(stack);
        match bytes {
            Value::String(s) => assert_eq!(
                s.as_bytes(),
                payload,
                "received bytes must match sent bytes exactly"
            ),
            other => panic!("expected payload, got {:?}", other),
        }
    }
}

#[test]
fn test_udp_receive_from_yields_strand() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
    // Design doc Checkpoint 3: a strand blocked in `receive_from`
    // must yield its OS thread so other strands can run.
    //
    // We spawn a strand that immediately blocks on `recv_from`. From
    // the test thread we wait briefly, send a datagram that wakes
    // it, and join. If `recv_from` were blocking the OS thread, the
    // test thread couldn't make forward progress between spawning
    // and joining (may shares a single OS thread by default in
    // tests). The fact that this test completes within the timeout
    // is the assertion.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    unsafe {
        scheduler_init();

        let (recv_sock_id, recv_port) = bind_succeeds(0);
        let received = Arc::new(AtomicBool::new(false));
        let received_clone = Arc::clone(&received);

        let handle = may::go!(move || {
            let stack = crate::stack::alloc_test_stack();
            let stack = push_int(stack, recv_sock_id);
            let stack = udp_receive_from(stack);
            let (_stack, success) = pop(stack);
            if matches!(success, Value::Bool(true)) {
                received_clone.store(true, Ordering::SeqCst);
            }
        });

        // Give the receive-strand a window to run and reach the block.
        //
        // This sleep is a heuristic, not a synchronisation primitive —
        // we cannot reliably know that the spawned strand has *entered*
        // the blocking poll inside recv_from from outside it. A
        // signal-before-blocking primitive (mpsc, Barrier) would only
        // assert the strand is *about* to call recv_from, not that it
        // has yielded.
        //
        // The kernel UDP buffer is the safety net: even if our
        // datagram arrives before the strand reaches recv_from, the
        // datagram is buffered and the eventual recv_from picks it up.
        // So this test is conservative — it asserts forward progress
        // (the strand wakes, the test completes within the deadline)
        // rather than the strict "recv_from yielded the OS thread"
        // claim. That stricter claim is what the may UdpSocket
        // implementation guarantees by construction (see
        // `may::net::udp::UdpSocket::recv_from` — it dispatches via
        // `yield_with_io` when the non-blocking syscall returns
        // EAGAIN), and we trust that here rather than re-prove it.
        std::thread::sleep(Duration::from_millis(50));

        // Sender uses raw may UdpSocket — same yield contract.
        let sender = MayUdpSocket::bind("0.0.0.0:0").expect("sender bind");
        sender
            .send_to(b"wake-up", format!("127.0.0.1:{}", recv_port))
            .expect("send");

        // Wait (with a hard deadline) for the receive strand to
        // observe the datagram. If recv_from were blocking the OS
        // thread, the may scheduler couldn't schedule the strand
        // back in and this would time out.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && !received.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        handle.join().expect("strand panicked");
        assert!(
            received.load(Ordering::SeqCst),
            "receive strand never observed the datagram — recv_from may have pinned the OS thread"
        );
    }
}

#[test]
fn test_udp_close_during_in_flight_recv() {
    let _guard = REGISTRY_LOCK.lock().unwrap();
    // The close-vs-in-flight-I/O race that motivated moving the
    // registry to Arc<UdpSocket>. Before the fix, close() during a
    // strand's in-flight recv_from would (a) return false even though
    // the user's intent was clearly to close, (b) leave the id leaked
    // on the free list, and (c) restore the socket back into the slot
    // when the recv strand finished, undoing the close.
    //
    // After the fix:
    // - close() returns true immediately (drops the registry's Arc).
    // - The id is freed and immediately recyclable.
    // - The in-flight strand's Arc keeps the OS socket alive until its
    //   recv_from returns; we wake it with a datagram.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    unsafe {
        scheduler_init();

        let (sock_id, port) = bind_succeeds(0);
        let recv_done = Arc::new(AtomicBool::new(false));
        let recv_done_clone = Arc::clone(&recv_done);

        // Strand A: block in recv_from. Holds an Arc<UdpSocket> via the
        // checkout path inside udp_receive_from.
        let handle = may::go!(move || {
            let stack = crate::stack::alloc_test_stack();
            let stack = push_int(stack, sock_id);
            let _ = udp_receive_from(stack);
            recv_done_clone.store(true, Ordering::SeqCst);
        });

        // Give the recv strand time to enter recv_from.
        std::thread::sleep(Duration::from_millis(50));

        // Main thread closes the socket. With the Arc fix this returns
        // true even though strand A still holds a clone.
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock_id);
        let stack = udp_close(stack);
        let (_, close_success) = pop(stack);
        assert!(
            matches!(close_success, Value::Bool(true)),
            "close during in-flight recv should return true (registry slot was occupied)"
        );

        // The recv strand is still blocked — its Arc keeps the OS
        // socket alive. Wake it with a datagram so it can exit.
        let waker = MayUdpSocket::bind("0.0.0.0:0").expect("waker bind");
        waker
            .send_to(b"wake", format!("127.0.0.1:{}", port))
            .expect("waker send");

        // Wait for the recv strand to finish (it will, because we just
        // woke it).
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && !recv_done.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
        handle.join().expect("recv strand panicked");
        assert!(
            recv_done.load(Ordering::SeqCst),
            "recv strand never completed after wake-up datagram"
        );

        // The id should be reusable: a new bind reusing the freed id
        // would prove the free-list got the entry. We don't assert the
        // *exact* id (the free list is LIFO so it usually does match,
        // but that's an implementation detail) — just that allocation
        // still works, i.e. close didn't corrupt the registry.
        let (sock_id2, _) = bind_succeeds(0);
        let stack = crate::stack::alloc_test_stack();
        let stack = push_int(stack, sock_id2);
        let stack = udp_close(stack);
        let (_, success) = pop(stack);
        assert!(matches!(success, Value::Bool(true)));
    }
}

#[test]
fn test_udp_socket_registry_constants() {
    // Documents the limits.
    //
    // MAX_SOCKETS matches `tcp::MAX_SOCKETS`. MAX_READ_SIZE
    // intentionally diverges from TCP: UDP datagrams are protocol-
    // capped at 65,507 bytes (IPv4) / 65,535 bytes (IPv6 base), so we
    // size the recv buffer to the next power of two and not the 1 MB
    // streaming-read cap TCP uses.
    assert_eq!(MAX_SOCKETS, 10_000);
    assert_eq!(MAX_READ_SIZE, 65_536);
}
