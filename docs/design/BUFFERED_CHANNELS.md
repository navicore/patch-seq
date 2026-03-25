# Buffered Channels

## Intent

Channel throughput is 400x slower than Python asyncio and 3,300x slower
than Go (#306). Pingpong latency is competitive (269ms vs 206ms Go), so
the per-message overhead is reasonable — the problem is throughput under
contention. The fanout benchmark (1 producer → 100 workers → 1M messages)
is where the gap shows: 551ms Seq vs 249ms Go vs 29ms Rust.

The goal is channel throughput within 5x of Go for fan-out patterns.

## Current Implementation

Channels use May's `mpmc::channel()`:

```rust
pub struct ChannelData {
    pub sender: mpmc::Sender<Value>,
    pub receiver: mpmc::Receiver<Value>,
}
```

Each `chan.send`:
1. Clone the `Value` (8-byte tagged pointer, plus deep clone for heap types like strings/variants)
2. Acquire internal lock in May's MPMC queue
3. Enqueue the value
4. Wake a waiting receiver (if any)
5. Return success flag

Each `chan.receive`:
1. Acquire internal lock
2. Dequeue a value (or cooperatively block if empty)
3. Return value + success flag

**Root causes from #306**:
- `Value` clone per message (heap types require deep clone)
- Lock contention: every send/receive acquires May's internal lock
- No batching: each message is an individual lock acquire/release
- Cooperative yield overhead between operations

## Constraints

- **Channel semantics unchanged** — `chan.make`, `chan.send`, `chan.receive`,
  `chan.close` keep their current stack effects.
- **Must work with May scheduler** — Channels must cooperatively yield
  when blocking (not spin or OS-block).
- **MPMC required** — Multiple senders and multiple receivers must work.
  The fanout benchmark depends on this.
- **Value encoding is now 8-byte tagged pointers** — Stack values are
  `u64`. This design optimizes the channel machinery independent of
  value encoding.

## Approach

### Step 1: Bounded Ring Buffer with Lock-Free Fast Path

Replace May's MPMC with a custom bounded ring buffer:

```rust
pub struct ChannelData {
    buffer: Box<[UnsafeCell<MaybeUninit<Value>>]>,
    head: AtomicUsize,    // next slot to read
    tail: AtomicUsize,    // next slot to write
    capacity: usize,
    closed: AtomicBool,
    // Fallback for blocking when buffer is full/empty
    waiters: may::sync::Mutex<WaiterList>,
}
```

**Fast path** (buffer not full/empty): CAS on head/tail, no lock.
**Slow path** (buffer full or empty): park on May-aware condvar.

Default buffer size: 256 messages. Configurable via `chan.make-buffered`.

### Step 2: Batch Send/Receive

Add optional batch operations for high-throughput patterns:

```seq
values chan chan.send-batch    # ( List Channel -- Int ) returns count sent
chan n chan.receive-batch      # ( Channel Int -- List Int ) returns values and count
```

These acquire the lock once and transfer N messages, amortizing the
synchronization cost.

### Step 3: Integer Fast Path

When the compiler knows a channel carries only integers (from type
inference), generate specialized send/receive that passes `i64` directly
instead of going through the `Value` encoding. This avoids heap allocation entirely.

```rust
// Specialized: no Value wrapping
pub fn send_int(chan: &IntChannelData, value: i64) -> bool { ... }
```

This is optional and can be deferred — the ring buffer alone should
provide the bulk of the improvement.

## Alternative Considered: crossbeam-channel

`crossbeam-channel` is the standard Rust high-performance MPMC channel.
It's lock-free, well-optimized, and battle-tested. However:
- It blocks OS threads on receive, not May coroutines
- Wrapping it with May-aware parking is possible but adds complexity
- May's own MPMC is already coroutine-aware

The custom ring buffer approach gives us control over the blocking
strategy while keeping May compatibility.

## What This Does NOT Fix

- **Value clone cost** — Each message still clones the value. Integer
  messages are cheap (inline `u64`), but heap types (strings, variants)
  require deep cloning.
- **Strand spawn overhead** — Skynet is slow because of mmap per strand,
  not channel throughput.
- **Single-consumer patterns** — Pingpong is already competitive; this
  targets fan-out/fan-in under contention.

## Checkpoints

1. **fanout under 100ms** (currently 551ms) — primary target
2. **pingpong stays under 300ms** — no regression on latency
3. **Existing channel tests pass** — `cargo test --all`
4. **New benchmark**: 1 producer → 1 consumer, 10M integer messages,
   target under 500ms
5. **May scheduler compatibility** — channels must cooperatively yield,
   never spin-wait or OS-block
