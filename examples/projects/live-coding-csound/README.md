# Seq → OSC → Csound (live-coding POC)

This directory is the Phase C POC from
[`docs/design/LIVE_CODING_CSOUND_POC.md`](../../../docs/design/LIVE_CODING_CSOUND_POC.md):
prove that Seq can drive an external audio engine (Csound) over OSC well
enough to support live-coding music.

## What's here

| File | Purpose |
|---|---|
| `osc.seq` | OSC 1.0 encoder, written in Seq. Library, no `main`. |
| `test_osc.seq` | Byte-exact unit tests for the encoder. |
| `test_osc_loopback.seq` | End-to-end UDP round-trip tests (no audio). |
| `live.csd` | Csound orchestra: OSC listener on port 7770 + kick instrument. |
| `tone.seq` | One-shot driver — sends a single `/kick 220.0` message. |
| `live.seq` | 8-beat metronome driver — sends 8 evenly-spaced kicks. |

## Audible run

The encoder/loopback tests run in CI (`just ci`). The audible parts
below need a working Csound install on your machine.

### 1. Install Csound

macOS (Homebrew):

```sh
brew install csound
```

Linux (Debian/Ubuntu):

```sh
sudo apt-get install csound
```

Verify:

```sh
csound --version
```

### 2. Start the listener

In one terminal, from the repo root:

```sh
csound -odac examples/projects/live-coding-csound/live.csd
```

`-odac` writes audio to your default output device. You should see
Csound print its banner, list the instruments, and sit waiting (last
line will say something like `SECTION 1:` followed by no further
output). Leave this terminal running.

### 3. Send one kick (`tone.seq`)

In a second terminal, build and run the one-shot driver:

```sh
just build
target/examples/projects-live-coding-csound-tone
```

You should hear a single short percussive tone at 220 Hz. The Seq
process exits immediately; Csound stays up so you can run again.

### 4. Run the metronome (`live.seq`)

```sh
target/examples/projects-live-coding-csound-live
```

You should hear 8 evenly-spaced kicks over roughly 4 seconds (120 BPM).

### 5. Live-coding loop

Edit `live.seq` (e.g. change `bpm-ms` from `500` to `250` for 240 BPM,
or `beats` from `8` to `16`), then run `just build` and re-execute
the binary. Csound keeps running between Seq runs, so the iteration
cycle is just edit → build → re-run.

To stop everything: `Ctrl+C` in the Csound terminal.

## Troubleshooting

**Nothing audible after `tone`/`live`.** Check the Csound terminal:
when an OSC message lands, Csound prints something like
`new alloc for instr 2:` and `ihold:` lines. If those are absent,
the message isn't reaching Csound. Verify the port matches (Csound
listens on 7770; Seq sends to 7770).

**`csound: command not found`.** Install per step 1 above.

**`Address already in use` from Csound.** Another process holds port
7770. `lsof -i :7770` to find it; kill the holder or change the port
in *both* `live.csd` and the `7770` literal in `tone.seq` / `live.seq`.

**Audio cuts out / glitches.** Csound's default audio backend can be
finicky. Try `csound -odac0 live.csd` to force the system default
device, or pass `-+rtaudio=...` to pick a specific backend (CoreAudio
on macOS, ALSA/JACK on Linux).

## How this fits the design doc

- **Checkpoint 1 (UDP loopback)** — covered by `test_osc_loopback.seq`,
  runs in CI.
- **Checkpoint 2 (OSC fixture test)** — covered by `test_osc.seq`,
  runs in CI.
- **Checkpoint 3 (Csound responds, one tone)** — `tone.seq` + this
  README. Manual verification.
- **Checkpoint 4 (a bar of music)** — `live.seq` + this README.
  Manual verification.
- **Checkpoint 5 (POC writeup decides spinout question)** — once
  you've run the metronome a few times and exercised the
  edit-build-rerun loop, the design doc gets a final block recording
  what worked, what surfaced as a Seq language gap, and whether the
  whole thing is worth spinning into its own repo.
