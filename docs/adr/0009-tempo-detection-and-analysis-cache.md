# 0009 — Tempo detection and analysis cache

- Status: Accepted
- Date: 2026-11-14

## Context

P5 shows the tempo of every track (decks, library column and sort) and
needs it for SYNC. The analysis pass (ADR 0007) already decodes each file
once, for silences and the waveform. Re-analysing a folder at every launch
costs seconds per track.

## Decision

- **Tempo estimation is pure domain code** (`dj_domain::tempo`), fed by the
  analysis pass:
  1. `OnsetDetector` builds an onset-strength envelope (5 ms hops): rises of
     compressed energy, with the low band (< 150 Hz, kicks) weighted over
     the full band.
  2. `estimate_beat_grid` detrends and smooths the envelope, takes the best
     autocorrelation lag between 60 and 200 BPM under a mild log-normal
     preference around 120 BPM, folds the result into **80–160 BPM** (as
     most DJ software does: 174 → 87, 70 → 140), then scans ±3 % in
     0.01 BPM steps with a comb aligned on every phase. The best comb gives
     the tempo and the **first beat** (`BeatGrid`).
  - Tested on synthetic drum loops from 70 to 174 BPM: ±0.05 BPM, first
    beat within 20 ms. Tracks without a steady beat get no tempo.
- **Analyses are cached in SQLite** (`dj_infra::cache`, `rusqlite` with the
  bundled SQLite: MIT / public domain). `CachedAnalyzer` decorates any
  `TrackAnalyzer`; an entry is reused while the file size, its modification
  time, the analysis options and `ANALYSIS_VERSION` are unchanged. The
  database lives in the application data folder
  (`~/Library/Application Support/Platine`, inside the sandbox container).
  Any cache error falls back to a fresh analysis: the cache is never
  load-bearing. Tests run without it.

## Consequences

- A folder analysed once reloads instantly, tempo included.
- Changing the analysis algorithms requires bumping `ANALYSIS_VERSION`.
- The 80–160 BPM fold suits the target repertoire (afro, house, gospel,
  coupé-décalé); a configurable range can come later.
