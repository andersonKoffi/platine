# 0007 — Session controller: analyse first, then load at the audible start

- Status: Accepted
- Date: 2026-11-14

## Context

The auto-mix scheduler (ADR 0004) needs each track's audible boundaries
before it can plan a transition, and the engine (ADR 0005) needs the
incoming deck already positioned on the first audible sample when the fade
starts. Something has to translate between the scheduler's commands, the
engine, the decoder threads and the UI.

## Decision

- `dj_api::session::Controller` is the composition root. It owns the
  scheduler, the engine handle, the two deck loaders, an analysis worker and
  the library, and runs on its own thread (`SessionHandle`), reacting to
  commands, worker reports and engine events (polled every 5 ms).
- **Analyse, then load.** On `LoadTrack`, the controller first runs the
  silence analysis (full decode, 10 ms windows, threshold −50 dBFS by
  default, configurable). It then asks the loader to decode **from the
  effective start** and cues the engine there. Decks are therefore always
  cued on their first audible sample: `Play` and transitions need no seek,
  and the leading silence is never played.
- `TrackLoaded` reaches the scheduler only when both the analysis and the
  loader's prefill (0.5 s) are done. Analysis or decoding failures become
  `TrackLoadFailed`, so a broken file is skipped without stopping the music.
- Every load or seek bumps the deck's generation; reports of older
  generations are ignored.
- Manual actions (crossfader, play/pause, seek, manual load) send
  `ManualIntervention` first; gain, EQ and master do not suspend the auto-mix.
- UI events (`SessionEvent`) are throttled to ~30 Hz for positions and levels.
  The FRB API (`api::mix`) maps them to DTOs and streams them to Dart.
- Shuffle uses a time-seeded xorshift generator; tests inject a seeded one.

## Consequences

- The whole chain is tested offline with generated WAV files: automatic
  loading of the next track, A/B role swaps, silence trimming, corrupt files,
  shuffle, suspension and resume.
- Starting a track costs one extra decode pass (analysis). It is cheap for
  local files (well under a second) and P5 caches the results in SQLite.
- The CLI example `automix` plays a whole folder without the UI.
