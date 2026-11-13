# 0002 — flutter_rust_bridge v2 with cargokit, crate outside the app

- Status: Accepted
- Date: 2026-11-13

## Context

Flutter must call the Rust engine and receive event streams (positions, VU
levels, deck states). The Rust workspace lives in `native/`, next to the app,
not inside it.

## Decision

- `flutter_rust_bridge` **2.13.0**, pinned with `=` on both sides (crate and
  Dart package must match exactly). Streams (`StreamSink`) carry engine events.
- Only `dj_api::api` is scanned by the code generator (`app/flutter_rust_bridge.yaml`).
- Build integration through the FRB **cargokit** template, copied into
  `app/rust_builder` and trimmed to macOS. Its podspec points to
  `../../../native/dj_api` and statically links `libdj_api.a` (`-force_load`).
  cargokit builds every architecture Xcode requests, which yields a universal
  binary for release builds.
- Generated glue (`app/lib/src/rust`, `native/dj_api/src/frb_generated.rs`) is
  **committed** so `cargo` works without Flutter; CI regenerates it and fails on
  diff. Riverpod/freezed outputs are **not** committed (`make gen`).
- The generated Rust module is isolated with `#[allow(...)]` for `unsafe_code`
  and clippy groups; the rest of the workspace keeps `unsafe_code = "deny"` and
  denies `unwrap`/`expect`/`panic`.

## Alternatives considered

- Native-assets integration (Dart build hooks): still young for Flutter macOS
  apps embedding a workspace crate; revisit later.
- Hand-written `dart:ffi` + `ffigen`: no streams, much more glue.

## Consequences

- One `make gen` after any API change.
- The cargokit copy is vendored; upgrading FRB means re-copying the template.
