#!/usr/bin/env bash
# Regenerates the flutter_rust_bridge glue (Rust + Dart) and formats it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT/app"
flutter_rust_bridge_codegen generate --no-build-runner

cd "$ROOT/native"
cargo fmt --all
