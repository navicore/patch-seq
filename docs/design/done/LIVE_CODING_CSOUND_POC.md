# Live-Coding POC: Seq → OSC → Csound

Status: Phase A and Phase B implemented · PR #434 (UDP runtime),
#436 (string byte-cleanliness), #437 (OSC encoder + byte
construction primitives), #439 (OSC-over-UDP loopback) · 2026-04-27 ·
Phase C (Csound `live.csd` + audible kick) remains.

## Intent

Validate that Seq is a credible host language for live-coding music by
driving an external audio engine (Csound) over OSC. The architecture
question — "can Seq's strands+channels schedule events tightly enough
when an audio engine handles sample-accurate playback?" — is more
important than any specific musical result. If the POC works, it seeds
a separate project later; if it doesn't, the same UDP work that
unblocks it also unblocks DNS, NTP, multicast, syslog, and any other
common datagram protocol — so the dependency work pays off either way.

The POC should live in **`patch-seq/examples/projects/live-coding-csound`**. A
live-coding POC belongs with the showcases.

## Constraints

- **UDP support must land cleanly.** It will outlive this POC — DNS
  resolvers, NTP clients, multicast discovery, syslog all want it.
  Don't shape `udp.*` to "what the POC happens to need."
- **No new Seq stdlib for music yet.** Let the POC tell us what's
  actually needed. OSC encoding can live inside the example dir until
  it proves stable enough to promote to `std:osc`.
- **Out of scope:** MIDI, SuperCollider integration, audio rendering,
  pattern-language design, packaging Csound. The user installs Csound
  themselves; the POC documents *how to start the engine*, not
  *how to ship one*.
- **Out of scope:** answering the spinout question now. POC first;
  decide based on what it teaches us.
- **Don't spin out a separate project at the start.** A POC inside
  `patch-seq/examples/` keeps the language and its first non-trivial
  external integration close together — language gaps it surfaces get
  fixed in the same repo without coordination overhead. Spin out only
  when the integration outgrows "one Seq script + one .csd file."

## Approach

Three sequential phases, each independently valuable.

**Phase A — UDP in patch-seq.** Add `udp.bind`, `udp.send-to`, and
`udp.receive-from` to the runtime, mirroring the shape of the existing
`tcp.*` words but with no connection state. Stack effects (rough):

```
udp.bind          ( port           -- socket success )
udp.send-to       ( bytes host port socket -- success )
udp.receive-from  ( socket          -- bytes host port success )
```

Tests in patch-seq that cover loopback send/receive. This phase is
useful independent of music — file the music POC as the motivating
example but don't gate on it.

**Phase B — OSC encoder, in Seq, in the example dir.** OSC 1.0 is a
small spec: type-tagged byte messages with simple alignment rules.
Write the encoder in Seq itself (no FFI, no new builtin). This stress-
tests `udp.send-to` and exercises real-world byte-packing in Seq.
Write a Seq message like `"/synth/play" [ 220.0 0.5 ] osc.send`.

**Phase C — Csound example.** A `live.csd` Csound orchestra that opens
an OSC port and triggers an instrument per message. A `live.seq`
driver that opens UDP, sends a tick-driven pattern. A `README.md` that
walks the user through `brew install csound`, starting the engine
(`csound -odac live.csd`), and running the Seq script. Success
criterion: a kick on every beat for 8 beats, BPM controlled by a Seq
literal.

## Domain Events

- **Input:** `LiveCodingPocRequested { engine: Csound, transport: OSC }`
- **Phase A:** `UdpLanded { stdlib: udp, has_tests: true }`
- **Phase B:** `OscEncoderProven { lives_in: example_dir, vendored: true }`
- **Phase C:** `CsoundExampleRuns { audible_on_macos: true }`
- **Aggregate:** `PocEvaluation { result: Spinout | FoldIntoStdlib | Shelved }`
- **Downstream:** if `Spinout`, a new repo gets `osc.seq` + a pattern
  vocab + a packaging story. If `FoldIntoStdlib`, the OSC encoder
  graduates to `std:osc` in patch-seq. If `Shelved`, UDP and the
  example sit on disk for the next attempt — no harm done.

## Checkpoints

1. **[done · PR #439]** UDP loopback test passes in patch-seq: send
   a message to yourself, receive it back, round-trip is byte-exact.
   Lives at `examples/projects/live-coding-csound/test_osc_loopback.seq`
   and runs as part of `just test-integration`.
2. **[done · PR #437]** OSC fixture test passes: encoded OSC messages
   from Seq match the byte layout in the OSC 1.0 spec for `,i`, `,f`,
   `,if`, and empty-arg payloads. Pinned in
   `examples/projects/live-coding-csound/test_osc.seq`.
3. **[done · 2026-04-27]** Csound responds: starting `live.csd` and
   sending one OSC message from `tone.seq` produces an audible tone.
   `printks` confirmed the message round-trip end-to-end.
4. **[done · 2026-04-27]** A bar of music plays: `live.seq` (120 BPM,
   8 beats) sends 8 OSC messages with `time.sleep-ms` between them;
   you hear 8 evenly-spaced kicks. No audible jitter at this rate.
5. **[done — see "POC outcomes" below]** POC writeup with evidence.

## Open Question

Whether the example should target Csound or SuperCollider for the
first cut. Csound is *one* binary you can start from a script;
SuperCollider's scsynth is more modular but requires a slightly more
complex install story. Going with Csound for the POC; revisit once
UDP+OSC work — the engine choice is interchangeable at the OSC layer.

## POC outcomes

First-pass writeup of what the POC surfaced. Authored by Claude as a
draft for the human author to redact, expand, or replace; the spinout
recommendation at the end is deliberately blank because that's a taste
call, not an evidence call.

### What worked

- **End-to-end is correct.** `tone.seq` produces one audible kick;
  `live.seq` produces 8 evenly-spaced kicks at 120 BPM. The OSC wire
  format is byte-exact (locked in by `test_osc.seq`) and Csound's
  `OSClisten "/kick", "f"` parses it without complaint.
- **Iteration cycle is acceptable.** Csound's `i 1 0 -1` keeps the
  listener instrument alive across Seq runs, so the loop is
  edit-`live.seq` → `just build` → re-run-binary. The build step is
  the slowest part (full Rust → LLVM → linker chain); empirically it
  felt fast enough to keep flow.
- **No audible jitter at 120 BPM.** `time.sleep-ms` plus a UDP
  loopback hop comfortably hits 500 ms-spaced beats. Csound's
  `rtevent` log showed inter-event time of exactly 0.5 s with no
  drift visible.

### Language gaps surfaced

These are concrete things that bit while writing the POC:

1. **Stack-effect annotations don't accept parameter names.**
   Tried `( socket -- socket )` and `( count -- pad-bytes )` twice;
   both rejected with `Unknown type: 'count'`. Lowercase identifiers
   in `( ... )` annotations are interpreted as types, not parameter
   names. Worked around by using type names (`Int`, `String`) which
   loses the documentation value of distinguishing between two `Int`s
   that mean very different things ("count" vs "socket handle"). A
   future enhancement could allow `( count: Int -- pad-bytes: String )`-
   style annotations purely for docs.

2. **Stack juggling for fixed-position multi-arg calls.** `udp.send-to`
   takes `( bytes host port socket -- ok )`. Threading those four
   values into position from a typical caller-state cost three
   `swap`s and a `rot` per call site (see `send-kick` in `live.seq`).
   This compounds for any builtin with three-or-more fixed args. Two
   directions help: a richer set of stack shufflers (`tuck`, etc.
   exist; might benefit from a `4-arg` variant), or a per-call-site
   pattern of stashing on `aux` and pulling args back in order. Both
   exist today; neither felt clean enough that the author reached
   for them on the first draft.

3. **`udp.send-to` argument order is awkward for the typical use.**
   Most calls have a long-lived socket handle and an ephemeral
   payload. Putting the socket *last* on the stack means that
   handle has to be juggled past the host/port/payload args every
   time. `( socket host port bytes -- ok )` (socket first) would
   compose more naturally for `dup ... udp.send-to` patterns. Not
   suggesting a breaking change, but worth noting if a redesign is
   ever on the table.

4. **No `osc.msg-N` family for arbitrary type tags.** The encoder
   ships `osc.msg`, `osc.msg-i`, `osc.msg-f`, `osc.msg-if`. A real
   music driver would also want `,iif`, `,iff`, `,sif`,
   `,fff`, etc. — every combination of basic types. Each currently
   needs a hand-written word with the same shape. A combinator-based
   message builder (think `osc.start-msg "/foo" then ,if-typetag
   1000 osc.append-i 220.0 osc.append-f osc.finish`) would scale
   without N^k explosion.

5. **No locals; recursive helpers for state.** `send-n-kicks` in
   `live.seq` is tail-recursive on `(count, socket)` because the
   language has explicitly rejected locals. The recursion is fine,
   but every state-holding loop has to be hand-coded as its own
   recursive word. The language's existing `times` combinator
   doesn't pass values through, and quotation auto-capture solves
   the related-but-different case of "extra args at call time."
   This is consistent with `feedback_no_generics.md` / "loops and
   locals explicitly rejected" — the author flags it not as a
   regression but as a knock-on cost of that earlier decision.

6. **Comment-pattern parser bug.** A specific combination of
   multi-line `#` header + section divider + per-test pre-comments
   inside the same file injects a spurious `Bool(false)` onto the
   runtime stack. Surfaced during CP6 of byte-cleanliness; not yet
   reproducible in a minimal case. Tracked in
   `project_comment_parser_bug` memory. Worked around by keeping
   POC comments terse.

### Code size

| File | Lines | Purpose |
|---|---|---|
| `osc.seq` | 65 | encoder library |
| `tone.seq` | 25 | one-shot driver |
| `live.seq` | 35 | metronome driver |
| `live.csd` | 50 | Csound orchestra |
| `README.md` | ~100 | install + run instructions |

About 275 lines total, ~half of which is comments and section
dividers. For comparison: a Python `python-osc` + Sonic-Pi-style
driver hits roughly the same line count for an equivalent demo.

### Latency

Not measured numerically. Subjectively: zero perceived delay between
`target/.../tone` exiting and the kick being audible. Adding a
proper measurement would mean `time.nanos` on the Seq side just
before `udp.send-to` and a Csound-side timestamp on receipt — both
exist, this just wasn't done yet.

### Spinout recommendation (deferred)

The design doc's question is whether to (a) spin this out into its own
repo, (b) fold the encoder into `std:osc` and keep the example here,
or (c) shelve. The author does not have enough taste-context to make
this call — that's the human author's decision and depends on the
broader vision for Seq's relationship to music tooling. The evidence
above is what was meant to be supplied.
