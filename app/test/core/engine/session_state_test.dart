import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';

void main() {
  final now = DateTime(2026, 11, 14, 12);
  SessionState apply(List<EngineEvent> events) => events.fold(
    const SessionState(),
    (state, event) => reduceSession(state, event, now),
  );

  const library = [
    TrackInfo(id: 1, path: '/a.mp3', title: 'A'),
    TrackInfo(id: 2, path: '/b.mp3', title: 'B'),
    TrackInfo(id: 3, path: '/c.mp3', title: 'C'),
  ];

  test('loading then loaded fills the deck', () {
    final state = apply(const [
      LibraryChanged(library),
      DeckLoading(deck: DeckSide.b, trackId: 2),
    ]);
    expect(state.deckB.loading, isTrue);
    expect(state.deckB.hasTrack, isFalse);

    final loaded = reduceSession(
      state,
      const DeckLoaded(
        deck: DeckSide.b,
        trackId: 2,
        duration: 100,
        start: 2,
        end: 95,
      ),
      now,
    );
    expect(loaded.deckB.hasTrack, isTrue);
    expect(loaded.deckB.position, 2);
    expect(loaded.deckB.remaining, 93);
    expect(loaded.track(2)?.title, 'B');
    expect(loaded.track(9), isNull);
    expect(loaded.track(null), isNull);
  });

  test('the waveform of the loading track survives the load', () {
    final waveform = WaveformData(
      pointsPerSecond: 100,
      low: Uint8List(3),
      mid: Uint8List(3),
      high: Uint8List(3),
    );
    final state = apply([
      const DeckLoading(deck: DeckSide.a, trackId: 1),
      WaveformReady(deck: DeckSide.a, trackId: 1, waveform: waveform),
      // A stale waveform for another track is ignored.
      WaveformReady(deck: DeckSide.a, trackId: 9, waveform: waveform),
      const DeckLoaded(
        deck: DeckSide.a,
        trackId: 1,
        duration: 3,
        start: 0,
        end: 3,
      ),
    ]);
    expect(state.deckA.waveform, same(waveform));
    final reloaded = reduceSession(
      state,
      const DeckLoaded(
        deck: DeckSide.a,
        trackId: 2,
        duration: 1,
        start: 0,
        end: 1,
      ),
      now,
    );
    expect(reloaded.deckA.waveform, isNull);
  });

  test('positions and levels update the decks', () {
    final state = apply(const [
      DeckPositionChanged(deck: DeckSide.a, position: 12.5, playing: true),
      LevelsChanged(LevelSnapshot(deckA: 0.5, deckB: 0.1, master: 0.6)),
    ]);
    expect(state.deckA.position, 12.5);
    expect(state.deckA.playing, isTrue);
    expect(state.deckA.positionUpdatedAt, now);
    expect(state.deckA.level, 0.5);
    expect(state.deckB.level, 0.1);
    expect(state.levels.master, 0.6);
    expect(state.levels.of(DeckSide.b), 0.1);
  });

  test('running auto-mix puts the crossfader on the playing deck', () {
    final state = apply(const [
      AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.b,
      ),
    ]);
    expect(state.crossfader, 1);
    expect(state.activeDeck, DeckSide.b);
    expect(state.mode, AutoMixMode.running);
  });

  test('transitions animate then settle on the incoming deck', () {
    final started = apply(const [
      TransitionStarted(from: DeckSide.a, to: DeckSide.b),
    ]);
    expect(started.transition?.from, DeckSide.a);
    expect(started.transition?.startedAt, now);
    final done = reduceSession(
      started,
      const TransitionCompleted(from: DeckSide.a, to: DeckSide.b),
      now,
    );
    expect(done.transition, isNull);
    expect(done.crossfader, 1);
  });

  test('suspension stops the crossfader animation', () {
    final state = apply(const [
      TransitionStarted(from: DeckSide.a, to: DeckSide.b),
      AutoMixChanged(
        phase: AutoMixPhase.crossfading,
        mode: AutoMixMode.suspended,
        from: DeckSide.a,
        to: DeckSide.b,
      ),
    ]);
    expect(state.transition, isNull);
    expect(state.mode, AutoMixMode.suspended);
    expect(state.activeDeck, DeckSide.b);
  });

  test('finished auto-mix has no active deck', () {
    final state = apply(const [
      AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.a,
      ),
      AutoMixChanged(phase: AutoMixPhase.finished, mode: AutoMixMode.running),
    ]);
    expect(state.activeDeck, isNull);
  });

  test('next track is the standby deck, else the head of the queue', () {
    var state = apply(const [
      LibraryChanged(library),
      QueueChanged([3]),
      AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.a,
      ),
    ]);
    expect(state.nextTrackId, 3);
    state = reduceSession(
      state,
      const DeckLoading(deck: DeckSide.b, trackId: 2),
      now,
    );
    expect(state.nextTrackId, 2);
    expect(const SessionState().nextTrackId, isNull);
  });

  test('skipped tracks and the end of the queue produce notices', () {
    final state = apply(const [
      LibraryChanged(library),
      TrackSkipped(trackId: 3, reason: 'corrupt'),
      TrackSkipped(trackId: 42, reason: 'missing'),
      QueueExhausted(),
      Underrun(DeckSide.a),
    ]);
    expect(state.notice?.id, 2);
    expect(state.notice?.message, contains('Titre 42'));
    expect(state.notice?.details, 'missing');
    expect(state.queueExhausted, isTrue);
    final again = reduceSession(state, const LibraryChanged(library), now);
    expect(again.queueExhausted, isFalse);
  });

  test('deck sides know their crossfader position and label', () {
    expect(DeckSide.a.other, DeckSide.b);
    expect(DeckSide.b.crossfaderSide, 1);
    expect(DeckSide.a.label, 'A');
  });

  test('analysed tempos reach the library and survive a library refresh', () {
    final state = apply(const [
      LibraryChanged(library),
      TrackAnalyzed(trackId: 2, bpm: 124, firstBeat: 0.3),
      TrackAnalyzed(trackId: 3),
    ]);
    expect(state.track(2)?.bpm, 124);
    expect(state.track(2)?.firstBeat, 0.3);
    expect(state.track(3)?.bpm, isNull);
    expect(state.track(1)?.bpm, isNull);

    // Append mode resends the whole library without first beats.
    final refreshed = reduceSession(
      state,
      const LibraryChanged([
        ...library,
        TrackInfo(id: 4, path: '/d.mp3', title: 'D'),
      ]),
      now,
    );
    expect(refreshed.track(2)?.bpm, 124);
    expect(refreshed.track(2)?.firstBeat, 0.3);
    expect(refreshed.track(4)?.bpm, isNull);
  });

  test('mix now needs a deck on air and a ready standby deck', () {
    final playing = apply(const [
      LibraryChanged(library),
      DeckLoaded(deck: DeckSide.a, trackId: 1, duration: 100, start: 0, end: 99),
      AutoMixChanged(
        phase: AutoMixPhase.playing,
        mode: AutoMixMode.running,
        active: DeckSide.a,
      ),
    ]);
    expect(playing.canMixNow, isFalse);
    final ready = reduceSession(
      playing,
      const DeckLoaded(
        deck: DeckSide.b,
        trackId: 2,
        duration: 100,
        start: 0,
        end: 99,
      ),
      now,
    );
    expect(ready.canMixNow, isTrue);
    final fading = reduceSession(
      ready,
      const AutoMixChanged(
        phase: AutoMixPhase.crossfading,
        mode: AutoMixMode.running,
        active: DeckSide.b,
      ),
      now,
    );
    expect(fading.canMixNow, isFalse);
  });
}
