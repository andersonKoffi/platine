# 0010 — Pitch, keylock and SYNC

- Status: Accepted
- Date: 2026-11-14

## Context

P5 adds a pitch fader per deck (±8/16/50 %), keylock (change the tempo
without changing the key) and SYNC (match the tempo of the other deck).
Speed changes must be instant and click-free, so they belong to the audio
thread (ADR 0003): resampling in the decoder threads would lag behind the
fader by the whole look-ahead.

## Decision

- **Varispeed in the voice.** Each deck reads its source at a smoothed rate
  with Catmull-Rom interpolation over a four-frame history. At rate 1.0 the
  fraction stays 0 and frames pass unchanged. Positions keep counting
  *source* frames (the frame heard next), so transition plans expressed in
  track time (ADR 0004) stay valid at any pitch.
- **Keylock with Signalsmith Stretch** (`signalsmith-stretch` crate and the
  C++ library, both MIT). With keylock on, the voice pulls
  `rate × 256` source frames per 256-frame block through the stretcher
  (transpose factor 1). Buffers are allocated when the engine is built; the
  render path stays allocation-free (`assert_no_alloc` covers it with
  keylock on). Switching keylock fades the deck out and back in (10 ms) to
  avoid clicks. Reported positions subtract the stretcher latency
  (≈ 0.2 s with the "cheaper" preset).
  - Rejected: Rubber Band (GPL), SoundTouch (LGPL, static linking issues),
    a home-made WSOLA (audible artefacts on music).
  - Consequence for the macOS build: the plugin links `libc++`
    (`s.libraries = 'c++'` in the podspec); building needs libclang
    (bindgen), which Xcode provides.
- **SYNC is a pure UI-side computation** (`syncPitch`): the pitch that makes
  the deck play at the tempo heard on the other deck, allowing half and
  double tempo, widening the pitch range when needed. Phase alignment is not
  done yet.

## Consequences

- Pitch and keylock work on any deck, including during auto-mix
  transitions; the auto-mix itself does not change pitches.
- Beat-matched auto-mix (tempo and phase alignment during transitions) can
  build on the beat grid (ADR 0009) and these controls.
