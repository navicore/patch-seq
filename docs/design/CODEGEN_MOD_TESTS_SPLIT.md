# Move codegen tests out of `codegen/mod.rs`

Status: design · 2026-04-21

## Intent

Split `crates/compiler/src/codegen/mod.rs` — currently 1448 L — into a thin
module aggregator (~100 L) and a sibling `codegen/tests.rs` holding the unit
tests. This completes the codegen modular-split series (after `runtime.rs`,
`specialization.rs`, `program.rs`, `inline/dispatch.rs`, `statements.rs`,
`words.rs`).

Unlike the earlier splits, `mod.rs` is *already* lean for production code:
lines 1-101 are module declarations, public re-exports, and crate-level
rustdoc. The remaining ~1350 L is one `#[cfg(test)] mod tests` block
containing ~28 end-to-end tests. Moving those tests into a sibling file
makes the orchestrator file fit on one screen without touching production
code at all.

## Constraints

- No test removals or weakening. Every `#[test]` keeps its name, body, and
  assertions. Test behaviour must be byte-identical.
- No public API change. `codegen::{CodeGen, CodeGenError, BUILTIN_SYMBOLS,
  RUNTIME_DECLARATIONS, emit_runtime_decls, ffi_c_args, ffi_return_type,
  get_target_triple}` keep their current paths.
- Tests that currently rely on `use super::*` must keep working; the move
  shifts `super::*` from "the `codegen` module" to "the `codegen::tests`
  module's parent" — which is still `codegen`, so imports are unchanged.
- No new dependencies, no `Cargo.toml` edits.
- Production code layout (`control_flow`, `ffi_wrappers`, `globals`,
  `inline`, `layout`, `platform`, `program`, `runtime`, `specialization`,
  `state`, `statements`, `types`, `virtual_stack`, `words`) stays as-is.

## Approach

Create `crates/compiler/src/codegen/tests.rs` containing the exact
contents of the current `mod tests { ... }` block (minus the outer
wrapper). In `mod.rs`, replace the inline test module with:

```rust
#[cfg(test)]
mod tests;
```

No `#[path]` attribute needed: Rust's standard submodule resolution
(`<parent_dir>/tests.rs`) picks up the file automatically because
`codegen` is already a directory-form module.

Tests are end-to-end (build a `Program` AST, call `codegen_program*`,
assert on the emitted IR string) and don't target any single sibling
file. Co-locating them with one production module would be misleading.
A sibling `tests.rs` is the honest home.

Optional follow-ups (deferred, not in this change):

- Group related tests into helper fixtures (e.g., a `mk_program(word)`
  builder that the ~15 tests currently repeat inline). That's a
  within-file cleanup; would fit a `/audit-rust-file` pass on
  `tests.rs` afterwards.
- Break the ~1350 L of tests into sub-files by concern (e.g.
  `tests/instrument.rs`, `tests/specialization.rs`). Only worth doing
  if the flat file proves hard to navigate.

## Domain Events

- **Split lands** → `mod.rs` drops from 1448 L to ~100 L. The audit
  checklist tick for `mod.rs` gets an ↗ pointer to `tests.rs` as the new
  candidate for future within-file cleanup (fixture extraction).
- **CI run** → `just test` must produce identical pass/fail counts and
  identical test names to what `main` produces today. Any diff is a
  regression in the move, not an acceptable churn.
- **Future audit** → `tests.rs` shows up as its own HIGH-bucket entry
  under an audit pass of `codegen/`. That's fine; tests have different
  audit criteria (fixture reuse, assertion clarity) than production
  code.

## Checkpoints

1. `just ci` passes. Test count unchanged (spot-check via `cargo test
   -p seq-compiler --lib 2>&1 | grep "test result"` before and after).
2. `mod.rs` is under ~120 L with no `#[cfg(test)]` blocks.
3. `tests.rs` compiles and every test in it still sees the same imports
   (`use super::*;` now means `codegen::*`, which is what it meant
   before — only the lexical location changed).
4. No production file other than `mod.rs` is modified.
5. Audit checklist (`tmp/rust-audit-crates-compiler-src-codegen.md`)
   updated: tick `mod.rs`, note `tests.rs` as a follow-up candidate.
