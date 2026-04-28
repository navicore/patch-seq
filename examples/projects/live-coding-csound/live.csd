<CsoundSynthesizer>
;
; live.csd — minimal OSC-driven kick synth for the Seq POC.
;
; Listens on UDP port 7770 for OSC messages addressed to /kick with one
; float argument (the pitch in Hz). Each message triggers a short
; percussive tone at that pitch.
;
; Usage:
;   csound -odac live.csd
;
; Then in another terminal, run the matching Seq driver (tone.seq for
; one beat, live.seq for an 8-beat metronome).
;
<CsOptions>
-odac
</CsOptions>
<CsInstruments>

sr      = 44100
ksmps   = 64
nchnls  = 2
0dbfs   = 1

; Sine table for the kick body.
gisine  ftgen   1, 0, 4096, 10, 1

; Open the OSC listener once at score time.
gihandle OSCinit 7770

; --------------------------------------------------------------------------
; instr 1 — always-on OSC listener.
;
; Pulls one /kick message per iteration and schedules instr 2 to play it.
; Note: event "i", ... is k-rate, so the kgoto loop polls OSClisten and
; only fires when a new message arrives.
; --------------------------------------------------------------------------
instr 1
  kfreq   init 0
loop:
  kk      OSClisten gihandle, "/kick", "f", kfreq
  if (kk > 0) then
    event "i", 2, 0, 0.4, kfreq
  endif
          kgoto loop
endin

; --------------------------------------------------------------------------
; instr 2 — short pitched tone with a percussive amp envelope.
;
; p4 = pitch in Hz. Envelope decays to silence over the note's duration
; (p3, 0.4 s in this POC) so successive kicks don't smear together.
; --------------------------------------------------------------------------
instr 2
  ifreq   = p4
  kenv    expon 0.5, p3, 0.001
  asig    oscili kenv, ifreq, 1
          outs asig, asig
endin

</CsInstruments>
<CsScore>
; Run instr 1 indefinitely (negative duration). The score below pads the
; session out to 24 hours so the engine stays alive while you live-code.
i 1 0 -1
f 0 86400
e
</CsScore>
</CsoundSynthesizer>
