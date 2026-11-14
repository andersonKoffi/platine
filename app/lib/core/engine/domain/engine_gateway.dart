import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/models.dart';

/// Port to the Rust audio engine. Implemented by the bridge adapter in
/// `data/`, and by fakes in tests.
abstract interface class EngineGateway {
  /// Engine updates; available before [start] and kept across restarts.
  Stream<EngineEvent> get events;

  /// Opens the audio output and starts the engine with [settings].
  Future<void> start(MixSettings settings);

  Future<ScanResult> loadFolder(String path, FolderLoadMode mode);

  Future<void> setAutoMix({required bool enabled});

  Future<void> setCrossfadeDuration(double seconds);

  Future<void> setCrossfadeCurve(CrossfadeCurveType curve);

  Future<void> setSilenceThreshold(double db);

  Future<void> setShuffle({required bool enabled});

  Future<void> setRepeat({required bool enabled});

  Future<void> resumeAutoMix();

  /// Starts the fade to the next track right away.
  Future<void> mixNow();

  Future<void> togglePlay(DeckSide deck);

  Future<void> seek(DeckSide deck, double seconds);

  /// 0 = only deck A, 1 = only deck B.
  Future<void> setCrossfader(double position);

  Future<void> setDeckGain(DeckSide deck, double linear);

  /// Pitch fader: speed change in percent (`+4` = 4 % faster).
  Future<void> setPitch(DeckSide deck, double percent);

  Future<void> setEq(DeckSide deck, EqBandType band, double linear);

  Future<void> setMasterGain(double linear);

  Future<void> moveInQueue(int from, int to);

  Future<void> loadTrack(DeckSide deck, int trackId);
}
