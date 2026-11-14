@Tags(['screenshot'])
library;

import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:platine/app.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';

import '../helpers/fakes.dart';

/// Full-screen renders used to review the visual design. They are written
/// with `flutter test --tags screenshot --update-goldens` and are not
/// compared in the regular test run.
void main() {
  setUpAll(_loadFonts);

  for (final size in const [Size(1440, 900), Size(1180, 760)]) {
    final name = '${size.width.round()}x${size.height.round()}';

    testWidgets('empty $name', (tester) async {
      await pumpPlatine(tester, FakeEngineGateway(), size: size);
      await tester.pump(const Duration(milliseconds: 700));
      await expectLater(
        find.byType(PlatineApp),
        matchesGoldenFile('goldens/empty_$name.png'),
      );
    });

    testWidgets('mixing $name', (tester) async {
      final gateway = FakeEngineGateway();
      await pumpPlatine(tester, gateway, size: size);
      _emitMix(gateway);
      await tester.pump();
      await tester.pump(const Duration(seconds: 2));
      await expectLater(
        find.byType(PlatineApp),
        matchesGoldenFile('goldens/mixing_$name.png'),
      );
    });
  }
}

Future<void> _loadFonts() async {
  final grotesk = FontLoader('SpaceGrotesk')
    ..addFont(rootBundle.load('assets/fonts/SpaceGrotesk-Medium.ttf'))
    ..addFont(rootBundle.load('assets/fonts/SpaceGrotesk-Bold.ttf'));
  await grotesk.load();
  final root = Platform.environment['FLUTTER_ROOT'];
  final icons = File(
    '$root/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
  );
  if (root != null && icons.existsSync()) {
    final loader = FontLoader('MaterialIcons')
      ..addFont(Future.value(ByteData.sublistView(icons.readAsBytesSync())));
    await loader.load();
  }
}

/// Synthetic music-like waveform: kick on every beat, hats in between.
WaveformData _waveform(int seed, double seconds) {
  final random = math.Random(seed);
  const pps = 100;
  final length = (seconds * pps).round();
  final low = Uint8List(length);
  final mid = Uint8List(length);
  final high = Uint8List(length);
  for (var i = 0; i < length; i++) {
    final t = i / pps;
    final beat = (t * 2.1) % 1;
    final section = 0.55 + 0.45 * math.sin(t / 9 + seed);
    final kick = math.exp(-beat * 9);
    final hat = math.exp(-((beat + 0.5) % 1) * 18);
    low[i] = (255 * (0.15 + 0.8 * kick) * section).clamp(0, 255).round();
    mid[i] = (255 * (0.25 + 0.35 * random.nextDouble()) * section)
        .clamp(0, 255)
        .round();
    high[i] = (255 * (0.1 + 0.6 * hat) * section).clamp(0, 255).round();
  }
  return WaveformData(pointsPerSecond: pps, low: low, mid: mid, high: high);
}

void _emitMix(FakeEngineGateway gateway) {
  const library = [
    TrackInfo(
      id: 1,
      path: '/Music/Brunch/MARIZU - Jejely.mp3',
      title: 'Jejely',
      artist: 'MARIZU',
    ),
    TrackInfo(
      id: 2,
      path: '/Music/Brunch/MARIZU - God My King.mp3',
      title: 'God My King',
      artist: 'MARIZU',
    ),
    TrackInfo(id: 3, path: '/Music/Brunch/Afro Sunrise.flac', title: 'Afro Sunrise'),
    TrackInfo(id: 4, path: '/Music/Brunch/Amapiano Nights.mp3', title: 'Amapiano Nights', artist: 'Kabza'),
    TrackInfo(id: 5, path: '/Music/Brunch/Coupé Décalé Mix.wav', title: 'Coupé Décalé Mix'),
    TrackInfo(id: 6, path: '/Music/Brunch/Highlife Groove.mp3', title: 'Highlife Groove', artist: 'Osibisa'),
  ];
  gateway
    ..emit(const LibraryChanged(library))
    ..emit(const DeckLoading(deck: DeckSide.a, trackId: 1))
    ..emit(const DeckLoading(deck: DeckSide.b, trackId: 2))
    ..emit(WaveformReady(deck: DeckSide.a, trackId: 1, waveform: _waveform(1, 214)))
    ..emit(WaveformReady(deck: DeckSide.b, trackId: 2, waveform: _waveform(2, 189)))
    ..emit(const DeckLoaded(deck: DeckSide.a, trackId: 1, duration: 214, start: 0.4, end: 211))
    ..emit(const DeckLoaded(deck: DeckSide.b, trackId: 2, duration: 189, start: 1.2, end: 186))
    ..emit(const DeckPositionChanged(deck: DeckSide.a, position: 96.4, playing: true))
    ..emit(const DeckPositionChanged(deck: DeckSide.b, position: 1.2, playing: false))
    ..emit(const LevelsChanged(LevelSnapshot(deckA: 0.7, deckB: 0, master: 0.68)))
    ..emit(
      const AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.a,
      ),
    )
    ..emit(const QueueChanged([3, 4, 5, 6]));
}
