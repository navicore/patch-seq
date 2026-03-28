# Migrate Runtime Operations to peek_heap_mut

## Intent

`peek_heap_mut` / `peek_heap_mut_second` (added in #368) let runtime
operations read and mutate a sole-owned heap value on the stack without
the `Arc::new` / `Arc::try_unwrap` cycle that `pop` + `push` requires.
`list.push` already uses this — apply the same pattern to other
operations that pop a heap value, transform it, and push it back.

## Candidates

Ranked by expected impact (frequency x cost of the current cycle):

| Operation | Stack effect | Pattern | Benefit |
|-----------|-------------|---------|---------|
| **map.set** | `( Map K V -- Map )` | pop 3, insert, push | High — map-heavy code (JSON, config) |
| **map.remove** | `( Map K -- Map )` | pop 2, remove, push | Medium |
| **variant.append** | `( Variant T -- Variant )` | pop 2, extend, push | Medium — used by stdlib builders |
| **list.set** | `( List Int T -- List )` | pop 3, replace, push | Medium |
| **string.concat** | `( Str Str -- Str )` | pop 2, format, push | Low — produces new string regardless |

**Skip**: `string.concat` always allocates a new string, so avoiding the
Arc cycle on inputs saves little. Read-only operations (`list.get`,
`map.get`, `variant.tag`, `variant.field-at`) pop but don't push back
the same type — less benefit and more complex to optimize.

## Constraints

- **Same constraints as #368** — no semantic changes, must be sound under
  May, no change to Value or StackValue encoding.
- **Each migration is independent** — can land one at a time.
- **Fallback required** — every operation must fall through to the
  existing pop/push path when the value is shared (refcount > 1).

## Approach

For each candidate, the pattern is identical to what `list.push` does:

```rust
// Fast path: mutate in place if sole-owned
if let Some(Value::Map(map)) = peek_heap_mut_third(stack)  // or appropriate depth
{
    // pop only the non-map args, mutate map in place
    return stack;
}
// Slow path: pop all, transform, push
```

For `map.set` (`( Map K V -- Map )`), the map is at sp-3. We'd need
`heap_value_mut(stack.sub(3))` — the generic `heap_value_mut` already
supports arbitrary depths.

Order of work:
1. `map.set` — highest impact, straightforward
2. `variant.append` — same shape as list.push
3. `map.remove` — similar to map.set
4. `list.set` — slightly trickier (index validation before mutation)

## Checkpoints

1. **Each operation: all existing tests pass** after migration
2. **Clippy + fmt clean** after each change
3. **Benchmark**: if a map-heavy benchmark exists, verify no regression;
   if not, consider adding one (map build-100k analog)
4. **No new builtins** — optimizations are invisible to Seq code
