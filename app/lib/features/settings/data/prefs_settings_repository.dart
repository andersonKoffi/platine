import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/settings/domain/settings_repository.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// [SettingsRepository] backed by `shared_preferences`.
class PrefsSettingsRepository implements SettingsRepository {
  new([SharedPreferencesAsync? prefs])
    : _prefs = prefs ?? SharedPreferencesAsync();

  final SharedPreferencesAsync _prefs;

  static const _automix = 'mix.automix';
  static const _crossfade = 'mix.crossfadeSeconds';
  static const _curve = 'mix.curve';
  static const _shuffle = 'mix.shuffle';
  static const _repeat = 'mix.repeat';
  static const _silence = 'mix.silenceThresholdDb';

  @override
  Future<MixSettings> load() async {
    const defaults = MixSettings();
    final curveName = await _prefs.getString(_curve);
    return MixSettings(
      automix: await _prefs.getBool(_automix) ?? defaults.automix,
      crossfadeSeconds:
          await _prefs.getDouble(_crossfade) ?? defaults.crossfadeSeconds,
      curve: CrossfadeCurveType.values.firstWhere(
        (curve) => curve.name == curveName,
        orElse: () => defaults.curve,
      ),
      shuffle: await _prefs.getBool(_shuffle) ?? defaults.shuffle,
      repeat: await _prefs.getBool(_repeat) ?? defaults.repeat,
      silenceThresholdDb:
          await _prefs.getDouble(_silence) ?? defaults.silenceThresholdDb,
    );
  }

  @override
  Future<void> save(MixSettings settings) async {
    await _prefs.setBool(_automix, settings.automix);
    await _prefs.setDouble(_crossfade, settings.crossfadeSeconds);
    await _prefs.setString(_curve, settings.curve.name);
    await _prefs.setBool(_shuffle, settings.shuffle);
    await _prefs.setBool(_repeat, settings.repeat);
    await _prefs.setDouble(_silence, settings.silenceThresholdDb);
  }
}
