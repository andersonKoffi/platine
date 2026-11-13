# 0003 — Real-time threading model

- Status: Accepted
- Date: 2026-11-13

## Context

The audio callback must never allocate, lock, do I/O, log or panic. Decoding,
analysis and UI traffic must therefore live elsewhere, and the auto-mix
crossfade must start within one buffer of its target.

## Decision

Threads:

1. **Audio callback** (`dj_engine`, driven by cpal or the offline renderer):
   mixes both decks, applies smoothed parameters (≈10 ms ramps), meters levels.
   Reads `EngineCommand`s from an `rtrb` SPSC queue, writes `EngineEvent`s to
   another. Everything is preallocated.
2. **One decoder thread per deck** (`dj_infra`): symphonia → rubato (to the
   output rate) → `rtrb` sample ring with **≥ 2 s** of look-ahead. Seeks and
   loads bump a generation counter so the callback discards stale samples.
3. **Controller thread** (`dj_api`): owns the domain objects (queue, shuffle,
   `AutoMixScheduler`), consumes engine events, sends commands, and forwards
   events to Flutter throttled to ~30 Hz.
4. **Analysis thread(s)**: silence, BPM, waveforms, cached in SQLite.

Transitions are **scheduled ahead**: once the standby deck is ready and the
active track's `effectiveEnd` is known, the controller sends
`ScheduleTransition { at_frame, duration_frames, curve }`. The callback starts
it sample-accurately. A manual intervention sends a cancel command.

Pitch and keylock are applied **in the callback** while reading the ring
(Hermite interpolation, or preallocated Signalsmith Stretch with keylock), so
tempo changes are immediate despite the 2 s ring.

## Consequences

- Crossfade timing does not depend on UI or controller jitter.
- A no-allocation test (`assert_no_alloc`) and criterion benchmarks guard the
  callback.
- Ring capacity must cover 2 s at the maximum playback rate (+50 %).
