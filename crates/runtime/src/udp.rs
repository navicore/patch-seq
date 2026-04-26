//! UDP Socket Operations for Seq
//!
//! Provides non-blocking UDP datagram operations using May's
//! coroutine-aware I/O. `udp.receive-from` yields the strand
//! while waiting for a datagram instead of blocking the OS thread.
//!
//! These functions are exported with C ABI for LLVM codegen.
//!
//! ## Byte-cleanliness limitation
//!
//! Payloads are carried as Seq `String` values, which are UTF-8 by
//! invariant (`SeqString::as_str` uses `from_utf8_unchecked`). UDP
//! send rejects nothing here, but a String can only have been
//! constructed in the first place if it was valid UTF-8 — so the
//! effective sendable payload is "any UTF-8 byte sequence." UDP
//! receive validates the same way: non-UTF-8 datagrams are dropped
//! with a `false` success flag.
//!
//! Most binary protocols (DNS records, NTP packets, OSC int32/float32
//! arguments, multicast TLV) include bytes that aren't valid UTF-8.
//! Closing this gap is tracked in
//! `docs/design/STRING_BYTE_CLEANLINESS.md` — the OSC encoder phase
//! of the live-coding POC is the canonical failing case that will
//! drive that audit. UDP itself stays as-is.

use crate::stack::{Stack, pop, push};
use crate::value::Value;
use may::net::UdpSocket;
use std::sync::Mutex;

// Maximum number of concurrent sockets to prevent unbounded growth.
// Same cap as `tcp.rs`.
const MAX_SOCKETS: usize = 10_000;

// Maximum bytes to read per datagram.
//
// UDP datagrams are protocol-capped at 65,507 bytes for IPv4 (the
// `udp.length` header is 16-bit, minus IP+UDP headers), and 65,535
// for IPv6 base-headered datagrams. We use the next power of two
// (65,536) as the receive buffer size — anything larger cannot
// arrive on the wire, so allocating more would be pure waste.
//
// This intentionally diverges from `tcp.rs`'s 1 MB cap, which makes
// sense for streaming reads but not for one-datagram-per-call recv.
const MAX_READ_SIZE: usize = 65_536;

// Socket registry with ID reuse via free list.
//
// This is a deliberate copy of the registry in `tcp.rs`. The two will
// be lifted into a shared `socket_registry` module once both are
// landed and the right abstraction shape is obvious — TCP has a
// listener/stream split and UDP doesn't, so forcing a generic
// abstraction now risks the wrong shape.
struct SocketRegistry<T> {
    sockets: Vec<Option<T>>,
    free_ids: Vec<usize>,
}

impl<T> SocketRegistry<T> {
    const fn new() -> Self {
        Self {
            sockets: Vec::new(),
            free_ids: Vec::new(),
        }
    }

    fn allocate(&mut self, socket: T) -> Result<i64, &'static str> {
        if let Some(id) = self.free_ids.pop() {
            self.sockets[id] = Some(socket);
            return Ok(id as i64);
        }
        if self.sockets.len() >= MAX_SOCKETS {
            return Err("Maximum socket limit reached");
        }
        let id = self.sockets.len();
        self.sockets.push(Some(socket));
        Ok(id as i64)
    }

    fn get_mut(&mut self, id: usize) -> Option<&mut Option<T>> {
        self.sockets.get_mut(id)
    }

    fn free(&mut self, id: usize) {
        if let Some(slot) = self.sockets.get_mut(id)
            && slot.is_some()
        {
            *slot = None;
            self.free_ids.push(id);
        }
    }
}

static SOCKETS: Mutex<SocketRegistry<UdpSocket>> = Mutex::new(SocketRegistry::new());

/// Bind a UDP socket to a local port.
///
/// Stack effect: ( port -- socket bound-port Bool )
///
/// Binds to `0.0.0.0:port`. `port=0` lets the OS pick a free port; the
/// returned `bound-port` is the actual bound port (equal to `port` if
/// non-zero). On failure pushes `(0, 0, false)`.
///
/// # Safety
/// Stack must have an Int (port) on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_udp_bind(stack: Stack) -> Stack {
    unsafe {
        let (stack, port_val) = pop(stack);
        let port = match port_val {
            Value::Int(p) => p,
            _ => return push_bind_failure(stack),
        };

        if !(0..=65535).contains(&port) {
            return push_bind_failure(stack);
        }

        let addr = format!("0.0.0.0:{}", port);
        let socket = match UdpSocket::bind(&addr) {
            Ok(s) => s,
            Err(_) => return push_bind_failure(stack),
        };

        // Capture the actual bound port before the registry takes ownership.
        let bound_port = match socket.local_addr() {
            Ok(addr) => addr.port() as i64,
            Err(_) => return push_bind_failure(stack),
        };

        let mut sockets = SOCKETS.lock().unwrap();
        match sockets.allocate(socket) {
            Ok(socket_id) => {
                let stack = push(stack, Value::Int(socket_id));
                let stack = push(stack, Value::Int(bound_port));
                push(stack, Value::Bool(true))
            }
            Err(_) => push_bind_failure(stack),
        }
    }
}

unsafe fn push_bind_failure(stack: Stack) -> Stack {
    unsafe {
        let stack = push(stack, Value::Int(0));
        let stack = push(stack, Value::Int(0));
        push(stack, Value::Bool(false))
    }
}

/// Send a datagram to a host:port from a bound UDP socket.
///
/// Stack effect: ( bytes host port socket -- Bool )
///
/// Pops `socket`, `port`, `host`, `bytes` (in that order, so `bytes`
/// is below all of them on entry). Returns `false` on type mismatch,
/// invalid socket, address-resolution failure, or send error.
///
/// # Safety
/// Stack must have Int (socket), Int (port), String (host),
/// String (bytes) — top-down — on entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_udp_send_to(stack: Stack) -> Stack {
    unsafe {
        let (stack, socket_val) = pop(stack);
        // Reject negative ids before the `as usize` cast: a negative
        // i64 wraps to usize::MAX, which would silently fall through
        // to a benign `None` lookup. Catching it here is a clearer
        // signal than the indirect not-found path.
        let socket_id = match socket_val {
            Value::Int(id) if id >= 0 => id as usize,
            _ => return push(stack, Value::Bool(false)),
        };

        let (stack, port_val) = pop(stack);
        let port = match port_val {
            Value::Int(p) if (0..=65535).contains(&p) => p,
            _ => return push(stack, Value::Bool(false)),
        };

        let (stack, host_val) = pop(stack);
        let host = match host_val {
            Value::String(s) => s,
            _ => return push(stack, Value::Bool(false)),
        };

        let (stack, bytes_val) = pop(stack);
        let bytes = match bytes_val {
            Value::String(s) => s,
            _ => return push(stack, Value::Bool(false)),
        };

        // Pull the socket out of the registry so we don't hold the lock during I/O.
        let socket = {
            let mut sockets = SOCKETS.lock().unwrap();
            match sockets.get_mut(socket_id).and_then(|slot| slot.take()) {
                Some(s) => s,
                None => return push(stack, Value::Bool(false)),
            }
        };

        let addr = format!("{}:{}", host.as_str(), port);
        let result = socket.send_to(bytes.as_str().as_bytes(), &addr);

        // Restore the socket regardless of send result.
        {
            let mut sockets = SOCKETS.lock().unwrap();
            if let Some(slot) = sockets.get_mut(socket_id) {
                *slot = Some(socket);
            }
        }

        push(stack, Value::Bool(result.is_ok()))
    }
}

/// Receive one datagram from a UDP socket.
///
/// Stack effect: ( socket -- bytes host port Bool )
///
/// Yields the strand until a datagram arrives. On failure pushes
/// `("", "", 0, false)` — invalid socket, recv error, datagram larger
/// than `MAX_READ_SIZE`, or non-UTF-8 payload (see module doc).
///
/// # Safety
/// Stack must have an Int (socket) on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_udp_receive_from(stack: Stack) -> Stack {
    unsafe {
        let (stack, socket_val) = pop(stack);
        let socket_id = match socket_val {
            Value::Int(id) if id >= 0 => id as usize,
            _ => return push_receive_failure(stack),
        };

        let socket = {
            let mut sockets = SOCKETS.lock().unwrap();
            match sockets.get_mut(socket_id).and_then(|slot| slot.take()) {
                Some(s) => s,
                None => return push_receive_failure(stack),
            }
        };

        let mut buffer = vec![0u8; MAX_READ_SIZE];
        let recv_result = socket.recv_from(&mut buffer);

        {
            let mut sockets = SOCKETS.lock().unwrap();
            if let Some(slot) = sockets.get_mut(socket_id) {
                *slot = Some(socket);
            }
        }

        let (size, src) = match recv_result {
            Ok(pair) => pair,
            Err(_) => return push_receive_failure(stack),
        };

        buffer.truncate(size);
        let payload = match String::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => return push_receive_failure(stack),
        };

        let stack = push(stack, Value::String(payload.into()));
        let stack = push(stack, Value::String(src.ip().to_string().into()));
        let stack = push(stack, Value::Int(src.port() as i64));
        push(stack, Value::Bool(true))
    }
}

unsafe fn push_receive_failure(stack: Stack) -> Stack {
    unsafe {
        let stack = push(stack, Value::String("".into()));
        let stack = push(stack, Value::String("".into()));
        let stack = push(stack, Value::Int(0));
        push(stack, Value::Bool(false))
    }
}

/// Close a UDP socket and free its handle.
///
/// Stack effect: ( socket -- Bool )
///
/// Returns `true` if the socket existed (and is now closed), `false`
/// if the handle was already invalid.
///
/// ## Race with in-flight I/O
///
/// `send_to` and `receive_from` use a take-and-restore pattern: they
/// remove the socket from the registry while running I/O, so the
/// registry lock isn't held across syscalls. If a strand calls
/// `close` on a handle whose socket is currently *taken* by another
/// strand's in-flight I/O, the slot looks empty (`is_some()` is
/// `false`), and `close` returns `false` as if the handle were
/// invalid — even though the I/O strand will restore the socket
/// moments later. This matches `tcp_close`'s behaviour and is
/// considered acceptable for the current concurrency model: callers
/// shouldn't be racing close against I/O on the same handle. If we
/// ever support that pattern, the registry needs a richer state
/// machine (taken-but-marked-for-close).
///
/// # Safety
/// Stack must have an Int (socket) on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_udp_close(stack: Stack) -> Stack {
    unsafe {
        let (stack, socket_val) = pop(stack);
        let socket_id = match socket_val {
            Value::Int(id) if id >= 0 => id as usize,
            _ => return push(stack, Value::Bool(false)),
        };

        let mut sockets = SOCKETS.lock().unwrap();
        let existed = sockets
            .get_mut(socket_id)
            .map(|slot| slot.is_some())
            .unwrap_or(false);

        if existed {
            sockets.free(socket_id);
        }

        push(stack, Value::Bool(existed))
    }
}

// Public re-exports with short names for in-module callers — the
// `tests` submodule below imports them via `use super::*`. The
// crate-root re-exports in `lib.rs` are the linker-facing aliases.
pub use patch_seq_udp_bind as udp_bind;
pub use patch_seq_udp_close as udp_close;
pub use patch_seq_udp_receive_from as udp_receive_from;
pub use patch_seq_udp_send_to as udp_send_to;

#[cfg(test)]
mod tests;
