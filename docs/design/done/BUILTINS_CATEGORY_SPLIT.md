# Builtins Phase 2 — Category Split

Status: design · 2026-04-21

## Intent

Break `crates/compiler/src/builtins.rs` (935 L) and its sibling
`builtins/docs.rs` (924 L) into per-domain category files so each Seq
builtin family (arithmetic, I/O, list, float, …) lives in one place with
its effect signatures, its docstrings, and — eventually — its tests.

Phase 1 (already shipped) split the file along the trivial axis: tests and
the `BUILTIN_DOCS` lazy map moved to siblings. That left two 900-ish-line
monoliths, each still organised by flat `// === Section ===` banners
inside one giant fn body (`builtin_signatures`) and one giant closure body
(`BUILTIN_DOCS`). Adding a single new builtin today means editing both
monoliths in parallel, with no enforced correspondence between them.

Phase 2 co-locates each category's signatures and docs in one sub-module,
so adding a new family entry is a one-file, two-line change.

## Constraints

- Public API stays identical: `builtin_signature`, `builtin_signatures`,
  `builtin_doc`, `builtin_docs` keep their current paths and signatures.
  Consumers (typechecker, LSP, codegen) must not need any import change.
- No signature or docstring changes — byte-equivalent output for both
  the populated `HashMap<String, Effect>` and the docs map.
- Duplicated category banners in the current `BUILTIN_DOCS` map (e.g.
  "TCP Operations", "Regex Operations", "Crypto Operations" appear twice
  from accretion) get merged. Net content unchanged; only layout differs.
- Tests stay where Phase 1 put them (`builtins/tests.rs`). No new tests
  required by this change, though category files enable cheap per-family
  drift-catch tests in follow-up.
- No new dependencies, no `Cargo.toml` edits.
- Out of scope: adding / removing / renaming any builtin; changing the
  macro DSL; restructuring `Effect` or related types.

## Approach

### Shared macros

The 7 `macro_rules!` helpers (`ty!`, `stack!`, `builtin!`, plus the four
family macros `builtins_int_int_to_int!`, etc.) are used pervasively by
the sigs code. Put them in `builtins/macros.rs` and expose each via

```rust
macro_rules! builtin { ... }
pub(super) use builtin;
```

Category sub-modules import what they need:
`use super::macros::{ty, stack, builtin};`. This is more explicit than
`#[macro_use] mod macros;` and matches the modern-Rust pattern we already
use elsewhere in the compiler. No crate-root pollution.

### Category layout

Mirror runtime.rs's 16-file granularity for consistency; each file holds
**both** the signatures add-fn and the docs add-fn for its domain.

Proposed files (rough estimates):

- `io.rs` (I/O + args + type conversions)
- `fs.rs` (files + directories)
- `arith.rs` (int arith + compare + boolean + bitwise)
- `stack.rs` (stack shuffles + aux slots)
- `concurrency.rs` (channels + strand spawn/weave)
- `callable.rs` (quotations + dataflow combinators + cond)
- `tcp.rs`
- `os.rs` (os + signal + terminal)
- `text.rs` (string + encoding + regex + crypto + compress + http)
- `adt.rs` (variants)
- `list.rs`
- `map.rs`
- `float.rs` (float arith + compare + conversions)
- `diagnostics.rs` (test + time + son + stack.dump)

Each file exports:

```rust
pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) { … }
pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) { … }
```

The aggregator `builtins.rs` becomes a thin file: imports, module
declarations, the three public fns (`builtin_signature`,
`builtin_signatures`, `builtin_doc`, `builtin_docs`), and a
`BUILTIN_DOCS` LazyLock that calls each category's `add_docs`.
`builtins/docs.rs` goes away.

### Docs banner deduplication

Before category extraction, read `BUILTIN_DOCS` top-to-bottom and note
which entries live under each duplicated banner. Merge into a single
logical block per category when copying into the sub-module. Net
content is unchanged; only the source layout collapses.

## Domain Events

- **Phase 2 lands** → `builtins.rs` drops from 935 L to ~80 L.
  `builtins/docs.rs` (924 L) is replaced by 14 category files averaging
  ~130 L each. Adding a new builtin becomes a single-file edit.
- **Audit checklist update** → tick the builtins entry in
  `tmp/rust-audit-crates-compiler-src.md` and remove the Phase 2
  deferred note.
- **Downstream** — typechecker (calls `builtin_signature` /
  `builtin_signatures`) and LSP (calls `builtin_doc` / `builtin_docs`)
  continue to work unchanged.

## Checkpoints

1. `just ci` passes. Test count unchanged.
2. `builtin_signatures()` and `builtin_docs()` return maps with
   identical keys and values before vs. after. Verify by quick diff:
   serialize to a sorted `Vec<(key, debug-value)>` pre- and post-split.
3. `builtins.rs` is under 100 L.
4. No category file exceeds ~250 L.
5. External callers (`typechecker`, `lsp`) compile without import edits.
6. No new `pub` items escape the `builtins` module.
