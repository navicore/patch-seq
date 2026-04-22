# Typechecker Refactor — `typechecker.rs`

Status: design · 2026-04-21

## Intent

Make `crates/compiler/src/typechecker.rs` (5939 L) navigable without
sacrificing behaviour. The goal is maintainability for human and AI
readers, not performance, API ergonomics, or new type-system features.
This is the biggest single `.rs` file in the workspace; its size drives
every audit and every exploration into the type system.

Observation: lines 1-2240 are production code (`TypeChecker` struct +
one ~2100-line `impl` block of ~60 methods); lines 2241-5939 are one
`#[cfg(test)] mod tests { ... }` block with ~80 test functions. The
tests account for 62% of the file. This is the same shape `codegen/
mod.rs` had before we moved its tests to a sibling `tests.rs`.

## Constraints

- No observable behaviour change. Every existing test keeps its name,
  body, and assertions. The inferred types produced for any Seq
  program must be identical before and after.
- No public API change. External callers (`crate::TypeChecker`,
  `check_program`, `take_*`, `register_external_*`) keep their current
  signatures and paths.
- No new lint rules, no new type-system features, no generics, no
  `Result<T, E>` enrichment beyond today's `Result<…, String>`. Pure
  reorganisation.
- No `Cargo.toml` edits, no dependency changes.
- Phase 2 (below) stays internal to the typechecker module. Sibling
  files (`call_graph.rs`, `unification.rs`, `resolver.rs`, …) don't
  move.
- Tests stay `#[cfg(test)]`; no conversion to integration tests.

## Approach

Two phases, shippable independently. Phase 1 alone is worth doing even
if Phase 2 never happens — it immediately cuts the file by 62%.

### Phase 1 — lift tests into a sibling file

Mirror the `codegen/mod.rs` → `codegen/tests.rs` split we just landed.

1. Convert `typechecker.rs` to directory form: rename to
   `typechecker/mod.rs` (or use Rust's "module file + sibling dir"
   convention — both work; pick whichever keeps existing imports
   happiest).
2. Move the `mod tests { ... }` body into `typechecker/tests.rs`.
3. `mod.rs` becomes `#[cfg(test)] mod tests;`.

Result: ~2240 L production file + ~3700 L tests file. Checkpoint: test
pass counts identical before and after.

### Phase 2 — split the production `impl` block by concern

With the test file gone, the remaining ~2240 L is one `impl TypeChecker`
block with methods that group into clear domains. Rust allows one type
to have multiple `impl` blocks across files, so each sub-module holds
its own `impl TypeChecker { … }` containing the methods for its
concern. The `TypeChecker` struct definition itself stays in
`typechecker/mod.rs` alongside `pub use` re-exports.

Proposed sub-module layout (rough per-file line estimates):

- `typechecker/mod.rs` — struct definition, public API surface, module
  declarations, re-exports (~100 L)
- `typechecker/state.rs` — constructors, accessors (`get_union`,
  `find_variant`, `take_*`), `register_external_*`,
  `capture_statement_type`, `set_call_graph` (~200 L)
- `typechecker/validation.rs` — `validate_main_effect`,
  `validate_union_field_types`, `validate_effect_types`,
  `validate_stack_types`, `validate_type`, `parse_type_name`,
  `is_valid_type_name` (~200 L)
- `typechecker/freshen.rs` — `fresh_var`, `freshen_effect`,
  `freshen_side_effect`, `freshen_stack`, `freshen_type` (~140 L)
- `typechecker/stack_utils.rs` — `stack_depth`,
  `get_trivially_copyable_top`, `count_concrete_types`,
  `get_row_var_base`, `get_type_at_position`, `rotate_type_to_top`,
  `pop_type`, `apply_effect`, `lookup_word_effect` (~300 L)
- `typechecker/driver.rs` — `check_program`, `check_word`,
  `infer_statements`, `infer_statements_from`, `infer_statement` (~250 L)
- `typechecker/control_flow.rs` — `infer_if`, `infer_match`,
  `push_variant_fields`, `is_divergent_branch` (~400 L)
- `typechecker/words.rs` — `infer_word_call`, `infer_to_aux`,
  `infer_from_aux`, `infer_call`, `resolve_arithmetic_sugar` (~400 L)
- `typechecker/combinators.rs` — `infer_dip`, `infer_keep`, `infer_bi`
  (~250 L)
- `typechecker/quotations.rs` — `infer_quotation`, `analyze_captures`,
  `adjust_stack_for_spawn` (~300 L)
- `typechecker/pick_roll.rs` — `handle_literal_pick`,
  `handle_literal_roll` (~150 L)

Cross-file method calls: since all methods live in one `TypeChecker`
impl (just spread across files), cross-file calls are just `self.foo()`.
No visibility gymnastics. No pub-widening. No new traits. The methods
retain their existing visibility (mostly `fn`, a handful `pub fn`).

Where a concern surfaces a small helper struct (e.g., a temporary holding
substitution + stack for `rotate_type_to_top`), that struct stays file-
local. No new exported types beyond today's set.

## Domain Events

- **Phase 1 lands** → `typechecker.rs` drops to ~100 L (aggregator) +
  ~2100 L (production) + ~3700 L (tests, relocated). The audit
  checklist `tmp/rust-audit-crates-compiler-src.md` gets ticked for
  `typechecker.rs` with an ↗ pointer to the new `tests.rs` and a
  deferred note for Phase 2.
- **Phase 2 lands** → 10 focused sub-module files replace the single
  ~2100 L production `impl` block. Each file is ≤400 L. The first
  concern to be changed or audited in isolation becomes trivial to
  reason about without loading the rest.
- **Any downstream** — `lib.rs`, `codegen/`, `lsp/` — continues to
  compile unchanged. External callers never needed to know
  typechecker internals existed.

## Checkpoints

1. `just ci` passes after Phase 1. Test pass count from `cargo test
   -p seq-compiler --lib 2>&1 | grep "test result"` matches main
   exactly (spot-check: the top-level run on `main` before the change).
2. After Phase 2: no sub-module exceeds ~400 L. `typechecker/mod.rs`
   stays under ~120 L (type definition + module declarations + re-
   exports, no logic).
3. No `pub` item escapes the typechecker module that didn't already
   escape. Verify by grepping `pub use typechecker::` across the
   workspace before and after: the set should be byte-identical.
4. A spot audit — read two randomly picked `infer_*` methods in their
   new files — should feel self-contained: each opens, you see what
   it needs, no need to load the whole typechecker.
5. Phase 2 can be further split across multiple PRs (e.g., lift out
   `validation` + `freshen` + `state` first, then `control_flow` +
   `quotations` + `combinators`, then the rest). Reviewers should be
   able to approve each PR standalone.
