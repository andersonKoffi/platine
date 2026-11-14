# 0008 — UI architecture, design system and native folder access

- Status: Accepted
- Date: 2026-11-14

## Context

P4 delivers the first real interface on top of the session API (ADR 0007).
It must stay testable without the native engine, follow the layering of
ADR 0001, look like a neon/glass "club" app, and let the user pick or drop a
music folder inside the macOS sandbox.

## Decision

- **One session store, many views.** `SessionStore` (Riverpod) subscribes to
  the `EngineGateway` event stream and folds events into an immutable
  `SessionState` with a pure `reduceSession` function. Widgets select the
  slices they need; commands go through small feature controllers
  (`DeckControlsController`, `CrossfaderController`, `MasterGain`,
  `LibraryLoader`, `MixSettingsController`). Only
  `core/engine/data/frb_engine_gateway.dart` touches generated bridge types.
- **Design tokens as a `ThemeExtension`.** `PlatineTokens` holds colours,
  glows, radii, spacing and motion durations; widgets read
  `context.tokens` and never hard-code values. Reusable primitives
  (`GlassPanel`, `NeonIconButton`, `VuMeter`) live in
  `core/design_system`. Space Grotesk (OFL) is bundled; numbers use tabular
  figures. `context.reduceMotion` disables vinyl spin and celebrations
  when the system asks for reduced motion.
- **Smooth visuals from coarse events.** The engine reports positions and
  levels about 30 times per second; the vinyl extrapolates its angle with a
  `Ticker` between updates instead of asking for more events.
- **Native folder picker.** A tiny `MethodChannel`
  (`platine/folder_picker`) opens an `NSOpenPanel` sheet. It replaces
  `file_selector`, whose `cross_file` constraint conflicts with
  `desktop_drop`. The user-selected entitlement grants read access to the
  chosen folder for the session; persistent access (security-scoped
  bookmarks via `macos_secure_bookmarks`) is planned for P6.
- **Drag and drop behind a seam.** `desktop_drop` (MIT) provides Finder drops.
  The shell gets its drop zone from `dropZoneProvider`; tests override it
  with a pass-through because the plugin needs its native library, which is
  missing in `flutter test`.
- **Keyboard first.** All transport and mixer actions have shortcuts
  (`CallbackShortcuts`), listed in an in-app help sheet (`?`).

## Consequences

- Widget tests run against `FakeEngineGateway` with no native code; the real
  engine is covered by `integration_test/`.
- Adding a UI feature means a controller plus a widget; the reducer stays the
  single place where engine events change state.
- Folder access does not survive a restart until P6 adds bookmarks.
