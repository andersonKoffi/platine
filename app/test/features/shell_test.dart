import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/presentation/deck_view.dart';
import 'package:platine/features/library/presentation/library_panel.dart';
import 'package:platine/features/mixer/presentation/mixer_panel.dart';

import '../helpers/fakes.dart';

void main() {
  late FakeEngineGateway gateway;

  setUp(() => gateway = FakeEngineGateway());

  testWidgets('starts the engine and shows the drop zone', (tester) async {
    await pumpPlatine(tester, gateway);
    expect(gateway.startedWith, const MixSettings());
    expect(find.text('Dépose un dossier de musique ici'), findsOneWidget);
    expect(find.text('Platine'), findsOneWidget);
  });

  testWidgets('choosing a folder loads it', (tester) async {
    gateway.scanResult = const ScanResult(
      tracks: tracks,
      skipped: [SkippedEntry(path: '/m/x.mp3', reason: 'denied')],
    );
    await pumpPlatine(tester, gateway, pickedFolder: '/m/Set');
    await tester.tap(find.textContaining('Choisir un dossier'));
    await tester.pump();
    expect(gateway.calls, contains('loadFolder /m/Set replace'));
    await tester.pump();
    expect(find.textContaining('1 éléments illisibles'), findsOneWidget);
    await tester.pump(const Duration(seconds: 5));
  });

  testWidgets('shows decks, mixer and library once a folder is loaded', (
    tester,
  ) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    await tester.pump();

    expect(find.byType(DeckView), findsNWidgets(2));
    expect(find.byType(MixerPanel), findsOneWidget);
    expect(find.byType(LibraryPanel), findsOneWidget);
    // Track 1 is on deck A, track 2 on deck B, track 3 waits in the queue.
    expect(find.text('Intro'), findsWidgets);
    expect(find.text('Groove'), findsWidgets);
    expect(find.text('Finale'), findsWidgets);
    expect(find.text('En marche'), findsOneWidget);
  });

  testWidgets('library search filters the list', (tester) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    await tester.pump();
    await tester.enterText(find.byType(TextField), 'fin');
    await tester.pump();
    final list = find.descendant(
      of: find.byType(LibraryPanel),
      matching: find.text('Intro'),
    );
    expect(list, findsNothing);
  });

  testWidgets('transport, crossfader and presets reach the engine', (
    tester,
  ) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    await tester.pump();

    await tester.tap(find.byTooltip('Lecture platine A'));
    await tester.tap(find.byTooltip('Crossfader tout sur B'));
    await tester.tap(find.byTooltip('Fondu de 4 secondes'));
    await tester.tap(find.byTooltip('Charger sur la platine B').first);
    // The row also listens for double taps: the single tap wins after the
    // double-tap timeout.
    await tester.pump(kDoubleTapTimeout + const Duration(milliseconds: 50));

    expect(gateway.calls, contains('togglePlay a'));
    expect(gateway.calls, contains('setCrossfader 1.00'));
    expect(gateway.calls, contains('setCrossfadeDuration 4.0'));
    expect(gateway.calls, contains('loadTrack b 1'));
  });

  testWidgets('suspended auto-mix offers to resume', (tester) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    gateway.emit(
      const AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.suspended,
        active: DeckSide.a,
      ),
    );
    await tester.pump();
    expect(find.text('Suspendu'), findsOneWidget);
    await tester.tap(find.text('Reprendre'));
    await tester.pump();
    expect(gateway.calls, contains('resumeAutoMix'));
  });

  testWidgets('keyboard shortcuts drive the decks and the help screen', (
    tester,
  ) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.keyQ);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyW);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit5);
    await tester.pump();
    expect(gateway.calls, containsAll(['togglePlay a', 'togglePlay b']));
    expect(gateway.calls, contains('setCrossfader 0.10'));
    expect(gateway.calls, contains('setCrossfadeDuration 16.0'));

    await tester.tap(find.byTooltip('Raccourcis clavier (?)'));
    await tester.pump();
    expect(find.text('Raccourcis clavier'), findsOneWidget);
    expect(find.text('Afficher cette aide'), findsOneWidget);
  });

  testWidgets('skipped tracks show a notice', (tester) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    gateway.emit(const TrackSkipped(trackId: 3, reason: 'corrupt'));
    await tester.pump();
    expect(find.textContaining('Finale'), findsWidgets);
    expect(find.textContaining('illisible'), findsOneWidget);
    await tester.pump(const Duration(seconds: 5));
  });

  testWidgets('narrow windows put the library under the decks', (tester) async {
    await pumpPlatine(tester, gateway, size: const Size(1000, 900));
    emitPlayingSession(gateway);
    await tester.pump();
    final deck = tester.getTopLeft(find.byType(DeckView).first);
    final library = tester.getTopLeft(find.byType(LibraryPanel));
    expect(library.dy, greaterThan(deck.dy));
  });

  testWidgets('a track dragged from the library loads on a stopped deck', (
    tester,
  ) async {
    await pumpPlatine(tester, gateway);
    emitPlayingSession(gateway);
    gateway.emit(
      const DeckPositionChanged(deck: DeckSide.a, position: 10, playing: true),
    );
    await tester.pump();

    Future<void> dragFinaleTo(DeckSide side) async {
      final row = find.descendant(
        of: find.byType(LibraryPanel),
        matching: find.text('Finale'),
      );
      final deck = find.byWidgetPredicate(
        (w) => w is DeckView && w.side == side,
      );
      final gesture = await tester.startGesture(tester.getCenter(row));
      await tester.pump(kLongPressTimeout);
      await gesture.moveBy(const Offset(0, -20));
      await tester.pump();
      await gesture.moveTo(tester.getCenter(deck));
      await tester.pump();
      await gesture.up();
      await tester.pump();
    }

    // Deck A is on air: the drop is refused so the music never stops.
    await dragFinaleTo(DeckSide.a);
    expect(gateway.calls.where((c) => c.startsWith('loadTrack')), isEmpty);

    await dragFinaleTo(DeckSide.b);
    expect(gateway.calls, contains('loadTrack b 3'));
  });

  testWidgets('mix now is offered when the next track is ready', (
    tester,
  ) async {
    await pumpPlatine(tester, gateway);
    await tester.pump();
    final button = find.byTooltip('Enchaîner maintenant (N)');
    expect(button, findsOneWidget);
    await tester.tap(button);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    expect(gateway.calls.where((c) => c == 'mixNow'), isEmpty);

    emitPlayingSession(gateway);
    await tester.pump();
    await tester.tap(button);
    await tester.pump();
    expect(gateway.calls.where((c) => c == 'mixNow'), hasLength(1));
    await tester.sendKeyEvent(LogicalKeyboardKey.keyN);
    expect(gateway.calls.where((c) => c == 'mixNow'), hasLength(2));
  });
}
