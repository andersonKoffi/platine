import 'package:material_ui/material_ui.dart';

/// Wraps [child] in a zone accepting folders dropped from the Finder.
/// [onFolder] receives the folder to load (`null` if nothing usable was
/// dropped).
typedef DropZoneBuilder =
    Widget Function({
      required Widget child,
      required Future<void> Function(String? folder) onFolder,
      required VoidCallback onEnter,
      required VoidCallback onLeave,
    });
