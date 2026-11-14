import 'package:platine/features/shell/data/desktop_drop_zone.dart';
import 'package:platine/features/shell/domain/drop_zone.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'drop_zone_provider.g.dart';

/// Drop zone implementation (native in the app, pass-through in tests).
@Riverpod(keepAlive: true)
DropZoneBuilder dropZone(Ref ref) => desktopDropZone;
