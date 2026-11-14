# Platine

A beautiful, fun DJ app for macOS (Apple Silicon and Intel). Flutter UI, Rust real-time audio engine.

> Status: **P4 — UI MVP**. Neon dark theme, animated vinyl decks, mixer (crossfader,
> 3-band kill EQ, VU meters), auto-mix panel, searchable library with drag-to-reorder
> queue, Finder drag and drop, keyboard shortcuts (press `?`). See ADR 0008.
> Next: P5 (BPM, waveforms, SQLite cache, pitch/keylock).

## Repository layout

```
native/            Cargo workspace (Rust, edition 2024)
  dj_domain/       Pure domain: entities, auto-mix state machine, ports. No I/O, no audio.
  dj_engine/       Real-time audio graph (decks, EQ, crossfader, master, meters).
  dj_infra/        Adapters: cpal, symphonia, folder scan, analysis, SQLite cache.
  dj_api/          Façade exposed to Flutter (flutter_rust_bridge).
  test_support/    Test fixtures and fakes.
app/               Flutter macOS app (Riverpod)
  lib/core/        Bridge bootstrap, cross-cutting features (diagnostics, later design system).
  lib/features/    decks, mixer, library, automix, settings — each split into
                   presentation / application / domain / data.
  rust_builder/    cargokit plugin that compiles native/dj_api into the app.
docs/adr/          Architecture Decision Records.
tool/              Bootstrap and code-generation scripts.
```

## Prerequisites

| Tool | Version |
|---|---|
| macOS + Xcode | recent macOS, latest stable Xcode (with command-line tools) |
| CocoaPods | latest (`brew install cocoapods`) |
| Flutter | 3.50.0 stable |
| Rust | 1.100.0 (pinned by `native/rust-toolchain.toml`, installed by rustup automatically) |
| flutter_rust_bridge_codegen | 2.13.0 — `cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked` |

## First run

```bash
make bootstrap   # macOS runner scaffold, pub get, bridge + Riverpod codegen
make run         # launches Platine
```

## Everyday commands

```bash
make check             # cargo fmt/clippy/test + dart format/flutter analyze/test
make gen               # after changing native/dj_api/src/api or Riverpod providers
make test-integration  # integration tests against the real Rust engine
make help              # everything else
```

Hear the engine without the UI:

```bash
cd native
# Auto-mix a whole folder (8 s fades; add --shuffle and/or --repeat)
cargo run --release -p dj_api --example automix -- ~/Music/Set 8 --shuffle
# Crossfade two files at a given time
cargo run --release -p dj_infra --example mix -- ~/Music/a.mp3 ~/Music/b.flac 30 8
```

## Generated code policy

- **Committed:** the bridge glue (`app/lib/src/rust/**`, `native/dj_api/src/frb_generated.rs`),
  so that the Rust workspace builds on its own. CI regenerates it and fails on any diff.
- **Not committed:** `*.g.dart` / `*.freezed.dart` (Riverpod, freezed). Run `make gen`.

## Conventions

- Code, comments, commits (Conventional Commits) and docs in English.
- No `unwrap()` / `expect()` / `panic!` outside tests (enforced by clippy).
- Generated bridge types are only imported from `data/` adapters (enforced by
  `app/test/architecture/bridge_boundary_test.dart`).
- No GPL/AGPL dependency.
- Every structural decision gets an ADR in `docs/adr/`.

## CI

GitHub Actions on `macos-latest`: Rust fmt/clippy/tests, line coverage ≥ 90 % on `dj_domain`
and ≥ 70 % overall (`cargo llvm-cov`), bridge codegen freshness,
`flutter analyze`, `flutter test`, universal release build (checked with `lipo`),
and integration tests.
