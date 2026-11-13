import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/app.dart';
import 'package:platine/core/diagnostics/application/diagnostics_providers.dart';
import 'package:platine/core/diagnostics/domain/diagnostics_repository.dart';
import 'package:platine/core/diagnostics/domain/native_versions.dart';

class _FakeDiagnosticsRepository implements DiagnosticsRepository {
  const new();

  @override
  String ping() => 'pong';

  @override
  NativeVersions nativeVersions() =>
      const NativeVersions(domain: '0', engine: '0', infra: '0');
}

void main() {
  testWidgets('shows the app title and a healthy engine badge', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          diagnosticsRepositoryProvider.overrideWithValue(
            const _FakeDiagnosticsRepository(),
          ),
        ],
        child: const PlatineApp(),
      ),
    );

    expect(find.text('Platine'), findsOneWidget);
    expect(find.text('Engine: pong'), findsOneWidget);
    expect(find.byType(Chip), findsOneWidget);
  });
}
