# UDP Runtime Support

Status: design · 2026-04-26 · issue [#433]

## Intent

Add UDP socket primitives to the Seq runtime. UDP unblocks a long
list of common protocols — DNS resolvers, NTP clients, multicast
discovery, syslog, OSC for live-coding, QUIC's underlying transport,
most game-network patterns. The near-term motivating use case is the
Seq → OSC → Csound POC documented in `LIVE_CODING_CSOUND_POC.md`, but
the API design serves all of those, not just OSC. UDP is worth landing
even if the music POC fails.

## Constraints

- **TCP API stays unchanged.** UDP is purely additive.
- **Mirror `tcp.*` shape.** Sockets are `Int` handles in a registry;
  every operation returns a success `Bool` on top so callers can
  `[ ... ] [ ... ] if`. Same `MAX_SOCKETS = 10_000` and per-datagram
  `MAX_READ_SIZE = 1 MB` caps as `tcp.*`.
- **Strands must yield while waiting on `recv_from`.** Use
  `may::net::UdpSocket` for coroutine-aware blocking, same pattern
  `tcp.read` uses today.
- **Out of scope first cut:** multicast, broadcast, IPv6-specific
  ergonomics. IPv6 should work transparently for IPv6-literal host
  strings, but no `udp.join-multicast-group` etc. yet.
- **Don't over-abstract on day one.** `SocketRegistry<T>` exists in
  `tcp.rs`. Duplicate for UDP and refactor into a shared module
  *after* both are working — UDP has no listener/stream split, so
  forcing a shared abstraction now risks the wrong shape.

## Approach

`crates/runtime/src/udp.rs` mirrors `tcp.rs`. Public C-ABI surface:

```
udp.bind          ( Int                   -- a Int Int Bool )
                    ( requested-port -- socket bound-port success )
udp.send-to       ( a String String Int Int -- a Bool )
                    ( bytes host port socket -- success )
udp.receive-from  ( a Int -- a String String Int Bool )
                    ( socket -- bytes host port success )
udp.close         ( a Int -- a Bool )
                    ( socket -- success )
```

`udp.bind` returns *both* the socket handle and the actual bound port
(an extension over the issue's draft). Without that, the standard
"bind to port 0, OS picks one" idiom is unusable — and the loopback
test the issue specifies needs the assigned port to send to. For
non-zero requests, the returned port equals the request.

Wire-up follows the existing pattern: C-ABI exports in
`crates/runtime/src/lib.rs`, type signatures in
`crates/compiler/src/builtins/` (likely a new `udp.rs` sub-module
alongside `tcp.rs`), AST validation entry in `ast/program.rs`, codegen
runtime symbol mapping in `codegen/runtime/`. Same five-touchpoint
recipe as TCP.

## Domain Events

- **Input:** `UdpRuntimeRequested { source: issue#433 }`
- **Output:** `UdpLanded { has_tests: true, has_examples: false }`
  — runtime + builtins + codegen wired; loopback test green.
- **Downstream that's now unblocked:**
  - `OscEncoderProven` (POC phase B) — Seq-side encoder + `udp.send-to`.
  - `CsoundExampleRuns` (POC phase C) — full live-coding loop.
  - DNS / NTP / multicast / syslog clients in user code (not in this
    repo unless they want to be).
- **Out of scope:** `OscStdlibPromoted`, `MulticastApi`, `Ipv6Api`.

## Checkpoints

1. **Loopback round-trip test passes.** Bind to `127.0.0.1:0`, get the
   assigned port, send a payload from a second socket, receive it,
   assert byte-exact match including source host/port. Lives in
   `crates/runtime/src/udp/tests.rs`.
2. **Negative tests pass.** Invalid port (negative, > 65535) returns
   `(0, 0, false)`. `send-to` on closed socket → `false`.
   `receive-from` on closed socket → `("", "", 0, false)`.
3. **Strand yield is real, not blocking.** A two-strand test where
   strand A blocks on `udp.receive-from` and strand B runs to
   completion proves A doesn't pin the OS thread. (Mirror whatever
   `tcp.rs` does here, if any.)
4. **`just ci` green** — no regressions in TCP tests, examples, or
   integration suite.
5. **Live-coding POC phase B begins** — separate work, but its first
   commit should rely only on the merged `udp.*` words and a Seq-side
   OSC encoder. No further runtime additions.

## Open question

Should `udp.send-to`'s payload be `String` or a future `Bytes` type?
Today Seq's `String` is byte-addressable (`string.byte-length`,
`string.char-at` returns a byte) and OSC encoders will produce
arbitrary bytes via string concatenation. So `String` works. Revisit
only if a real protocol needs to send literal NUL bytes that Seq
strings can't currently carry — and that's its own design problem,
not a UDP one.
