# REPL App Split — `crates/repl/src/app.rs`

Status: design · 2026-04-22

## Intent

`crates/repl/src/app.rs` is 2121 lines (1548 non-test). It's a single `impl
App` block mixing every concern of the TUI: construction + history, key
dispatch, input execution (definitions / expressions / includes / session
replay), slash commands, IR view computation, search mode, tab completion,
and rendering. Two free fns — `run_with_timeout` (~140 L) and
`floor_char_boundary` — sit above the struct.

The file is hard to navigate and the concerns barely overlap at the field
level: key handling reads different state than rendering, which reads
different state than the execution pipeline. A split would let each
resulting module have a narrow reason to change.

This doc is a scope decision, not an implementation plan. The goal is to
capture **whether** we should split and **along which axis**.

## Decision

**Yes — but in deliberate passes, not a single mega-PR.** The cheapest,
highest-value first cuts:

1. **Lift the 575-line test module** to `app/tests.rs` via a `#[path]`
   attribute (or equivalently a private submodule that re-uses `use
   super::*`). No behavior change; halves the file at zero risk.
2. **Extract the free fns** (`run_with_timeout`, `floor_char_boundary`)
   into `run.rs` (process-spawn helper) and `text_utils.rs`. Neither
   touches `App` state; they're free-floating.

Those two moves alone drop `app.rs` from 2121 → ~1400 L and isolate
the God-object from its test surface.

Any deeper cut (splitting `impl App` across concern modules) can be a
follow-up design once the above settles; it has higher risk (method
fragmentation, visibility churn) and needs its own plan.

## Constraints

- **Single-file audit tool must not invent cross-file work.**
  `/audit-rust-file` will not execute a split itself; any structural move
  has to be landed separately.
- No public API change outside `crates/repl`. The binary entry is `main.rs`;
  it only needs `App::new`, `App::with_file`, `App::handle_key`,
  `App::render`, `App::save_history` to keep working.
- Tests must not be weakened. Moving them to `app/tests.rs` is a location
  change only — same assertions, same fixtures.
- No dependency changes, no `Cargo.toml` edits.
- Out of scope: rewriting `handle_key` (270 L), changing event dispatch,
  altering rendering behavior, or replacing the search/IR-view logic.

## Approach (first two passes only)

### Pass 1 — tests out

Create `crates/repl/src/app_tests.rs` (or `crates/repl/src/app/tests.rs`
under a `mod app;` directory layout). Move the `#[cfg(test)] mod tests`
block verbatim. `app.rs` retains a `#[cfg(test)] mod tests;` declaration.
Expected drop: ~575 L.

### Pass 2 — free fns out

- `run_with_timeout(path: &Path) -> RunResult` → `crates/repl/src/run.rs`.
  Declared in `main.rs` as `mod run;`. Callers in `app.rs` switch from
  bare function call to `crate::run::run_with_timeout`.
- `floor_char_boundary(s: &str, pos: usize) -> usize` → a new
  `crates/repl/src/text_utils.rs`. Single-line-body utility, private to
  the crate.

Expected drop: ~160 L. Total after passes 1+2: app.rs ≈ 1400 L.

### Pass 3 (deferred)

Split `impl App` along concern lines — candidates: `input` (key
dispatch), `execute` (input pipeline), `commands` (slash commands),
`search` (search mode), `ir_view` (IR pane generation), `render`
(rendering). Each becomes a separate file with its own `impl App` block.
This needs its own design doc; risks include visibility churn (private
fields becoming `pub(crate)`) and method fragmentation that makes flow
harder to trace. Not in scope for this doc.

## Domain events

None. Pure reorganization. No observable behavior change, no new log
lines, no new channels or lifecycle hooks.

## Checkpoints

After each pass:

- `just ci` green (fmt, clippy `-D warnings`, unit tests, build,
  integration, seq lint).
- `cargo test -p seq-repl` passes — specifically the relocated tests run
  and count matches pre-move.
- REPL smoke test: `cargo run -p seq-repl` opens the TUI, accepts input,
  renders, quits cleanly.

Success criterion for the overall effort: `app.rs` fits in a single
editor screen of scroll and each concern has an obvious home. Pass 3
(the deeper cut) is worth taking up only if passes 1+2 don't feel like
enough.
