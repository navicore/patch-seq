# String Byte-Cleanliness Audit

Status: design · 2026-04-26 · follow-up from `UDP_RUNTIME.md` open question

## Intent

Confirm — and where necessary, enforce — that a Seq `String` carries
arbitrary bytes including NUL (`0x00`) without silent truncation,
across every public runtime path that accepts or produces a String.
Most likely Seq is already byte-clean internally (`SeqString` is
Rust-side, and Rust `String` permits interior NULs; `string.byte-length`
and byte-indexed `string.char-at` already imply bytes-not-codepoints).
The risk is at FFI boundaries that use `CString` or rely on C-strlen
semantics — those silently lose data.

This is a follow-up audit, not a feature: the goal is documenting the
current behaviour and closing any gaps, not adding new types or
significantly widening the API.

## Constraints

- **Do not introduce a separate `Bytes` type.** That tradeoff was
  considered in `UDP_RUNTIME.md` and rejected in favour of keeping
  `String` byte-clean. Reopening that decision is its own design
  problem, not part of this audit.
- **Do not widen public API ergonomically.** At most one boundary
  word — something like `string->cstring` — added at C-FFI sites
  that genuinely cannot carry interior NULs. That word should
  *fail loudly* (not truncate) on NUL-bearing input.
- **Existing public APIs must keep their current signatures.** If
  `file.slurp` is currently NUL-safe, it stays. If it's not, fix
  the implementation, not the signature.
- **Out of scope:** Unicode normalization questions, grapheme
  clusters, encoding conversions. The audit is byte-level only.

## Approach

Three phases, each independently shippable:

1. **Inventory.** List every runtime function that accepts or
   produces a Seq String. Group by category: I/O (`file.*`,
   `io.*`), networking (`tcp.*`, `udp.*`, `http.*`), encoding
   (`encoding.*`, `crypto.*`), collections (`list.*`, `map.*`,
   `chan.*`), string ops (`string.*`), FFI (`seq_ffi_*` wrappers).
   Mark each as **likely-clean** (Rust-only path), **suspect**
   (crosses FFI), or **unknown** (needs reading).

2. **Test fixture.** Add a Rust integration test that round-trips
   a fixed sentinel — e.g. `"hello\x00middle\x00end"` — through
   every public path in the inventory. Failures point at exactly
   which functions truncate.

3. **Close gaps.** For each failing path, either fix the
   implementation (preferred) or add a boundary word that rejects
   NUL-bearing input with a clear error. Document each FFI site's
   policy in the function's doc comment.

## Domain Events

- **Trigger:** UDP/OSC work, or any user filing a bug like "my
  binary protocol's payload is being truncated".
- **Output:** `StringByteCleanlinessVerified { paths_audited: N,
  gaps_found: K, gaps_closed: K }` — the audit ends with either
  zero gaps or a documented fix per gap.
- **Downstream now confidently unblocked:** BSON / MessagePack /
  Protobuf encoders in user code, binary file digests, OSC
  encoders that pad with NULs (motivating case), image and
  archive parsing, network protocols with binary framing.
- **Out of scope:** any work that requires a Bytes type — that's
  a separate design.

## Checkpoints

1. **Inventory exists** in `docs/STRING_BYTE_INVENTORY.md` (or
   inline in this doc's appendix), classifying every public
   String-touching runtime function.
2. **Round-trip test passes** for the sentinel through every path
   in the inventory.
3. **Each FFI boundary documented.** Function doc comments say
   either "byte-clean — interior NULs preserved" or "rejects NULs
   — use this when crossing C-string boundaries".
4. **`just ci` green** with the audit's tests in the regular suite,
   so future regressions are caught.
5. **Open question from `UDP_RUNTIME.md` resolved.** That doc's
   payload-type concern can be closed once the audit confirms UDP
   send/receive preserves NULs.

## Open question

Whether to publish the inventory as a permanent doc or fold it
into runtime-source doc comments. Lean toward doc comments — they
travel with the code and don't drift — with a short index in
`STRING_BYTE_CLEANLINESS.md` that just lists which categories
were audited and when.
