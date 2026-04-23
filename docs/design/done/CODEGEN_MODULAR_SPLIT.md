# Codegen Modular Split — `specialization.rs` and `runtime.rs`

Status: design · 2026-04-21

## Intent

Break two oversized files in `crates/compiler/src/codegen/` into smaller,
scope-focused modules so each file represents one conceptual responsibility
and can be reasoned about (and tested) with minimal cross-file context.

- `specialization.rs` (1667 L) — register-based codegen for primitive-typed
  words. Mixes types, the SSA-stack abstraction, eligibility analysis, and
  LLVM IR lowering for every op category.
- `runtime.rs` (1533 L) — two large static tables (runtime function
  declarations and Seq-word → C-symbol mapping). Pure data with category
  comment banners; grew monolithic by accretion.

File size is a symptom; the real goal is that each resulting file has a
narrow reason to change and few dependencies on siblings.

## Constraints

- No public API change outside the `codegen` module. External callers
  (`codegen::BUILTIN_SYMBOLS`, `RUNTIME_DECLARATIONS`, `emit_runtime_decls`,
  `RegisterType`, `SpecSignature`) keep their current paths via re-exports.
- Tests must not be weakened. Existing specialization unit tests move with
  the types they exercise; no new `#[allow]` attrs to silence churn.
- No dependency changes. No `Cargo.toml` edits.
- Specialization rules (the 65 supported operations, eligibility criteria,
  `musttail` semantics, safe-div/shift codegen) stay byte-equivalent. This
  is a reorganization, not a behavior change.
- Out of scope: new tests beyond what naturally attaches to extracted
  units, performance tuning, refactoring of `mod.rs` / `words.rs` /
  `statements.rs` (separate audits).

## Approach

### `runtime.rs` → `runtime/` directory

The file is essentially ~35 category blocks of `RuntimeDecl` + a parallel
table of Seq-word → C-symbol mappings. Each category is independent data.
Split along roughly these axes (each file exposes `pub(super) const
DECLS: &[RuntimeDecl]` and `pub(super) const SYMBOLS: &[(&str, &str)]`):

- `numeric.rs` — arithmetic, comparisons, bitwise, boolean, float ops
- `stack.rs` — dup/swap/rot/pick/roll and tagged-stack ops
- `text.rs` — strings, symbols, variants, encoding
- `collections.rs` — lists, maps
- `concurrency.rs` — channels, strands, weaves, quotations, closures,
  combinators, scheduler
- `io_os.rs` — files, directories, tcp, http, terminal, signals, os, args
- `data.rs` — crypto, compress, regex
- `diagnostics.rs` — test framework, son, stack introspection, report, time
- `misc.rs` — exit code, cond helpers, anything left over

`runtime/mod.rs` flattens the category slices into the public
`RUNTIME_DECLARATIONS: Vec<RuntimeDecl>` and `BUILTIN_SYMBOLS: HashMap<...>`
via `LazyLock`, and exposes `emit_runtime_decls`. Rough target: ~10 files
averaging ~150 L; each is trivially testable with "this slice has N
expected pairs" drift-catch assertions.

### `specialization.rs` → `specialization/` directory

Split by concern, not by line count:

- `types.rs` — `RegisterType`, `SpecSignature` (low-dep, pure; existing
  `test_register_type_from_type` / `test_spec_signature_suffix` move here)
- `context.rs` — `RegisterContext` SSA stack abstraction (existing
  `test_register_context_stack_ops` moves here)
- `eligibility.rs` — `can_specialize`, `extract_register_types`,
  `is_body_specializable`, `is_statement_specializable` — analysis only,
  no IR emission, testable against small `WordDef` fixtures
- `lowering.rs` (or a `lowering/` sub-dir if it stays >500 L) — the
  `codegen_specialized_*` methods that emit LLVM IR: statement dispatch,
  word call, icmp/fcmp, safe-div, safe-shift, recursive call, return,
  if-lowering
- `mod.rs` — re-exports the public surface

Where practical, pure IR-string builders get extracted from `impl CodeGen`
so lowering can be exercised without constructing full codegen state. That
is a stretch goal, not a requirement of this split.

## Domain Events

- **Split lands for `runtime.rs`** → all downstream code referring to
  `codegen::BUILTIN_SYMBOLS` / `RUNTIME_DECLARATIONS` / `emit_runtime_decls`
  must continue to compile unchanged. CI catches regressions.
- **Split lands for `specialization.rs`** → `statements.rs` (uses
  `RegisterType`) and `state.rs` (uses `SpecSignature`) continue to
  import from `super::specialization::*`; only the internal file layout
  moves. Re-exports in `specialization/mod.rs` preserve those paths.
- **Audit checklist update** → after each split, tick the corresponding
  HIGH item in `tmp/rust-audit-crates-compiler-src-codegen.md` with a
  one-line summary; the new sub-files become individually auditable in
  future `/audit-rust-file` runs.

## Checkpoints

1. `just ci` passes after the `runtime/` split with zero diff in produced
   LLVM IR for a representative program (compile `examples/` and diff
   `--keep-ir` output before/after to confirm no behavior drift).
2. `just ci` passes after the `specialization/` split, including the
   three existing unit tests (relocated but not modified).
3. The two affected `mod.rs` files stay under ~100 L (aggregator only,
   no logic).
4. Every new leaf file is <300 L and has at most one or two sibling
   dependencies. Spot-check by reading the final `use` blocks.
5. No new `pub` items leak outside `codegen`; visibility audit limited
   to `pub(super)` / `pub(crate)` within the new sub-trees.
