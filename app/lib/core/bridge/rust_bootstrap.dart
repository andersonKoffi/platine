import 'package:platine/src/rust/frb_generated.dart';

/// Loads the native library and starts the Rust runtime.
///
/// This file and the `data/` layers are the only places allowed to import
/// generated bridge code (enforced by `test/architecture`).
Future<void> initNativeBridge() => RustLib.init();
