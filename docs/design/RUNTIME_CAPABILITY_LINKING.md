# Runtime Capability Linking — structural no-dead-code on all platforms

> Status: Phase 0 **implemented** (2026-09); Phases 1–2 remain proposed.
> Extends the finished work documented in `docs/BINARY_FOOTPRINT.md` and
> the no-dead-code test (`crates/compiler/tests/no_dead_code.rs`).
>
> Phase 0 implementation notes:
> - `http_client` was already feature-gated; the ungated rustls surface
>   turned out to be `tls.rs` plus `tcp.rs`'s TLS-upgrade arm
>   (`StreamKind::Tls`, `upgrade_tcp_in_place`).
> - Base archive (10.8 MB vs 62 MB full; `just build-runtime-base`, own
>   target dir because the staticlib names collide) is embedded next to
>   the full one; seqc selects by scanning referenced words for the
>   capability namespaces (`net.http.`, `net.tls.`, `crypto.`, `regex.`,
>   `compress.`) — conservative: dead user code still selects full.
> - Verified x86_64: io-only hello links base and passes no_dead_code
>   structurally (zero capability code in the archive); `net/http`
>   example links full (1111 rustls symbols — capability presence).
> - The stub FFI symbols (`patch_seq_sha256` etc.) contain the test
>   needles as substrings but are plain Rust fns — GC removes them when
>   unreferenced on every architecture (unlike ring's asm).
> - Fixed en passant: the runtime temp file used a fixed name
>   (`/tmp/libseq_runtime.a`) — concurrent builds clobbered each other;
>   now a per-process subdirectory (`-lseq_runtime` needs the literal
>   archive name, so uniqueness lives in the directory).

## Problem

The no-dead-code guarantee is **emergent, not structural**: seqc links a
single fat `libseq_runtime.a` (62 MB, every capability compiled in) and
relies on `-Wl,--gc-sections` to prune what the program doesn't use.

That worked on x86_64-linux (the only Linux the project had been exercised
on). It does **not** work on aarch64-linux: `ring`'s hand-written asm
kernels are referenced in ways the linker's reachability analysis cannot
see, so they survive GC and root their neighbors. Observed on
aarch64-linux, clang 21.1.8 (RESF), release profile:

| needle | x86_64-linux budget | aarch64-linux observed |
|---|---|---|
| rustls | 4 (tolerated drop-wrappers) | 396 |
| sha2 / hmac / ed25519 / aes_gcm | 0 (forbidden) | 3 / 23 / 9 / 14 |

This is the same failure class already documented and `#[ignore]`d for
macOS ("4 ring asm stragglers"), ~100× worse. aarch64-linux is now a
first-class target (arm VMs are the standard dev/test environment); the
guarantee must not depend on linker heroics per-platform.

Contributing design hole found while diagnosing: the runtime's feature
table (`full = ["crypto", "http", "regex", "compression"]`) gates the
crypto deps, `url`, `regex`, and `flate2`/`zstd` — but the hand-rolled
HTTP client (`crates/runtime/src/http_client/`) and its `rustls`
dependency are **unconditional** (`rustls = { workspace = true }`,
not optional, not under any feature). Every runtime build embeds the
entire TLS stack regardless of features.

## Principle

**What lands in a binary is decided by what gets linked, not by what
survives the linker.** Platform-parity (x86_64-linux, aarch64-linux,
macOS) is a requirement, not a best-effort.

## Design

### Phase 0 — gate the escapee (small, standalone, unblocks aarch64)

Move `http_client/` and its deps under the `http` feature:

- `rustls` becomes `optional = true`, listed in `http = [...]` (note:
  rustls already uses `default-features = false, features = ["ring", ...]`
  — keep that pinning).
- `http_client/` module tree compiled only under `#[cfg(feature = "http")]`;
  `net.*` words that need it return a clear "capability not linked" error
  when the feature is off.
- Runtime built without `http` contains no rustls/ring at all → hello.seq
  (io-only) links clean on aarch64-linux **structurally**, and macOS's
  four stragglers likely vanish for the same reason (no ring in the
  archive to begin with).

Phase 0 alone should turn the aarch64 test green without any `#[ignore]`.

### Phase 1 — per-program feature selection in seqc

seqc already validates which words a program references
(`validate_word_calls_with_externals`). Add a **word → capability map**
driven by module prefixes:

| program references | runtime feature |
|---|---|
| `io.*`, stack/list core | (base) |
| `net.*`, http words | `http` (pulls crypto where the TLS stack needs it) |
| crypto words (`sha256`, hmac, aes…) | `crypto` |
| regex words | `regex` |
| compression words | `compression` |

The map is data (a table in the compiler), auditable and unit-testable:
`parse + select` gets its own tests, independent of linking.

Mechanics — two options, decision needed:

- **A. Curated bundle lattice.** build.rs builds N embedded archives
  (e.g. `base`, `base+crypto`, `full`). Few moving parts, but either a
  powerset explosion (16 × ~60 MB — unacceptable) or coarse bundles that
  re-admit the problem one level up ("full" for a regex-only program).
- **B. Per-capability archive split (preferred).** Split the runtime into
  sibling crates mirroring the feature table — `seq-runtime-core`,
  `seq-runtime-http`, `seq-runtime-crypto`, `seq-runtime-text`,
  `seq-runtime-compress` — each `crate-type = ["staticlib", "rlib"]`,
  dependency edges `http → crypto → core`, `text → core`,
  `compress → core`. seqc embeds each archive separately and passes
  `-l` for exactly the selected set. Total embedded bytes ≈ today's
  single archive; selection is ordinary linker input.

Option B is more churn but is the maintainable shape: adding a capability
means adding a crate, not reasoning about bundle matrices.

Side fix either way: `temp_dir().join("libseq_runtime.a")` is a fixed
filename — two concurrent seqc builds clobber each other. Per-archive
names (or `mktemp`) fall out of this work naturally.

### Phase 2 — two-sided test contract

The tripwire becomes a contract, not just a leak detector:

- hello.seq: **zero** forbidden/tolerated leakage on x86_64-linux AND
  aarch64-linux (drop the macOS `#[ignore]` if Phase 0 removes its
  stragglers).
- Inverse assertions: a `net.*`-using program **must** contain rustls
  symbols; a non-crypto program must contain none. Capability presence
  and absence both asserted, on both architectures.

## Alternatives considered (rejected)

- **Keep gc-sections, fix ring's section granularity on aarch64**
  (inject `-ffunction-sections` into ring's cc build). Treats the
  symptom; every new asm-bearing dependency re-opens the hole; still
  leaves the guarantee emergent.
- **`--whole-archive` + `--gc-sections` heroics / lld**. Same class:
  linker-version-dependent behavior differences are exactly what bit us
  (clang 21 vs 23 was a red herring here, but only by luck of the counts).
- **LTO the runtime into programs.** Runtime is linked as native archive
  by design (IR-level linking of the whole runtime would also change
  build-time characteristics dramatically). Not pursued.

## Rollout

1. Phase 0 PR: feature-gate `http_client` + rustls; aarch64 CI leg (or a
   committed QEMU/VM test note) goes green; update
   `footprint-measurements.md`.
2. Phase 1 PR(s): word→capability table + selection; archive split (B);
   fixed temp-filename race fixed as part of the split.
3. Phase 2 PR: two-sided tests; drop the macOS ignore if justified;
   record per-arch budgets as zeros.

## Open questions

- Exact word vocabulary audit for each capability prefix (does anything
  outside `net.*` implicitly want http? FFI `linker_flags` interplay with
  per-archive `-l` order?).
- Does `may` (green threads) pull anything feature-worthy on its own?
- CI story for aarch64: native arm runners vs QEMU — separate decision.
