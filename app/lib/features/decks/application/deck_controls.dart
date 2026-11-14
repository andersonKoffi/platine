import 'dart:async';

import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'deck_controls.g.dart';

/// Ranges of the pitch fader, in percent.
enum PitchRangeOption {
  eight(8),
  sixteen(16),
  fifty(50);

  const new(this.percent);

  final double percent;

  PitchRangeOption get next =>
      PitchRangeOption.values[(index + 1) % PitchRangeOption.values.length];
}

/// Gain and isolator settings of a deck (linear: 1 = neutral, 0 = kill),
/// and its pitch fader.
class DeckControls {
  const new({
    this.gain = 1,
    this.low = 1,
    this.mid = 1,
    this.high = 1,
    this.pitch = 0,
    this.pitchRange = PitchRangeOption.eight,
    this.keylock = false,
  });

  static const double maxLevel = 2;

  final double gain;
  final double low;
  final double mid;
  final double high;

  /// Speed change in percent, within [pitchRange].
  final double pitch;
  final PitchRangeOption pitchRange;

  /// Pitch changes keep the key of the track.
  final bool keylock;

  /// Speed ratio (1 = original speed).
  double get speed => 1 + pitch / 100;

  /// Pitch fader position: 0 = slowest, 0.5 = original, 1 = fastest.
  double get pitchPosition => 0.5 + pitch / (2 * pitchRange.percent);

  DeckControls copyWith({
    double? gain,
    double? low,
    double? mid,
    double? high,
    double? pitch,
    PitchRangeOption? pitchRange,
    bool? keylock,
  }) => DeckControls(
    gain: gain ?? this.gain,
    low: low ?? this.low,
    mid: mid ?? this.mid,
    high: high ?? this.high,
    pitch: pitch ?? this.pitch,
    pitchRange: pitchRange ?? this.pitchRange,
    keylock: keylock ?? this.keylock,
  );

  double band(EqBandType band) => switch (band) {
    EqBandType.low => low,
    EqBandType.mid => mid,
    EqBandType.high => high,
  };

  DeckControls withBand(EqBandType band, double value) => switch (band) {
    EqBandType.low => copyWith(low: value),
    EqBandType.mid => copyWith(mid: value),
    EqBandType.high => copyWith(high: value),
  };

  DeckControls withGain(double value) => copyWith(gain: value);
}

/// Gain, EQ and pitch of one deck, forwarded to the engine.
@Riverpod(keepAlive: true)
class DeckControlsController extends _$DeckControlsController {
  late DeckSide _side;

  @override
  DeckControls build(DeckSide side) {
    _side = side;
    return const DeckControls();
  }

  void setGain(double value) {
    final gain = value.clamp(0, DeckControls.maxLevel).toDouble();
    state = state.withGain(gain);
    unawaited(ref.read(engineGatewayProvider).setDeckGain(_side, gain));
  }

  void setBand(EqBandType band, double value) {
    final level = value.clamp(0, DeckControls.maxLevel).toDouble();
    state = state.withBand(band, level);
    unawaited(ref.read(engineGatewayProvider).setEq(_side, band, level));
  }

  /// Sets the pitch in percent, clamped to the current range.
  void setPitch(double percent) {
    final range = state.pitchRange.percent;
    final pitch = (percent.clamp(-range, range) * 100).round() / 100;
    if (pitch == state.pitch) return;
    state = state.copyWith(pitch: pitch);
    unawaited(ref.read(engineGatewayProvider).setPitch(_side, pitch));
  }

  /// Moves the fader to `position` (0 = slowest, 1 = fastest).
  void setPitchPosition(double position) =>
      setPitch((position.clamp(0, 1) - 0.5) * 2 * state.pitchRange.percent);

  void resetPitch() => setPitch(0);

  void toggleKeylock() {
    state = state.copyWith(keylock: !state.keylock);
    unawaited(
      ref
          .read(engineGatewayProvider)
          .setKeylock(_side, enabled: state.keylock),
    );
  }

  /// Sets the pitch so that this deck plays at [targetBpm] (SYNC), widening
  /// the range if needed. Returns `false` when the tempos are too far apart.
  bool syncTo({required double bpm, required double targetBpm}) {
    final percent = syncPitch(bpm: bpm, targetBpm: targetBpm);
    if (percent == null) return false;
    final needed = rangeFor(percent);
    if (needed.percent > state.pitchRange.percent) {
      state = state.copyWith(pitchRange: needed);
    }
    setPitch(percent);
    return true;
  }

  /// Next range (±8 → ±16 → ±50 → ±8); the pitch is clamped to it.
  void cycleRange() {
    final range = state.pitchRange.next;
    state = state.copyWith(pitchRange: range);
    if (state.pitch.abs() > range.percent) setPitch(state.pitch);
  }

  /// Kills the band, or restores it to neutral if it is already killed.
  void toggleKill(EqBandType band) {
    setBand(band, state.band(band) == 0 ? 1 : 0);
  }
}

/// Pitch (in percent) that makes a track at [bpm] play at [targetBpm],
/// allowing half and double tempo (70 BPM syncs to 140 at +0 %).
/// `null` when no option fits within ±50 %.
double? syncPitch({required double bpm, required double targetBpm}) {
  if (bpm <= 0 || targetBpm <= 0) return null;
  double? best;
  for (final factor in const [1.0, 2.0, 0.5]) {
    final percent = (targetBpm / (bpm * factor) - 1) * 100;
    if (percent.abs() <= 50 && (best == null || percent.abs() < best.abs())) {
      best = percent;
    }
  }
  return best == null ? null : (best * 100).round() / 100;
}

/// Smallest range that contains [percent].
PitchRangeOption rangeFor(double percent) => PitchRangeOption.values.firstWhere(
  (range) => percent.abs() <= range.percent,
  orElse: () => PitchRangeOption.fifty,
);
