# myrust Doc-Site Skill Design

Status: design · 2026-04-23

## Intent

We've had good results with `myrust`'s one-shot setup skills
(`setup-rust-ci`, `setup-crates-release`) and its audit pair
(`audit-rust-plan` / `audit-rust-file`). Apply the same shape to
**generated documentation sites** so any Rust project can gain the
`patch-seq` setup with one skill invocation. Separately, consider
whether a doc-audit pair is worth the weight.

## Constraints

- **One-shot setup, not a framework.** Skill writes concrete files
  (workflow, config, landing-page stub, generator script if needed)
  and stops. No runtime component, no "init/update/refresh" lifecycle.
- **Opinionated defaults, no menu.** Pick one good shape per target and
  commit to it. If the user later wants something different they edit
  the generated files by hand.
- **Don't fight the toolchain.** The skill must honor an existing
  `rust-toolchain.toml` and not pin a different nightly. CI-readable
  layout (runs on `ubuntu-latest`, artifacts via
  `actions/upload-pages-artifact`, deploy via `actions/deploy-pages`).
- **No overlap with `setup-rust-ci`.** Docs gets its own workflow file
  (`docs.yml`). `ci.yml` stays untouched.
- **Out of scope:** theming, custom CSS, versioned doc hosting
  (historical tags), doc search backends, external hosting providers,
  anything that's not a one-PR incremental add to a Rust repo.

## Approach

### Primary skill: `setup-rust-docs`

**Two shapes, chosen at invocation time** based on a single prompt:

1. **`rustdoc`** (default, minimal) — `cargo doc --no-deps` to
   `target/doc`, deploy to Pages. `#[doc = include_str!("../README.md")]`
   added to each crate root that doesn't already have it. Workspace
   root redirect if multi-crate.
2. **`mdbook`** (rich) — mirror what patch-seq has: `book.toml`,
   `docs/SUMMARY.md` scaffold, `scripts/generate-examples-docs.sh`
   pattern, `just gen-docs` recipe, `docs.yml` workflow with a
   triggering `paths` list.

The skill asks once ("rustdoc-only? or mdbook?") and proceeds
without further menus. Detection heuristic to suggest a default: if
the repo already has a `docs/` dir or `book.toml`, suggest mdbook;
else suggest rustdoc.

### Files it writes

Shared by both shapes:

- `.github/workflows/docs.yml` — build + deploy-to-Pages, triggered
  on `push` to `main` with a `paths:` filter and
  `workflow_dispatch`. Permissions block for `pages: write`.

rustdoc shape, additional:

- `#[doc = include_str!("../README.md")]` at each crate root (skip if
  already present).
- `.github/workflows/docs.yml` runs `cargo doc --no-deps
  --workspace`, uploads `target/doc`.

mdbook shape, additional:

- `book.toml` with sensible defaults.
- `docs/SUMMARY.md` seeded from `README.md` + any top-level `docs/*.md`.
- `scripts/generate-<name>-docs.sh` if the project has an
  `examples/` tree with per-folder `README.md` files (generator that
  stitches them into a single `EXAMPLES.md`).
- `just gen-docs` / `just build-docs` recipes appended to `justfile`
  if one exists.

### Secondary: doc audit skills — deferred, not built

My honest take is this is **not redundant, but lower priority than
setup-rust-docs, and partially overlapping with existing tooling**:

- `rustdoc::missing_docs` and `rustdoc::broken_intra_doc_links` lints
  already catch surface problems if the setup skill enables them in
  `Cargo.toml` `[lints]`. `setup-rust-docs` should do that by default.
- `audit-rust-file` already flags undocumented `pub` items in its
  Section C.
- What a doc-audit pair would add is prose-quality checks: guide
  freshness, example-code drift, dead links in `.md` files,
  cross-references to code that moved.

**Recommendation:** ship `setup-rust-docs` with strict doc lints, see
whether the gap feels real in practice, and only then design the
audit pair.

## Domain events

- **User invokes `setup-rust-docs`** → one PR worth of files lands.
  User pushes; Pages deploy fires on the next main push.
- **Existing project already has some of these files** → skill refuses
  to overwrite; reports what it would have written and exits clean.
  No silent merge.
- **Strict lints land** → downstream `cargo doc` runs may now fail on
  missing docs. Skill notes this in its exit message as an expected
  outcome, not a bug.

## Checkpoints

1. `setup-rust-docs` invoked on a fresh scratch Rust crate produces a
   doc site on Pages within one push-to-main cycle.
2. `setup-rust-docs` invoked on `patch-seq` (which already has the
   mdbook shape) detects conflicts and exits without touching files.
3. The rustdoc shape's `cargo doc --no-deps --workspace` runs locally
   without warnings on a simple one-crate and one workspace scratch
   project.
4. The mdbook shape reproduces `patch-seq`'s current docs.yml +
   `book.toml` + generator script shape close enough that running the
   skill on patch-seq (after deleting those files) produces a working
   equivalent.
5. Strict doc lints added to the scratch crate surface `missing_docs`
   warnings on undocumented `pub` items — confirming the overlap
   mentioned above before we decide on a doc-audit skill.
