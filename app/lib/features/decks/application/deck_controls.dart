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
  });

  static const double maxLevel = 2;

  final double gain;
  final double low;
  final double mid;
  final double high;

  /// Speed change in percent, within [pitchRange].
  final double pitch;
  final PitchRangeOption pitchRange;

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
  }) => DeckControls(
    gain: gain ?? this.gain,
    low: low ?? this.low,
    mid: mid ?? this.mid,
    high: high ?? this.high,
    pitch: pitch ?? this.pitch,
    pitchRange: pitchRange ?? this.pitchRange,
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
