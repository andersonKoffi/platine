import 'dart:typed_data' show Float32List, Uint8List;

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show Uint64List;
import 'package:flutter_test/flutter_test.dart';
import 'package:platine/core/engine/data/frb_engine_gateway.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/src/rust/api/mix.dart' as rust;
import 'package:platine/src/rust/lib.dart';

void main() {
  test('library and deck updates map to domain events', () {
    final library = mapUpdate(
      rust.EngineUpdate.libraryChanged(
        tracks: [rust.TrackDto(id: BigInt.one, path: '/a.mp3', title: 'A')],
      ),
    );
    expect(library, isA<LibraryChanged>());
    expect((library as LibraryChanged).tracks.single.id, 1);

    final loaded = mapUpdate(
      rust.EngineUpdate.deckLoaded(
        deck: rust.Deck.b,
        trackId: BigInt.two,
        durationSeconds: 10,
        startSeconds: 1,
        endSeconds: 9,
      ),
    );
    expect(loaded, isA<DeckLoaded>());
    loaded as DeckLoaded;
    expect(loaded.deck, DeckSide.b);
    expect(loaded.end, 9);

    expect(
      mapUpdate(
        rust.EngineUpdate.deckLoading(deck: rust.Deck.a, trackId: BigInt.one),
      ),
      isA<DeckLoading>(),
    );
    expect(
      mapUpdate(
        const rust.EngineUpdate.deckPosition(
          deck: rust.Deck.a,
          positionSeconds: 3,
          playing: true,
        ),
      ),
      isA<DeckPositionChanged>(),
    );
  });

  test('waveforms map to domain events', () {
    final event = mapUpdate(
      rust.EngineUpdate.waveform(
        deck: rust.Deck.a,
        trackId: BigInt.from(7),
        pointsPerSecond: 100,
        low: Uint8List.fromList([1, 2]),
        mid: Uint8List.fromList([3, 4]),
        high: Uint8List.fromList([5, 6]),
      ),
    );
    expect(event, isA<WaveformReady>());
    event as WaveformReady;
    expect(event.trackId, 7);
    expect(event.waveform.length, 2);
    expect(event.waveform.duration, 0.02);
    expect(event.waveform.high, [5, 6]);
  });

  test('levels keep the louder channel', () {
    final levels = mapUpdate(
      rust.EngineUpdate.levels(
        rust.LevelsDto(
          deckA: F32Array2(Float32List.fromList([0.2, 0.4])),
          deckB: F32Array2(Float32List.fromList([0.1, 0])),
          master: F32Array2(Float32List.fromList([0.5, 0.3])),
        ),
      ),
    );
    levels as LevelsChanged;
    expect(levels.levels.deckA, closeTo(0.4, 1e-6));
    expect(levels.levels.master, closeTo(0.5, 1e-6));
  });

  test('auto-mix states and statuses map one to one', () {
    final cases = <rust.AutoMixStateDto, AutoMixPhase>{
      const rust.AutoMixStateDto.idle(): AutoMixPhase.idle,
      const rust.AutoMixStateDto.playing(active: rust.Deck.b):
          AutoMixPhase.playing,
      const rust.AutoMixStateDto.crossfading(
        from: rust.Deck.a,
        to: rust.Deck.b,
      ): AutoMixPhase.crossfading,
      const rust.AutoMixStateDto.waitingForNext(deck: rust.Deck.a):
          AutoMixPhase.waitingForNext,
      const rust.AutoMixStateDto.finished(): AutoMixPhase.finished,
    };
    for (final entry in cases.entries) {
      final event = mapUpdate(
        rust.EngineUpdate.autoMixChanged(
          state: entry.key,
          status: rust.AutoMixStatusDto.suspended,
        ),
      );
      event as AutoMixChanged;
      expect(event.phase, entry.value);
      expect(event.mode, AutoMixMode.suspended);
    }
    final off = mapUpdate(
      const rust.EngineUpdate.autoMixChanged(
        state: rust.AutoMixStateDto.idle(),
        status: rust.AutoMixStatusDto.off,
      ),
    );
    expect((off as AutoMixChanged).mode, AutoMixMode.off);
  });

  test('queue, transitions and notices map to domain events', () {
    final queue = mapUpdate(
      rust.EngineUpdate.queueChanged(upcoming: Uint64List.fromList([3, 4])),
    );
    expect((queue as QueueChanged).upcoming, [3, 4]);
    expect(
      mapUpdate(
        const rust.EngineUpdate.transitionStarted(
          from: rust.Deck.a,
          to: rust.Deck.b,
        ),
      ),
      isA<TransitionStarted>(),
    );
    expect(
      mapUpdate(
        const rust.EngineUpdate.transitionCompleted(
          from: rust.Deck.b,
          to: rust.Deck.a,
        ),
      ),
      isA<TransitionCompleted>(),
    );
    expect(
      mapUpdate(
        rust.EngineUpdate.trackSkipped(trackId: BigInt.two, reason: 'x'),
      ),
      isA<TrackSkipped>(),
    );
    expect(
      mapUpdate(const rust.EngineUpdate.queueExhausted()),
      isA<QueueExhausted>(),
    );
    expect(
      mapUpdate(const rust.EngineUpdate.underrun(deck: rust.Deck.b)),
      isA<Underrun>(),
    );
  });
}
