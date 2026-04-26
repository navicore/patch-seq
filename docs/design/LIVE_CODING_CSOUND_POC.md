# Live-Coding POC: Seq → OSC → Csound

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

1. **UDP loopback test passes** in patch-seq: send a message to
   yourself, receive it back, round-trip is byte-exact.
2. **OSC fixture test passes**: an encoded OSC message from Seq
   matches the byte layout in the OSC 1.0 spec for a known input
   (e.g. `/foo` with one int and one float).
3. **Csound responds**: starting `live.csd` and sending one OSC
   message from a one-line Seq script produces an audible tone.
4. **A bar of music plays**: a clock-driven Seq strand sends 8 beats
   on a metronome; you hear them on time, no audible jitter.
5. **POC writeup decides the spinout question** with evidence
   (latency measurements, code size of the example, Seq language
   gaps surfaced).

## Open Question

Whether the example should target Csound or SuperCollider for the
first cut. Csound is *one* binary you can start from a script;
SuperCollider's scsynth is more modular but requires a slightly more
complex install story. Going with Csound for the POC; revisit once
UDP+OSC work — the engine choice is interchangeable at the OSC layer.
