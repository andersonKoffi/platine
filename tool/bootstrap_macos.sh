#!/usr/bin/env bash
# Generates the parts of the macOS runner that are owned by `flutter create`
# (Xcode project, workspace, Podfile...). Files already in git (entitlements,
# AppInfo.xcconfig, MainFlutterWindow.swift) are never overwritten.
set -euo pipefail
cd "$(dirname "$0")/../app"

if [ -d macos/Runner.xcodeproj ]; then
  echo "macOS runner already complete."
  exit 0
fi

flutter create . --platforms=macos --project-name platine --org com.sophosstudios
# `flutter create` adds a counter-app sample test we do not want.
rm -f test/widget_test.dart
