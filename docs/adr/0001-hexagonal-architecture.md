# 0001 — Hexagonal architecture across Rust and Flutter

- Status: Accepted
- Date: 2026-11-13

## Context

Platine mixes hard real-time audio, file I/O, analysis jobs and a rich UI. The
auto-mix logic (queue, shuffle, crossfade scheduling) is the core value and must
be testable deterministically, without audio hardware or files.

## Decision

Ports and adapters, with dependencies pointing inwards:

- `dj_domain` — entities, value objects, auto-mix state machine and the ports
  (`AudioOutput`, `TrackDecoder`, `TrackAnalyzer`, `LibraryScanner`, `Clock`,
  `RandomSource`). Depends on nothing but `thiserror`.
- `dj_engine` — real-time graph. Depends on `dj_domain` only.
- `dj_infra` — adapters (cpal, symphonia, rubato, scanner, analysis, SQLite).
- `dj_api` — composition root and façade for Flutter. The only crate that
  knows about `flutter_rust_bridge`.

On the Flutter side, features are split into `presentation / application /
domain / data`. Generated bridge types never leave `data/`; repositories map
them to Dart models. Riverpod providers are the dependency-injection container.

Tooling is pinned to the latest stable releases at project start: Rust 1.100.0
(edition 2024), Flutter 3.50.0. Bumps happen in dedicated commits.

## Consequences

- The domain is unit-testable with fake `Clock` / `RandomSource`; the engine is
  testable through an offline renderer.
- Some mapping boilerplate between bridge types and Dart models.
- A test guards the bridge boundary on the Dart side; crate dependencies guard
  it on the Rust side.
