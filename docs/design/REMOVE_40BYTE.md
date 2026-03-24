# Remove 40-Byte StackValue Code

## Intent

The tagged-ptr (8-byte) representation is now the default across all three
crates and all benchmarks validate it (433 integration tests, compute
competitive with Go/Rust, collections 15,888x faster with COW). The old
40-byte StackValue code (`stack_old.rs`, `tagged_stack_old.rs`) and its
feature-flag dispatch layer are dead weight:

- 1,684 lines of unused code
- 13 `cfg(feature = "tagged-ptr")` branches in production code
- 28 `self.tagged_ptr` checks in the compiler codegen
- `ci-tptr` as a separate CI target (should just be `ci`)
- Every layout helper branches on `self.tagged_ptr` at runtime

Removing it simplifies maintenance and makes every future codegen change
smaller (one path instead of two).

## Constraints

- **No behavioral change** — the runtime, compiler, and generated IR
  stay identical to what tagged-ptr produces today
- **All 438 tests pass** — no regressions
- **Benchmarks unchanged** — performance stays the same
- **Don't touch value.rs** — the `Value` enum is still 40 bytes internally
  (Rust representation). Only the stack encoding changes.

## Approach

1. **Delete old files**: `stack_old.rs`, `tagged_stack_old.rs`
2. **Inline dispatchers**: `stack.rs` and `tagged_stack.rs` become the
   actual implementation (move `stack_new.rs` → `stack.rs`, etc.)
3. **Remove feature flags**: Delete `tagged-ptr` from all three
   `Cargo.toml` files. Remove `cfg` branches from `value.rs`,
   `float_ops.rs`, `arithmetic.rs`, `weave.rs`.
4. **Simplify codegen**: Remove `self.tagged_ptr` field and all branches
   in `layout.rs`. Each helper becomes a single path (the current
   tagged-ptr path). Remove `codegen_default()` test helpers.
5. **Simplify CI**: Remove `just ci-tptr`. The default `just ci` is
   the only path.
6. **Update docs**: `ARCHITECTURE.md` describes the 8-byte tagged
   pointer as the stack model (not "40-byte tagged values").

## Checkpoints

1. `cargo build --release` succeeds with no feature flags
2. `just ci` passes (438 tests)
3. `grep -r "tagged.ptr\|tagged_ptr\|40.byte\|stack_old\|stack_new" crates/`
   returns zero hits (excluding comments/docs)
4. Benchmarks match current numbers
