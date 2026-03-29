# Migrating to Seq 5.0

## Breaking Changes

### `list.push!` removed

`list.push!` has been removed. Use `list.push` instead — it now has the
same copy-on-write fast path internally. When the list is sole-owned
(the common case in loops), `list.push` mutates in place automatically.
When shared (after `dup`), it clones.

**Before (v4.x):**
```seq
list.make 1 list.push! 2 list.push! 3 list.push!
```

**After (v5.0):**
```seq
list.make 1 list.push 2 list.push 3 list.push
```

No performance difference — `list.push` uses the same `peek_heap_mut`
fast path that `list.push!` used.
