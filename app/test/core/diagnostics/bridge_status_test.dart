import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:platine/core/diagnostics/application/diagnostics_providers.dart';
import 'package:platine/core/diagnostics/domain/diagnostics_repository.dart';

class _MockDiagnosticsRepository extends Mock implements DiagnosticsRepository;

void main() {
  late _MockDiagnosticsRepository repository;

  ProviderContainer makeContainer() {
    final container = ProviderContainer(
      overrides: [diagnosticsRepositoryProvider.overrideWithValue(repository)],
    );
    addTearDown(container.dispose);
    return container;
  }

  setUp(() => repository = _MockDiagnosticsRepository());

  test('reports connected when the engine answers pong', () {
    when(() => repository.ping()).thenReturn('pong');

    final status = makeContainer().read(bridgeStatusProvider);

    expect(status, BridgeStatus.connected);
    verify(() => repository.ping()).called(1);
  });

  test('reports an unexpected answer otherwise', () {
    when(() => repository.ping()).thenReturn('nope');

    final status = makeContainer().read(bridgeStatusProvider);

    expect(status, BridgeStatus.unexpectedAnswer);
  });
}
