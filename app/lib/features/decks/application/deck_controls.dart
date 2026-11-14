import 'dart:async';

import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'deck_controls.g.dart';

/// Gain and isolator settings of a deck (linear: 1 = neutral, 0 = kill).
class DeckControls {
  const new({this.gain = 1, this.low = 1, this.mid = 1, this.high = 1});

  static const double maxLevel = 2;

  final double gain;
  final double low;
  final double mid;
  final double high;

  double band(EqBandType band) => switch (band) {
    EqBandType.low => low,
    EqBandType.mid => mid,
    EqBandType.high => high,
  };

  DeckControls withBand(EqBandType band, double value) => switch (band) {
    EqBandType.low => DeckControls(
      gain: gain,
      low: value,
      mid: mid,
      high: high,
    ),
    EqBandType.mid => DeckControls(
      gain: gain,
      low: low,
      mid: value,
      high: high,
    ),
    EqBandType.high => DeckControls(
      gain: gain,
      low: low,
      mid: mid,
      high: value,
    ),
  };

  DeckControls withGain(double value) =>
      DeckControls(gain: value, low: low, mid: mid, high: high);
}

/// Gain and EQ of one deck, forwarded to the engine.
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

  /// Kills the band, or restores it to neutral if it is already killed.
  void toggleKill(EqBandType band) {
    setBand(band, state.band(band) == 0 ? 1 : 0);
  }
}
