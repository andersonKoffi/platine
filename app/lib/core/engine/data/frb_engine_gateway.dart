import 'dart:async';

import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/engine_gateway.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/src/rust/api/mix.dart' as rust;

/// Adapter: maps the generated bridge API to the [EngineGateway] port.
class FrbEngineGateway implements EngineGateway {
  final StreamController<EngineEvent> _events =
      StreamController<EngineEvent>.broadcast();
  StreamSubscription<rust.EngineUpdate>? _updates;

  @override
  Stream<EngineEvent> get events => _events.stream;

  @override
  Future<void> start(MixSettings settings) async {
    await _updates?.cancel();
    await rust.startSession(
      config: rust.SessionConfig(
        automix: settings.automix,
        crossfadeSeconds: settings.crossfadeSeconds,
        curve: _curve(settings.curve),
        shuffle: settings.shuffle,
        repeat: settings.repeat,
        silenceThresholdDb: settings.silenceThresholdDb,
      ),
    );
    _updates = rust.engineUpdates().listen(
      (update) => _events.add(mapUpdate(update)),
      onError: _events.addError,
    );
  }

  @override
  Future<ScanResult> loadFolder(String path, FolderLoadMode mode) async {
    final result = await rust.loadFolder(
      path: path,
      mode: mode == FolderLoadMode.replace
          ? rust.FolderMode.replace
          : rust.FolderMode.append,
    );
    return ScanResult(
      tracks: result.tracks.map(_track).toList(),
      skipped: [
        for (final entry in result.skipped)
          SkippedEntry(path: entry.path, reason: entry.reason),
      ],
    );
  }

  @override
  Future<void> setAutoMix({required bool enabled}) =>
      rust.setAutomixEnabled(enabled: enabled);

  @override
  Future<void> setCrossfadeDuration(double seconds) =>
      rust.setCrossfadeDuration(seconds: seconds);

  @override
  Future<void> setCrossfadeCurve(CrossfadeCurveType curve) =>
      rust.setCrossfadeCurve(curve: _curve(curve));

  @override
  Future<void> setSilenceThreshold(double db) =>
      rust.setSilenceThreshold(db: db);

  @override
  Future<void> setShuffle({required bool enabled}) =>
      rust.setShuffle(enabled: enabled);

  @override
  Future<void> setRepeat({required bool enabled}) =>
      rust.setRepeat(enabled: enabled);

  @override
  Future<void> resumeAutoMix() => rust.resumeAutomix();

  @override
  Future<void> mixNow() => rust.mixNow();

  @override
  Future<void> togglePlay(DeckSide deck) => rust.togglePlay(deck: _deck(deck));

  @override
  Future<void> seek(DeckSide deck, double seconds) =>
      rust.seek(deck: _deck(deck), seconds: seconds);

  @override
  Future<void> setCrossfader(double position) =>
      rust.setCrossfader(position: position.clamp(0, 1));

  @override
  Future<void> setKeylock(DeckSide deck, {required bool enabled}) =>
      rust.setKeylock(deck: _deck(deck), enabled: enabled);

  @override
  Future<void> setPitch(DeckSide deck, double percent) =>
      rust.setPitch(deck: _deck(deck), percent: percent.clamp(-50, 50));

  @override
  Future<void> setDeckGain(DeckSide deck, double linear) =>
      rust.setDeckGain(deck: _deck(deck), linear: linear);

  @override
  Future<void> setEq(DeckSide deck, EqBandType band, double linear) =>
      rust.setEq(deck: _deck(deck), band: _band(band), linear: linear);

  @override
  Future<void> setMasterGain(double linear) =>
      rust.setMasterGain(linear: linear);

  @override
  Future<void> moveInQueue(int from, int to) =>
      rust.moveInQueue(from: from, to: to);

  @override
  Future<void> loadTrack(DeckSide deck, int trackId) =>
      rust.loadTrack(deck: _deck(deck), trackId: BigInt.from(trackId));
}

/// Maps one bridge update to a domain event.
EngineEvent mapUpdate(rust.EngineUpdate update) {
  return switch (update) {
    rust.EngineUpdate_LibraryChanged(:final tracks) => LibraryChanged(
      tracks.map(_track).toList(),
    ),
    rust.EngineUpdate_DeckLoading(:final deck, :final trackId) => DeckLoading(
      deck: _side(deck),
      trackId: trackId.toInt(),
    ),
    rust.EngineUpdate_DeckLoaded(
      :final deck,
      :final trackId,
      :final durationSeconds,
      :final startSeconds,
      :final endSeconds,
    ) =>
      DeckLoaded(
        deck: _side(deck),
        trackId: trackId.toInt(),
        duration: durationSeconds,
        start: startSeconds,
        end: endSeconds,
      ),
    rust.EngineUpdate_Waveform(
      :final deck,
      :final trackId,
      :final pointsPerSecond,
      :final low,
      :final mid,
      :final high,
    ) =>
      WaveformReady(
        deck: _side(deck),
        trackId: trackId.toInt(),
        waveform: WaveformData(
          pointsPerSecond: pointsPerSecond,
          low: low,
          mid: mid,
          high: high,
        ),
      ),
    rust.EngineUpdate_TrackAnalyzed(
      :final trackId,
      :final bpm,
      :final firstBeatSeconds,
    ) =>
      TrackAnalyzed(
        trackId: trackId.toInt(),
        bpm: bpm,
        firstBeat: firstBeatSeconds,
      ),
    rust.EngineUpdate_DeckPosition(
      :final deck,
      :final positionSeconds,
      :final playing,
    ) =>
      DeckPositionChanged(
        deck: _side(deck),
        position: positionSeconds,
        playing: playing,
      ),
    rust.EngineUpdate_Levels(:final field0) => LevelsChanged(
      LevelSnapshot(
        deckA: _peak(field0.deckA),
        deckB: _peak(field0.deckB),
        master: _peak(field0.master),
      ),
    ),
    rust.EngineUpdate_AutoMixChanged(:final state, :final status) => _autoMix(
      state,
      status,
    ),
    rust.EngineUpdate_QueueChanged(:final upcoming) => QueueChanged([
      for (final id in upcoming) id.toInt(),
    ]),
    rust.EngineUpdate_TransitionStarted(:final from, :final to) =>
      TransitionStarted(from: _side(from), to: _side(to)),
    rust.EngineUpdate_TransitionCompleted(:final from, :final to) =>
      TransitionCompleted(from: _side(from), to: _side(to)),
    rust.EngineUpdate_TrackSkipped(:final trackId, :final reason) =>
      TrackSkipped(trackId: trackId.toInt(), reason: reason),
    rust.EngineUpdate_QueueExhausted() => const QueueExhausted(),
    rust.EngineUpdate_Underrun(:final deck) => Underrun(_side(deck)),
  };
}

AutoMixChanged _autoMix(rust.AutoMixStateDto state, rust.AutoMixStatusDto s) {
  final mode = switch (s) {
    rust.AutoMixStatusDto.off => AutoMixMode.off,
    rust.AutoMixStatusDto.running => AutoMixMode.running,
    rust.AutoMixStatusDto.suspended => AutoMixMode.suspended,
  };
  return switch (state) {
    rust.AutoMixStateDto_Idle() => AutoMixChanged(
      phase: AutoMixPhase.idle,
      mode: mode,
    ),
    rust.AutoMixStateDto_Playing(:final active) => AutoMixChanged(
      phase: AutoMixPhase.playing,
      mode: mode,
      active: _side(active),
    ),
    rust.AutoMixStateDto_Crossfading(:final from, :final to) => AutoMixChanged(
      phase: AutoMixPhase.crossfading,
      mode: mode,
      from: _side(from),
      to: _side(to),
    ),
    rust.AutoMixStateDto_WaitingForNext(:final deck) => AutoMixChanged(
      phase: AutoMixPhase.waitingForNext,
      mode: mode,
      active: _side(deck),
    ),
    rust.AutoMixStateDto_Finished() => AutoMixChanged(
      phase: AutoMixPhase.finished,
      mode: mode,
    ),
  };
}

TrackInfo _track(rust.TrackDto track) => TrackInfo(
  id: track.id.toInt(),
  path: track.path,
  title: track.title,
  artist: track.artist,
  bpm: track.bpm,
);

double _peak(List<double> stereo) =>
    stereo.fold<double>(0, (peak, value) => value > peak ? value : peak);

DeckSide _side(rust.Deck deck) => deck == rust.Deck.a ? DeckSide.a : DeckSide.b;

rust.Deck _deck(DeckSide side) =>
    side == DeckSide.a ? rust.Deck.a : rust.Deck.b;

rust.CurveKind _curve(CrossfadeCurveType curve) => switch (curve) {
  CrossfadeCurveType.equalPower => rust.CurveKind.equalPower,
  CrossfadeCurveType.linear => rust.CurveKind.linear,
  CrossfadeCurveType.cut => rust.CurveKind.cut,
};

rust.Band _band(EqBandType band) => switch (band) {
  EqBandType.low => rust.Band.low,
  EqBandType.mid => rust.Band.mid,
  EqBandType.high => rust.Band.high,
};
