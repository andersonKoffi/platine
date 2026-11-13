import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:platine/app.dart';
import 'package:platine/core/bridge/rust_bootstrap.dart';
import 'package:platine/core/diagnostics/data/frb_diagnostics_repository.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(initNativeBridge);

  test('the real Rust engine answers pong', () {
    expect(const FrbDiagnosticsRepository().ping(), 'pong');
  });

  test('native crate versions are exposed', () {
    final versions = const FrbDiagnosticsRepository().nativeVersions();
    expect(versions.domain, isNotEmpty);
    expect(versions.engine, isNotEmpty);
    expect(versions.infra, isNotEmpty);
  });

  testWidgets('the app shows a healthy engine badge', (tester) async {
    await tester.pumpWidget(const ProviderScope(child: PlatineApp()));
    expect(find.text('Engine: pong'), findsOneWidget);
  });
}
