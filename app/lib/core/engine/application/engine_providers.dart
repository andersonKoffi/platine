import 'dart:async';

import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/data/frb_engine_gateway.dart';
import 'package:platine/core/engine/domain/engine_event.dart';
import 'package:platine/core/engine/domain/engine_gateway.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'engine_providers.g.dart';

/// The audio engine port. Overridden with a fake in tests.
@Riverpod(keepAlive: true)
EngineGateway engineGateway(Ref ref) => FrbEngineGateway();

/// Single source of truth of what the engine reports.
@Riverpod(keepAlive: true)
class SessionStore extends _$SessionStore {
  @override
  SessionState build() {
    final subscription = ref
        .watch(engineGatewayProvider)
        .events
        .listen(_apply);
    ref.onDispose(() => unawaited(subscription.cancel()));
    return const SessionState();
  }

  void _apply(EngineEvent event) {
    state = reduceSession(state, event, DateTime.now());
  }

  /// The user moved the crossfader: it leaves any automatic animation.
  void crossfaderMoved(double position) {
    state = state.copyWith(crossfader: position, clearTransition: true);
  }

  /// Shows a short message to the user.
  void notify(String message, {String? details}) {
    state = state.copyWith(
      notice: Notice(
        id: (state.notice?.id ?? 0) + 1,
        message: message,
        details: details,
      ),
    );
  }
}
