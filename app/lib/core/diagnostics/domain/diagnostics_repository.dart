import 'package:platine/core/diagnostics/domain/native_versions.dart';

/// Port to the native engine's diagnostics endpoints.
abstract interface class DiagnosticsRepository {
  /// Returns `"pong"` when the bridge is alive.
  String ping();

  /// Returns the versions of the native crates.
  NativeVersions nativeVersions();
}
