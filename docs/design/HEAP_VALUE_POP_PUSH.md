# Eliminate Arc<Value> Pop/Push Overhead for Heap Types

## Intent

Every runtime operation on a heap value (String, Variant, Map, Channel, etc.)
pays an `Arc::new` + `Arc::try_unwrap` tax on each pop/push cycle — even when
the value is sole-owned on the stack. This is the dominant cost for operations
that read a value, transform it, and write it back.

`list.push!` already demonstrates the fix: read the raw `u64` tagged pointer
from the stack, do `Arc::from_raw` + `Arc::get_mut` in place, then `forget`
to leave the pointer untouched. The goal is to generalize this pattern so
any runtime operation on a sole-owned heap value can skip the alloc/dealloc
cycle.

## Constraints

- **No change to language semantics** — `dup` still creates a shared reference
  (Arc refcount bump), `drop` still decrements. Only sole-owned values get the
  fast path.
- **Must remain sound under May's M:N scheduler** — strand stacks are
  single-threaded, but values can be sent across strands via channels. The
  `Arc::get_mut` check is the safety guard (fails if refcount > 1).
- **No change to `Value` enum or `StackValue` encoding** — this is a
  runtime-internal optimization, not a representation change.
- **Cross-strand sharing still works** — `chan.send` clones the value; the
  receiver gets its own Arc. This is unchanged.
- **Out of scope**: arena-allocated heap objects, NaN-boxing, changes to
  codegen or LLVM IR. This is purely a Rust runtime change.

## Approach

Add `pop_heap_mut` and `peek_heap_mut` primitives to `crates/core/src/stack.rs`
that give `&mut Value` access to a heap-typed stack slot without the Arc
alloc/dealloc cycle:

```rust
/// Peek at the top heap value mutably, without popping.
/// Returns None if the value is inline (Int/Bool) or shared (refcount > 1).
pub unsafe fn peek_heap_mut(stack: Stack) -> Option<&mut Value> {
    let sv = *stack.sub(1);
    if is_tagged_int(sv) || sv == TAG_FALSE || sv == TAG_TRUE {
        return None;
    }
    // Arc::get_mut succeeds only if refcount == 1
    let arc_ptr = sv as *mut ArcInner<Value>;  // implementation detail
    // ... get &mut Value if sole-owned
}
```

Then migrate hot-path runtime operations to use the new primitives instead
of `pop` + transform + `push`. Priority order by call frequency:

1. **list.push** — already done via `list.push!` pattern, generalize
2. **map.set / map.get** — pop map, lookup/insert, push map
3. **variant.append** — pop variant, extend fields, push variant
4. **string operations** — concat, substring, etc. that produce new strings

For each operation, the pattern is:
- Try `peek_heap_mut` — if sole-owned, mutate in place, return stack unchanged
- Fall back to `pop` + transform + `push` when shared

## Checkpoints

1. **`peek_heap_mut` primitive exists** in `crates/core/src/stack.rs` with unit tests
2. **`list.push` uses it** — `list.push!` removed as separate builtin (regular
   `list.push` gets the fast path automatically)
3. **All 438+ tests pass** — no regressions
4. **build-100k stays at ~3ms** — no regression from consolidating list.push!/list.push
5. **map.set benchmark** — if one exists, verify no regression; if not, add one
6. **No new builtins** — the optimization is invisible to Seq code
7. **`list.push!` removed** — breaking change, requires major version bump
   (v5.0) with migration guide (MIGRATION_5.0.md)
