import 'package:platine/core/diagnostics/domain/diagnostics_repository.dart';
import 'package:platine/core/diagnostics/domain/native_versions.dart';
import 'package:platine/src/rust/api/diagnostics.dart' as native;

/// Adapter: maps the generated bridge API to the domain port.
class FrbDiagnosticsRepository implements DiagnosticsRepository {
  const FrbDiagnosticsRepository();

  @override
  String ping() => native.ping();

  @override
  NativeVersions nativeVersions() {
    final versions = native.nativeVersions();
    return NativeVersions(
      domain: versions.domain,
      engine: versions.engine,
      infra: versions.infra,
    );
  }
}
