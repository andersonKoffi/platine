import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:platine/app.dart';
import 'package:platine/core/bridge/rust_bootstrap.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initNativeBridge();
  runApp(const ProviderScope(child: PlatineApp()));
}
