import 'package:material_ui/material_ui.dart';
import 'package:platine/core/diagnostics/presentation/bridge_status_badge.dart';

/// Root widget of Platine.
///
/// P0 shell: the design system, decks, mixer and library land in P4.
class PlatineApp extends StatelessWidget {
  const PlatineApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Platine',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        colorSchemeSeed: Colors.deepPurple,
      ),
      home: const _ShellPage(),
    );
  }
}

class _ShellPage extends StatelessWidget {
  const _ShellPage();

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('Platine', style: textTheme.displayMedium),
            const SizedBox(height: 16),
            const BridgeStatusBadge(),
          ],
        ),
      ),
    );
  }
}
