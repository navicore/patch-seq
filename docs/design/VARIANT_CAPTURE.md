# Variant Capture in Closures

## Intent

Enable closures to capture `Variant` values (lists, maps, unions) from
the caller's stack. This is the actual blocker for rewriting the 7
remaining manual-recursive loops in sss.seq — not `integer-fold`, not
multi-value capture, not any missing combinator. The typechecker and
auto-capture mechanism already handle Variants correctly; the codegen
rejects them with "Variant captures are not yet supported."

## What Blocks sss.seq

Every remaining manual loop needs to capture a List into a fold or
`integer-fold` quotation so the body can `list.get` by index. The
typechecker computes the correct captures, but `emit_capture_push`
(codegen/words.rs:410-417) hard-errors on `Type::Var` matching
"Variant".

## Why This Is Small

The codegen for capture push already handles 5 types:

| Type | Getter | Pusher |
|------|--------|--------|
| Int | `env_get_int` → i64 | `push_int` |
| Bool | `env_get_bool` → i64 | `push_int` |
| Float | `env_get_float` → f64 | `push_float` |
| String | combined `env_push_string` | (self-contained) |
| Quotation | `env_get_quotation` → i64 | `push_quotation` |

String already uses the "combined get+push" pattern because passing
`Value` by value through FFI crashes on Linux. Variant needs the same
pattern: a single runtime function that reads from the env and pushes
onto the stack, all in Rust.

## Approach

### Runtime (`closures.rs`)

Add `patch_seq_env_push_variant`:
```rust
pub unsafe extern "C" fn patch_seq_env_push_variant(
    stack: Stack,
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> Stack {
    // same bounds checks as env_push_string
    let value = &*env_data.add(idx);
    match value {
        Value::Variant(v) => push(stack, Value::Variant(v.clone())),
        _ => panic!("expected Variant"),
    }
}
```

### Codegen (`words.rs:emit_capture_push`)

Replace the Variant error arm with the String-style combined path:
```rust
Type::Var(name) if name.starts_with("Variant") || ... => {
    let new_stack_var = self.fresh_temp();
    writeln!(&mut self.output,
        "  %{} = call ptr @patch_seq_env_push_variant(ptr %{}, ptr %env_data, i64 %env_len, i32 {})",
        new_stack_var, stack_var, index
    )?;
    return Ok(new_stack_var);
}
```

Also add the Map variant (`Value::Map`) with
`patch_seq_env_push_map` — same pattern, since maps are equally
useful to capture.

### LLVM declarations (`codegen/runtime.rs`)

Add:
```
declare ptr @patch_seq_env_push_variant(ptr, ptr, i64, i32)
declare ptr @patch_seq_env_push_map(ptr, ptr, i64, i32)
```

### Typechecker

**Zero changes.** The typechecker already handles Variant types in
capture analysis. The limitation was purely in codegen.

## Constraints

- No type system changes
- No new capture semantics — existing auto-capture fires correctly
- The `push_closure` runtime (which pops captures from the caller's
  stack at creation time) already handles `Value::Variant` correctly —
  it pops any `Value` and stores it in the Arc env
- Variant capture clones the `Arc<VariantData>` — O(1) refcount bump,
  not a deep copy

## Checkpoints

1. `list-of 1 lv 2 lv 3 lv 0 [ swap list.get drop i.+ ] integer-fold`
   — auto-captures a list into an `integer-fold` body
2. `lagrange-outer-loop` in sss.seq rewritten as `integer-fold` with
   xs and ys lists auto-captured
3. At least 3 other sss.seq loops rewritten
4. Existing closure tests pass unchanged
5. `just ci` clean
