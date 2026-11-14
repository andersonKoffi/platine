import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/theme.dart';
import 'package:platine/features/shell/presentation/platine_shell.dart';

/// Root widget of Platine.
class PlatineApp extends StatelessWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Platine',
      debugShowCheckedModeBanner: false,
      theme: buildPlatineTheme(),
      home: const PlatineShell(),
    );
  }
}
