import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

import '../helpers/fakes.dart';

void main() {
  late FakeEngineGateway gateway;
  late InMemorySettingsRepository repository;
  late ProviderContainer container;

  setUp(() {
    gateway = FakeEngineGateway();
    repository = InMemorySettingsRepository(
      const MixSettings(crossfadeSeconds: 12, shuffle: true),
    );
    container = ProviderContainer(
      overrides: testOverrides(gateway, settings: repository),
    );
    addTearDown(container.dispose);
  });

  test('start-up restores saved settings and starts the engine', () async {
    await container.read(engineStartupProvider.future);
    expect(gateway.startedWith?.crossfadeSeconds, 12);
    expect(container.read(mixSettingsControllerProvider).shuffle, isTrue);
  });

  test('changes are persisted and sent to the engine', () async {
    final controller = container.read(mixSettingsControllerProvider.notifier)
      ..setCrossfadeSeconds(42)
      ..setCurve(CrossfadeCurveType.cut)
      ..setAutoMix(enabled: false)
      ..setShuffle(enabled: false)
      ..setRepeat(enabled: true)
      ..setSilenceThreshold(-60);
    await Future<void>.delayed(Duration.zero);

    final settings = container.read(mixSettingsControllerProvider);
    expect(settings.crossfadeSeconds, MixSettings.maxCrossfadeSeconds);
    expect(repository.stored, settings);
    expect(repository.saves, 6);
    expect(gateway.calls, [
      'setCrossfadeDuration 30.0',
      'setCrossfadeCurve cut',
      'setAutoMix false',
      'setShuffle false',
      'setRepeat true',
      'setSilenceThreshold -60.0',
    ]);
    expect(controller, isNotNull);
    expect(settings.hashCode, repository.stored.hashCode);
  });

  test('a failing engine start is reported', () async {
    gateway.startError = StateError('no device');
    await expectLater(
      container.read(engineStartupProvider.future),
      throwsStateError,
    );
  });

  test('mono output is persisted and sent to the engine', () async {
    await container.read(engineStartupProvider.future);
    container
        .read(mixSettingsControllerProvider.notifier)
        .setMonoOutput(enabled: true);
    expect(container.read(mixSettingsControllerProvider).monoOutput, isTrue);
    expect(gateway.calls, contains('setMonoOutput true'));
    await pumpEventQueue();
    expect((await repository.load()).monoOutput, isTrue);
  });
}
