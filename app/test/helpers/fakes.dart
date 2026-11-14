import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/app.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/engine_gateway.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/library/application/library_controller.dart';
import 'package:platine/features/settings/application/settings_controller.dart';
import 'package:platine/features/settings/domain/settings_repository.dart';
import 'package:platine/features/shell/application/drop_zone_provider.dart';

/// Records every command; events are pushed by the test.
class FakeEngineGateway implements EngineGateway {
  final StreamController<EngineEvent> _events =
      StreamController<EngineEvent>.broadcast(sync: true);
  final List<String> calls = [];
  MixSettings? startedWith;
  ScanResult scanResult = const ScanResult(tracks: [], skipped: []);
  Error? startError;

  void emit(EngineEvent event) => _events.add(event);

  @override
  Stream<EngineEvent> get events => _events.stream;

  @override
  Future<void> start(MixSettings settings) async {
    startedWith = settings;
    if (startError != null) throw startError!;
  }

  @override
  Future<ScanResult> loadFolder(String path, FolderLoadMode mode) async {
    calls.add('loadFolder $path ${mode.name}');
    return scanResult;
  }

  @override
  Future<void> setAutoMix({required bool enabled}) async =>
      calls.add('setAutoMix $enabled');

  @override
  Future<void> setCrossfadeDuration(double seconds) async =>
      calls.add('setCrossfadeDuration $seconds');

  @override
  Future<void> setCrossfadeCurve(CrossfadeCurveType curve) async =>
      calls.add('setCrossfadeCurve ${curve.name}');

  @override
  Future<void> setSilenceThreshold(double db) async =>
      calls.add('setSilenceThreshold $db');

  @override
  Future<void> setShuffle({required bool enabled}) async =>
      calls.add('setShuffle $enabled');

  @override
  Future<void> setRepeat({required bool enabled}) async =>
      calls.add('setRepeat $enabled');

  @override
  Future<void> resumeAutoMix() async => calls.add('resumeAutoMix');

  @override
  Future<void> mixNow() async => calls.add('mixNow');

  @override
  Future<void> togglePlay(DeckSide deck) async =>
      calls.add('togglePlay ${deck.name}');

  @override
  Future<void> seek(DeckSide deck, double seconds) async =>
      calls.add('seek ${deck.name} $seconds');

  @override
  Future<void> setCrossfader(double position) async =>
      calls.add('setCrossfader ${position.toStringAsFixed(2)}');

  @override
  Future<void> setKeylock(DeckSide deck, {required bool enabled}) async =>
      calls.add('setKeylock ${deck.name} $enabled');

  @override
  Future<void> setPitch(DeckSide deck, double percent) async =>
      calls.add('setPitch ${deck.name} ${percent.toStringAsFixed(1)}');

  @override
  Future<void> setDeckGain(DeckSide deck, double linear) async =>
      calls.add('setDeckGain ${deck.name}');

  @override
  Future<void> setEq(DeckSide deck, EqBandType band, double linear) async =>
      calls.add('setEq ${deck.name} ${band.name} $linear');

  @override
  Future<void> setMasterGain(double linear) async =>
      calls.add('setMasterGain ${linear.toStringAsFixed(2)}');

  @override
  Future<void> moveInQueue(int from, int to) async =>
      calls.add('moveInQueue $from $to');

  @override
  Future<void> loadTrack(DeckSide deck, int trackId) async =>
      calls.add('loadTrack ${deck.name} $trackId');
}

/// Settings kept in memory.
class InMemorySettingsRepository implements SettingsRepository {
  new([this.stored = const MixSettings()]);

  MixSettings stored;
  int saves = 0;

  @override
  Future<MixSettings> load() async => stored;

  @override
  Future<void> save(MixSettings settings) async {
    stored = settings;
    saves++;
  }
}

/// Overrides shared by widget tests.
List<Override> testOverrides(
  FakeEngineGateway gateway, {
  InMemorySettingsRepository? settings,
  String? pickedFolder,
}) {
  return [
    engineGatewayProvider.overrideWithValue(gateway),
    settingsRepositoryProvider.overrideWithValue(
      settings ?? InMemorySettingsRepository(),
    ),
    folderPickerProvider.overrideWithValue(() async => pickedFolder),
    // desktop_drop needs its native library, absent from unit tests.
    dropZoneProvider.overrideWithValue(_passThroughDropZone),
  ];
}

Widget _passThroughDropZone({
  required Widget child,
  required Future<void> Function(String? folder) onFolder,
  required VoidCallback onEnter,
  required VoidCallback onLeave,
}) {
  return child;
}

/// Pumps the whole app on a 1440×900 window.
Future<void> pumpPlatine(
  WidgetTester tester,
  FakeEngineGateway gateway, {
  Size size = const Size(1440, 900),
  String? pickedFolder,
}) async {
  tester.view
    ..physicalSize = size
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: testOverrides(gateway, pickedFolder: pickedFolder),
      child: const PlatineApp(),
    ),
  );
  await tester.pump();
  await tester.pump();
}

const tracks = [
  TrackInfo(id: 1, path: '/m/Set/01 Intro.mp3', title: 'Intro', artist: 'DJ'),
  TrackInfo(id: 2, path: '/m/Set/02 Groove.mp3', title: 'Groove'),
  TrackInfo(id: 3, path: '/m/Set/03 Finale.mp3', title: 'Finale'),
];

/// Brings the fake session to "deck A playing track 1, deck B ready".
void emitPlayingSession(FakeEngineGateway gateway) {
  gateway
    ..emit(const LibraryChanged(tracks))
    ..emit(const DeckLoading(deck: DeckSide.a, trackId: 1))
    ..emit(const DeckLoading(deck: DeckSide.b, trackId: 2))
    ..emit(
      const DeckLoaded(
        deck: DeckSide.a,
        trackId: 1,
        duration: 200,
        start: 1,
        end: 195,
      ),
    )
    ..emit(
      const DeckLoaded(
        deck: DeckSide.b,
        trackId: 2,
        duration: 180,
        start: 0,
        end: 178,
      ),
    )
    ..emit(
      const AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.a,
      ),
    )
    ..emit(const QueueChanged([3]));
}
