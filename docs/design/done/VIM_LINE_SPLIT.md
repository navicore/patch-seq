# vim-line Split — `crates/vim-line/src/vim.rs`

Status: design · 2026-04-23

## Intent

`crates/vim-line/src/vim.rs` is 1379 lines (996 non-test). It's a single `impl
VimLineEditor` block mixing every concern of the editor: the `Mode` /
`Operator` enums, the `VimLineEditor` struct, cursor-motion helpers
(`move_left`/`move_right`/`move_word_*`/`move_to_matching_bracket`/
`find_matching_*`), edit helpers (`delete_char`/`delete_to_end`/`delete_line`/
`paste_after`/`paste_before`), the five mode-specific key handlers
(`handle_normal`/`handle_insert`/`handle_operator_pending`/`handle_visual`/
`handle_replace_char`), operator application (`apply_operator`/
`apply_operator_line`), and the `LineEditor` trait impl. Plus ~380 lines of
tests.

The file is hard to navigate and the concerns are genuinely separable: a
motion helper never looks at a mode; a handler never looks at another
handler's state. A split would let each resulting module have a narrow
reason to change.

This doc is a scope decision, not an implementation plan. The goal is to
capture **whether** we should split and **along which axis**.

## Decision

**Yes — in three deliberate passes, not a single mega-PR.** The cheapest,
highest-value first cut:

1. **Lift the ~380-line test module** to `vim/tests.rs` via `#[path]` or a
   submodule file at `crates/vim-line/src/vim/tests.rs`. No behaviour change;
   drops the file by ~28%.
2. **Extract motion helpers and edit helpers** into `vim/motions.rs` and
   `vim/edits.rs` as free-fn or thin-impl modules. These methods are pure
   over `(cursor, text)` and don't touch mode — genuinely severable.
3. **Extract per-mode handlers** to `vim/handlers/{normal,insert,
   operator_pending,visual,replace_char}.rs`. This is the higher-risk cut;
   methods share mutable state with `self` and may need a thin `&mut Self`
   pattern or a helper struct.

Passes 1 + 2 get `vim.rs` from 1379 → ~700 L at low risk. Pass 3 is worth
its own design once 1 and 2 settle.

## Constraints

- **Single-file audit tool must not invent cross-file work.**
  `/audit-rust-file` will not execute a split itself; any structural move
  has to be landed separately.
- No public API change outside `crates/vim-line`. `lib.rs` re-exports
  `VimLineEditor`; `LineEditor` trait, `EditResult`, `Key`, `KeyCode`,
  `TextEdit`, `Action` are the public contract. None of those live in
  `vim.rs` except `VimLineEditor` itself, which stays at a stable path via
  the `pub use vim::VimLineEditor` re-export.
- Tests must not be weakened. Moving them to `vim/tests.rs` is a location
  change only — same assertions, same fixtures.
- No dependency changes, no `Cargo.toml` edits.
- Out of scope: rewriting any mode handler, changing event dispatch,
  altering motion semantics. Behaviour must be byte-identical.

## Approach (first two passes only)

### Pass 1 — tests out

Create `crates/vim-line/src/vim/tests.rs`. Move the
`#[cfg(test)] mod tests` block verbatim. `vim.rs` retains a
`#[cfg(test)] mod tests;` declaration. Expected drop: ~380 L (1379 → ~1000).

### Pass 2 — motion and edit helpers out

- `crates/vim-line/src/vim/motions.rs` — free functions taking `(cursor:
  usize, text: &str) -> usize` (or `Option<usize>` for failure) for
  `move_left`, `move_right`, `move_line_start`, `move_first_non_blank`,
  `move_line_end`, `move_line_end_insert`, `move_word_forward`,
  `move_word_backward`, `move_word_end`, `move_up`, `move_down`,
  `move_to_matching_bracket`, `find_matching_forward`,
  `find_matching_backward`. The inherent-method wrappers on
  `VimLineEditor` stay (thin shims) so handlers don't change.
- `crates/vim-line/src/vim/edits.rs` — similarly for `delete_char`,
  `delete_to_end`, `delete_line`, `paste_after`, `paste_before`, plus
  `apply_operator` / `apply_operator_line`. These touch `self.yank_buffer`
  and `self.cursor` so they stay as methods on a small `&mut` struct or
  accept those fields explicitly — TBD at implementation time.

Expected drop: ~250 L. Total after passes 1 + 2: vim.rs ≈ 700 L.

### Pass 3 (deferred)

Split `impl VimLineEditor` by mode handler. Each of
`handle_normal`/`handle_insert`/`handle_operator_pending`/
`handle_visual`/`handle_replace_char` becomes its own file under
`vim/handlers/`. The shared motion dispatch (hoisted out of the three
handlers that currently duplicate it — see the
`/audit-rust-file` findings for `vim.rs`) can land in pass 2 as a
method or in pass 3 as a standalone `dispatch_motion` fn. Risks: method
fragmentation makes control flow harder to trace; several handlers call
each other through `self.apply_operator`, which may force the helper
into a shared location.

## Domain events

None. Pure reorganization. No observable behaviour change, no new log
lines, no new public API.

## Checkpoints

After each pass:

- `just ci` green (fmt, clippy `-D warnings`, unit tests, build,
  integration, seq lint).
- `cargo test -p vim-line` passes — test count matches pre-move.
- Every consumer (`crates/repl`) builds unchanged — `LineEditor` trait
  and `VimLineEditor` type are unmoved.

Success criterion for the overall effort: each resulting file fits in a
single editor screen of scroll and each concern has an obvious home.
Pass 3 (the handler split) is worth taking up only if passes 1 + 2
don't feel like enough.
