# String Byte-Cleanliness

Status: design · 2026-04-26 · supersedes earlier audit-only sketch

## Intent

Make Seq `String` a sequence of arbitrary bytes — no UTF-8 invariant —
so binary protocols (OSC, DNS records, NTP packets, MessagePack,
Protobuf, image formats, raw crypto bytes) can be carried, sent, and
received without resorting to a separate `Bytes` type.

The earlier sketch of "audit at FFI boundaries, add a `string->cstring`
word" was a half-measure: under the existing UTF-8 invariant the
`Value::String(...)` constructor either validates (rejecting binary
input) or doesn't (UB at every later `as_str()` via
`from_utf8_unchecked`). Neither lets a Seq program *construct* a
String containing, say, `0xDC` — which is exactly what an OSC encoder
must do for `float32` arguments. The honest fix is to drop the UTF-8
invariant from the type itself.

## Constraints

- **No new top-level value type.** A separate `Bytes` discriminant
  doubles every byte-aware builtin (concat, send, store-in-list,
  channel-pass) and matches Python 3's str/bytes split, which is the
  canonical "we should have just used one type" lesson. Concatenative
  languages (Forth, Erlang, Lua, Go-style) typically have one
  byte-string type. We follow.
- **No silent semantic change for Seq programs that work today.**
  Every existing string operation continues to accept the inputs it
  accepts today and produce the same output. The change *adds*
  legal inputs (arbitrary bytes), it doesn't *remove* any.
- **Text operations remain text operations.** `string.length` keeps
  its codepoint semantic (not byte length); `string.to-upper` keeps
  Unicode case folding; `regex.*` keeps Unicode-class support. These
  ops validate UTF-8 at their boundary and fail loudly with the
  conventional `(value Bool)` failure tuple on invalid input — same
  pattern as `string->int`, `list.get`, etc.
- **Byte operations accept any bytes.** Concat, byte-length,
  starts-with, contains, equal?, split, channel send, list/map
  storage, network I/O, file I/O of binary content, crypto inputs —
  all become byte-clean.
- **No corner-cut at FFI.** Where we cross into a C-string boundary
  (today: nowhere we ship; potentially future libc-FFI), we add a
  validated boundary word that rejects NULs explicitly rather than
  truncating.

## Approach

### Type-level change

`SeqString` (in `crates/core/src/seqstring.rs`) drops the UTF-8
invariant.

```rust
// before
/// ptr + len must form a valid UTF-8 string
pub fn as_str(&self) -> &str {
    unsafe { from_utf8_unchecked(...) }   // UB if invariant broken
}

// after
/// ptr + len point to an arbitrary byte sequence — no UTF-8 guarantee.
pub fn as_bytes(&self) -> &[u8] { ... }

/// Try to view as a `&str`. Returns `None` if the bytes aren't
/// valid UTF-8. The handful of text-level ops use this and fail
/// loudly on invalid input.
pub fn as_str(&self) -> Option<&str> { ... }
```

Constructors stop validating UTF-8. `arena_string(&str)` /
`global_string(String)` just store the bytes; we additionally provide
`arena_bytes(&[u8])` / `global_bytes(Vec<u8>)` for binary callers.

### Consumer audit

Every internal `as_str()` call in `crates/core` and
`crates/runtime` is reclassified into one of three buckets, with the
appropriate per-site change:

| Bucket | Operations | Change |
|---|---|---|
| **Byte-clean** | `string.concat`, `string.byte-length`, `string.empty?`, `string.equal?`, `string.contains`, `string.starts-with`, `string.split` (byte-delimiter), `string.chomp`, `string.join` (Vec join), `crypto.*`, `encoding.base64-*`, `encoding.hex-*`, `compress.*`, `serialize` (SON), TCP/UDP/HTTP send & receive, file content slurp/spit/append, channel send, list/map storage, variant fields | switch to `as_bytes()` |
| **Text-required** | `string.length` (codepoints), `string.char-at`, `string.substring`, `string.find`, `string.to-upper`, `string.to-lower`, `string.trim`, `string.json-escape`, `string->int`, `regex.*`, `symbol.*`, `os.getenv` / paths, `file.*` paths, `io.write-line` (display), value `Display` impls | call `as_str()`, return failure tuple / -1 / empty on `None` |
| **API-internal** | `SeqString` Display, `Value::Display`, `Value::PartialEq`, SON binary frame headers | mostly switches to `as_bytes()`; a few text-level (Display) keep validating |

Per-site classification lives in inline comments next to each call
site after the audit pass — the source itself becomes the inventory.

### Receive paths

`udp_receive_from` and `tcp_read` drop their `String::from_utf8(buffer)`
validation. Bytes go straight into a `SeqString` via the new
`global_bytes` constructor. Both ops can now serve binary protocols.

### String literals

The tokenizer's `unescape_string` already supports `\xFF` and `\0` —
we verify this (and fix it if missing) so Seq source can construct
arbitrary byte strings inline. `"\x43\xDC\x00\x00"` produces a 4-byte
String containing the IEEE-754 big-endian bytes for `440.0`.

### New builtins for binary construction

Phase B's OSC encoder needs a way to convert Int / Float values into
their big-endian byte representations. Two minimal builtins:

```
int.to-bytes-be   ( Int -- String )    # 8-byte big-endian i64
float.to-bytes-be ( Float -- String )  # 8-byte big-endian f64
```

OSC specifically wants 4-byte int32 / float32, but those are bit-trims
of the 8-byte versions. Adding both 4-byte and 8-byte variants is a
later decision based on what the encoder actually needs; for the first
cut, the 8-byte versions plus a `string.substring` byte-slicing op (or
manual indexing) is enough. We commit to landing at least the 4-byte
variants if the OSC encoder reads cleaner with them.

## Domain Events

- **Trigger:** Phase B (OSC encoder) needs to construct datagram
  payloads containing arbitrary bytes; `udp.send-to` needs to accept
  them.
- **Output:** `StringByteCleanLanded { invariant_dropped: true,
  byte_clean_paths: N, text_paths_validated: M }` — every path in the
  inventory is either byte-clean or explicitly UTF-8-validating; the
  round-trip sentinel test (with `0x00`, `0xDC`, `0xFF`, valid UTF-8)
  passes through every public path.
- **Downstream now unblocked:**
  - `OscEncoderProven` (POC Phase B) — Seq-side encoder + send.
  - DNS / NTP / multicast / syslog clients in user code.
  - Binary file parsing (images, archives, compiled formats).
  - Crypto primitives carrying arbitrary key/hash bytes
    without a base64/hex round-trip.

## Checkpoints

1. **Inventory is captured as inline comments** next to every site
   that crossed the byte/text boundary in the audit pass.
2. **Round-trip sentinel test passes** for the byte sequence
   `[0x00, 0xDC, b'x', 0xFF, partial-UTF-8]` through every public
   path that takes or returns a String.
3. **`Value::String` constructor stops validating UTF-8** —
   construction with arbitrary bytes succeeds.
4. **TCP `read` and UDP `receive_from` return raw bytes** — neither
   path validates UTF-8 anymore. The old "non-UTF-8 → false" tests
   are inverted: those bytes now arrive intact.
5. **Text-required operations fail loudly** on invalid UTF-8 input,
   using the `(value Bool)` failure pattern. New tests cover each
   operation with an invalid-UTF-8 input.
6. **String literal `"\xFF"` produces a 1-byte string.** If the
   tokenizer doesn't already support hex escapes, it does after this.
7. **OSC encoder works for `,if` and `,f` messages** — Phase B
   compiles and round-trips through `udp.send-to`.
8. **`just ci` is green** end-to-end — all stdlib, examples, integration
   tests pass.

## Out of scope

- Adding a separate `Bytes` value type. Decision recorded above.
- Codepoint-by-codepoint mutation APIs (insert-at, delete-at). Out of
  scope; existing operations remain functional (return-new-String).
- Locale-aware case folding / collation. Rust's standard
  `to_uppercase`/`to_lowercase` are Unicode-correct without locale
  awareness; that's what we already ship.
- Unicode normalization (NFC/NFD). Out of scope; not needed for any
  current Seq workload.
- Path encoding on Windows. Today Seq paths are `&str`-validated
  (UTF-8 or fail). On Windows non-UTF-16 path components would
  require `OsStr`-aware APIs; defer until a Windows user reports it.
