import 'package:flutter/foundation.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';

/// What the UI knows about a deck.
@immutable
class DeckState {
  const new({
    this.trackId,
    this.loading = false,
    this.duration = 0,
    this.start = 0,
    this.end = 0,
    this.position = 0,
    this.playing = false,
    this.level = 0,
    this.positionUpdatedAt,
    this.waveform,
  });

  static const DeckState empty = DeckState();

  final int? trackId;
  final bool loading;
  final double duration;
  final double start;
  final double end;
  final double position;
  final bool playing;

  /// Peak level, linear.
  final double level;

  /// When [position] was reported (to extrapolate between reports).
  final DateTime? positionUpdatedAt;

  /// Coloured overview of the track, once analysed.
  final WaveformData? waveform;

  bool get hasTrack => trackId != null && !loading;

  /// Seconds left before the end of the audible part.
  double get remaining => (end - position).clamp(0, double.infinity);

  DeckState copyWith({
    int? trackId,
    bool? loading,
    double? duration,
    double? start,
    double? end,
    double? position,
    bool? playing,
    double? level,
    DateTime? positionUpdatedAt,
    WaveformData? waveform,
  }) {
    return DeckState(
      trackId: trackId ?? this.trackId,
      loading: loading ?? this.loading,
      duration: duration ?? this.duration,
      start: start ?? this.start,
      end: end ?? this.end,
      position: position ?? this.position,
      playing: playing ?? this.playing,
      level: level ?? this.level,
      positionUpdatedAt: positionUpdatedAt ?? this.positionUpdatedAt,
      waveform: waveform ?? this.waveform,
    );
  }
}

/// A running automatic crossfade, for the crossfader animation.
@immutable
class TransitionView {
  const new({required this.from, required this.to, required this.startedAt});

  final DeckSide from;
  final DeckSide to;
  final DateTime startedAt;
}

/// A short message for the user (toast).
@immutable
class Notice {
  const new({required this.id, required this.message, this.details});

  /// Increases with every notice, so that identical messages still show.
  final int id;
  final String message;
  final String? details;
}

/// Everything the UI shows about the session.
@immutable
class SessionState {
  const new({
    this.library = const [],
    this.deckA = DeckState.empty,
    this.deckB = DeckState.empty,
    this.phase = AutoMixPhase.finished,
    this.mode = AutoMixMode.running,
    this.activeDeck,
    this.upcoming = const [],
    this.levels = const LevelSnapshot(),
    this.crossfader = 0.5,
    this.transition,
    this.notice,
    this.queueExhausted = false,
  });

  final List<TrackInfo> library;
  final DeckState deckA;
  final DeckState deckB;
  final AutoMixPhase phase;
  final AutoMixMode mode;

  /// Deck heard now (or awaited), if any.
  final DeckSide? activeDeck;
  final List<int> upcoming;
  final LevelSnapshot levels;

  /// Last known crossfader position (0 = A, 1 = B).
  final double crossfader;
  final TransitionView? transition;
  final Notice? notice;
  final bool queueExhausted;

  DeckState deck(DeckSide side) => side == DeckSide.a ? deckA : deckB;

  /// "Mix now" is possible: a deck on air and a ready track on the other one.
  bool get canMixNow {
    final active = activeDeck;
    if (active == null || phase != AutoMixPhase.playing) return false;
    if (mode == AutoMixMode.off) return false;
    final standby = deck(active.other);
    return standby.hasTrack && !standby.loading;
  }

  TrackInfo? track(int? id) {
    if (id == null) return null;
    for (final track in library) {
      if (track.id == id) return track;
    }
    return null;
  }

  /// Track that will play after the current one: the standby deck's track,
  /// or the head of the queue.
  int? get nextTrackId {
    final active = activeDeck;
    if (active != null) {
      final standby = deck(active.other).trackId;
      if (standby != null) return standby;
    }
    return upcoming.isEmpty ? null : upcoming.first;
  }

  SessionState withDeck(DeckSide side, DeckState deck) =>
      side == DeckSide.a ? copyWith(deckA: deck) : copyWith(deckB: deck);

  SessionState copyWith({
    List<TrackInfo>? library,
    DeckState? deckA,
    DeckState? deckB,
    AutoMixPhase? phase,
    AutoMixMode? mode,
    DeckSide? activeDeck,
    bool clearActiveDeck = false,
    List<int>? upcoming,
    LevelSnapshot? levels,
    double? crossfader,
    TransitionView? transition,
    bool clearTransition = false,
    Notice? notice,
    bool? queueExhausted,
  }) {
    return SessionState(
      library: library ?? this.library,
      deckA: deckA ?? this.deckA,
      deckB: deckB ?? this.deckB,
      phase: phase ?? this.phase,
      mode: mode ?? this.mode,
      activeDeck: clearActiveDeck ? null : activeDeck ?? this.activeDeck,
      upcoming: upcoming ?? this.upcoming,
      levels: levels ?? this.levels,
      crossfader: crossfader ?? this.crossfader,
      transition: clearTransition ? null : transition ?? this.transition,
      notice: notice ?? this.notice,
      queueExhausted: queueExhausted ?? this.queueExhausted,
    );
  }
}

/// Folds one engine event into the session state. Pure: [now] is injected.
SessionState reduceSession(
  SessionState state,
  EngineEvent event,
  DateTime now,
) {
  return switch (event) {
    LibraryChanged(:final tracks) => state.copyWith(
      library: _keepTempos(tracks, state.library),
      queueExhausted: false,
    ),
    DeckLoading(:final deck, :final trackId) => state.withDeck(
      deck,
      DeckState(trackId: trackId, loading: true),
    ),
    DeckLoaded(
      :final deck,
      :final trackId,
      :final duration,
      :final start,
      :final end,
    ) =>
      state.withDeck(
        deck,
        DeckState(
          trackId: trackId,
          duration: duration,
          start: start,
          end: end,
          position: start,
          positionUpdatedAt: now,
          // The waveform comes from the analysis, just before loading.
          waveform: state.deck(deck).trackId == trackId
              ? state.deck(deck).waveform
              : null,
        ),
      ),
    WaveformReady(:final deck, :final trackId, :final waveform) =>
      state.deck(deck).trackId == trackId
          ? state.withDeck(deck, state.deck(deck).copyWith(waveform: waveform))
          : state,
    TrackAnalyzed(:final trackId, :final bpm, :final firstBeat) =>
      state.copyWith(
        library: [
          for (final track in state.library)
            if (track.id == trackId)
              track.withTempo(bpm: bpm, firstBeat: firstBeat)
            else
              track,
        ],
      ),
    DeckPositionChanged(:final deck, :final position, :final playing) =>
      state.withDeck(
        deck,
        state
            .deck(deck)
            .copyWith(
              position: position,
              playing: playing,
              positionUpdatedAt: now,
            ),
      ),
    LevelsChanged(:final levels) =>
      state
          .copyWith(levels: levels)
          .withDeck(DeckSide.a, state.deckA.copyWith(level: levels.deckA))
          .withDeck(DeckSide.b, state.deckB.copyWith(level: levels.deckB)),
    final AutoMixChanged changed => _autoMixChanged(state, changed),
    QueueChanged(:final upcoming) => state.copyWith(upcoming: upcoming),
    TransitionStarted(:final from, :final to) => state.copyWith(
      transition: TransitionView(from: from, to: to, startedAt: now),
    ),
    TransitionCompleted(:final to) => state.copyWith(
      crossfader: to.crossfaderSide,
      clearTransition: true,
    ),
    TrackSkipped(:final trackId, :final reason) => state.copyWith(
      notice: Notice(
        id: (state.notice?.id ?? 0) + 1,
        message:
            '« ${state.track(trackId)?.title ?? 'Titre $trackId'} » '
            'illisible, titre suivant.',
        details: reason,
      ),
    ),
    QueueExhausted() => state.copyWith(queueExhausted: true),
    Underrun() => state,
  };
}

SessionState _autoMixChanged(SessionState state, AutoMixChanged event) {
  final next = state.copyWith(
    phase: event.phase,
    mode: event.mode,
    activeDeck: event.active ?? event.to,
    clearActiveDeck: event.active == null && event.to == null,
  );
  // While the auto-mix runs, the crossfader sits on the playing deck.
  final active = event.active;
  if (event.phase == AutoMixPhase.playing &&
      event.mode == AutoMixMode.running &&
      active != null) {
    return next.copyWith(crossfader: active.crossfaderSide);
  }
  if (event.mode != AutoMixMode.running) {
    return next.copyWith(clearTransition: true);
  }
  return next;
}

/// Keeps the tempos already analysed when the library is resent (append mode).
List<TrackInfo> _keepTempos(List<TrackInfo> tracks, List<TrackInfo> previous) {
  if (previous.isEmpty) return tracks;
  final known = {for (final track in previous) track.id: track};
  return [
    for (final track in tracks)
      switch (known[track.id]) {
        final TrackInfo old when old.bpm != null && track.bpm == null =>
          track.withTempo(bpm: old.bpm, firstBeat: old.firstBeat),
        final TrackInfo old when old.firstBeat != null => track.withTempo(
          bpm: track.bpm,
          firstBeat: old.firstBeat,
        ),
        _ => track,
      },
  ];
}
