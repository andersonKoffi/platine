# 0005 — Audio engine and decoding pipeline

- Status: Accepted
- Date: 2026-11-14

## Context

ADR 0003 fixed the thread model. This ADR records how audio moves through it
and the DSP choices of the MVP engine.

## Decision

**Chunks with generations.** Decoder threads push `AudioChunk`s (512 stereo
frames at the engine rate, ~4 KB, `Copy`) into one `rtrb` ring per deck,
sized for 3 s of look-ahead. Every load or seek gets a new `generation`; the
deck voice drops chunks of older generations and waits for newer ones until
the matching `Cue` command arrives. Seeks therefore never play stale audio
and need no lock. A chunk flagged `end_of_stream` lets the engine report
`TrackEnded` exactly.

**Decoding.** symphonia 0.6 (pure Rust; MPL-2.0, see ADR 0006) decodes MP3,
FLAC, WAV, AIFF, AAC/M4A, ALAC and Ogg Vorbis. The first packet is decoded
when opening, so corrupt files fail before anything is queued. Seeks are
sample-accurate (`SeekMode::Accurate` + skipping the remainder).

**Resampling.** rubato 5 `Fft` (synchronous, fixed input) converts each
track to the output device rate in the decoder thread, trimming the
resampler delay and flushing exactly `round(frames × ratio)` frames.

**Engine DSP (per frame, allocation-free):**
- click-free transport: a 10 ms envelope on play/pause; a seek while playing
  fades out, switches generation, fades in;
- 10 ms linear ramps on deck gain, EQ bands, crossfader and master;
- 3-band **isolator** EQ from Linkwitz-Riley 4 crossovers (250 Hz / 2.5 kHz)
  plus an all-pass on the low band: flat at unity, true kill at 0;
- crossfader with the domain curves; transitions run as an automation
  started when the outgoing deck reaches `start_at_frame` (checked every
  frame, so sample-accurate), reported with the engine clock;
- per-deck and master peak meters, positions every 1024 frames.

**Output.** cpal 0.18 on the default device. A supervisor thread owns the
stream; on any error other than `DeviceChanged` (which cpal handles by
following the default route) it rebuilds the stream on the new default
device with the same render callback, held in an `Arc<Mutex<_>>` that only
the current stream touches (`try_lock`, never contended).

## Consequences

- `cargo test` covers the whole chain offline (generated WAV fixtures, real
  loader threads, offline output): fade start on the exact sample, no
  discontinuity, end-of-track, corrupt files, seeks, mono files.
- A test with `assert_no_alloc` fails if the callback ever allocates; a
  criterion benchmark tracks its cost (~43 µs per 512 frames on CI-class
  hardware, i.e. < 0.5 % of the real-time budget at 48 kHz).
- Pitch/keylock (P5) will read chunks at a variable rate in the voice; the
  chunk format already carries absolute positions to support it.
- Known limit: if a new default device rejects the engine sample rate, the
  stream keeps retrying; adapting the rate on the fly is deferred to P6.
