# 0006 — Accept symphonia under MPL-2.0

- Status: Accepted (approved by the project owner on 2026-11-13)
- Date: 2026-11-14

## Context

The project avoids GPL/AGPL dependencies. symphonia, the only mature pure-Rust
decoder covering MP3, FLAC, WAV, AIFF, AAC/M4A, ALAC and Ogg, is licensed
under the Mozilla Public License 2.0.

## Decision

Use symphonia 0.6 unmodified, as a crates.io dependency.

## Consequences

- MPL-2.0 is a file-level copyleft: obligations only apply to modified
  symphonia source files, which we do not modify. Platine's own code keeps
  its license.
- If a symphonia file ever needs patching, the patched file must be shared
  under MPL-2.0 (a fork published on GitHub is enough).
- The about screen (P6) lists third-party licenses, symphonia included.
