import 'dart:async';

import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'mixer_controller.g.dart';

/// Master volume (linear, 1 = 0 dB).
@Riverpod(keepAlive: true)
class MasterGain extends _$MasterGain {
  static const double max = 1.5;

  @override
  double build() => 1;

  void set(double value) {
    state = value.clamp(0, max).toDouble();
    unawaited(ref.read(engineGatewayProvider).setMasterGain(state));
  }

  void nudge(double delta) => set(state + delta);
}

@Riverpod(keepAlive: true)
CrossfaderController crossfaderController(Ref ref) => CrossfaderController(ref);

/// Manual crossfader moves. Touching the crossfader suspends the auto-mix.
class CrossfaderController {
  new(this._ref);

  final Ref _ref;

  void move(double position) {
    final value = position.clamp(0, 1).toDouble();
    _ref.read(sessionStoreProvider.notifier).crossfaderMoved(value);
    unawaited(_ref.read(engineGatewayProvider).setCrossfader(value));
  }

  void nudge(double delta) {
    move(_ref.read(sessionStoreProvider).crossfader + delta);
  }

  void toDeck(DeckSide side) => move(side.crossfaderSide);
}
