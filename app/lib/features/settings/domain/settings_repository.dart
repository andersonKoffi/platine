import 'package:platine/core/engine/domain/models.dart';

/// Persistence of the user's mix settings.
abstract interface class SettingsRepository {
  Future<MixSettings> load();

  Future<void> save(MixSettings settings);
}
