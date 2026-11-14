import 'dart:async';

import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/settings/data/prefs_settings_repository.dart';
import 'package:platine/features/settings/domain/settings_repository.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'settings_controller.g.dart';

@Riverpod(keepAlive: true)
SettingsRepository settingsRepository(Ref ref) => PrefsSettingsRepository();

/// Current mix settings: persisted and pushed to the engine on every change.
@Riverpod(keepAlive: true)
class MixSettingsController extends _$MixSettingsController {
  @override
  MixSettings build() => const MixSettings();

  /// Loads the saved settings (called once at start-up).
  Future<MixSettings> restore() async {
    state = await ref.read(settingsRepositoryProvider).load();
    return state;
  }

  void setAutoMix({required bool enabled}) {
    _update(state.copyWith(automix: enabled));
    unawaited(ref.read(engineGatewayProvider).setAutoMix(enabled: enabled));
  }

  void setCrossfadeSeconds(double seconds) {
    final value = seconds.clamp(0, MixSettings.maxCrossfadeSeconds).toDouble();
    _update(state.copyWith(crossfadeSeconds: value));
    unawaited(ref.read(engineGatewayProvider).setCrossfadeDuration(value));
  }

  void setCurve(CrossfadeCurveType curve) {
    _update(state.copyWith(curve: curve));
    unawaited(ref.read(engineGatewayProvider).setCrossfadeCurve(curve));
  }

  void setShuffle({required bool enabled}) {
    _update(state.copyWith(shuffle: enabled));
    unawaited(ref.read(engineGatewayProvider).setShuffle(enabled: enabled));
  }

  void setRepeat({required bool enabled}) {
    _update(state.copyWith(repeat: enabled));
    unawaited(ref.read(engineGatewayProvider).setRepeat(enabled: enabled));
  }

  void setSilenceThreshold(double db) {
    _update(state.copyWith(silenceThresholdDb: db));
    unawaited(ref.read(engineGatewayProvider).setSilenceThreshold(db));
  }

  void _update(MixSettings settings) {
    state = settings;
    unawaited(ref.read(settingsRepositoryProvider).save(settings));
  }
}

/// Restores the settings and starts the audio engine.
@Riverpod(keepAlive: true)
Future<void> engineStartup(Ref ref) async {
  final settings = await ref
      .read(mixSettingsControllerProvider.notifier)
      .restore();
  // Subscribe to engine updates before the engine starts publishing.
  ref.read(sessionStoreProvider);
  await ref.read(engineGatewayProvider).start(settings);
}
