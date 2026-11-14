import 'package:platine/core/engine/domain/models.dart';

/// Updates published by the audio engine (Observer pattern).
sealed class EngineEvent {
  const new();
}

final class LibraryChanged extends EngineEvent {
  const new(this.tracks);

  final List<TrackInfo> tracks;
}

final class DeckLoading extends EngineEvent {
  const new({required this.deck, required this.trackId});

  final DeckSide deck;
  final int trackId;
}

final class DeckLoaded extends EngineEvent {
  const new({
    required this.deck,
    required this.trackId,
    required this.duration,
    required this.start,
    required this.end,
  });

  final DeckSide deck;
  final int trackId;

  /// Seconds.
  final double duration;

  /// First audible second.
  final double start;

  /// Last audible second.
  final double end;
}

final class DeckPositionChanged extends EngineEvent {
  const new({
    required this.deck,
    required this.position,
    required this.playing,
  });

  final DeckSide deck;
  final double position;
  final bool playing;
}

final class LevelsChanged extends EngineEvent {
  const new(this.levels);

  final LevelSnapshot levels;
}

final class AutoMixChanged extends EngineEvent {
  const new({
    required this.phase,
    required this.mode,
    this.active,
    this.from,
    this.to,
  });

  final AutoMixPhase phase;
  final AutoMixMode mode;

  /// Playing deck (playing), or awaited deck (waitingForNext).
  final DeckSide? active;
  final DeckSide? from;
  final DeckSide? to;
}

final class QueueChanged extends EngineEvent {
  const new(this.upcoming);

  final List<int> upcoming;
}

final class TransitionStarted extends EngineEvent {
  const new({required this.from, required this.to});

  final DeckSide from;
  final DeckSide to;
}

final class TransitionCompleted extends EngineEvent {
  const new({required this.from, required this.to});

  final DeckSide from;
  final DeckSide to;
}

final class TrackSkipped extends EngineEvent {
  const new({required this.trackId, required this.reason});

  final int trackId;
  final String reason;
}

final class QueueExhausted extends EngineEvent {
  const new();
}

final class Underrun extends EngineEvent {
  const new(this.deck);

  final DeckSide deck;
}
