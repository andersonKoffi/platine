import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:platine/core/diagnostics/data/frb_diagnostics_repository.dart';
import 'package:platine/core/diagnostics/domain/diagnostics_repository.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'diagnostics_providers.g.dart';

/// Diagnostics port. Overridden with a fake in tests.
@Riverpod(keepAlive: true)
DiagnosticsRepository diagnosticsRepository(Ref ref) =>
    const FrbDiagnosticsRepository();

/// Health of the native bridge.
enum BridgeStatus { connected, unexpectedAnswer }

/// Pings the native engine once and exposes the result.
@riverpod
BridgeStatus bridgeStatus(Ref ref) {
  final answer = ref.watch(diagnosticsRepositoryProvider).ping();
  return answer == 'pong'
      ? BridgeStatus.connected
      : BridgeStatus.unexpectedAnswer;
}
