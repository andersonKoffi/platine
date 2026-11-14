import 'dart:io';

import 'package:desktop_drop/desktop_drop.dart';
import 'package:material_ui/material_ui.dart';

/// Drop zone backed by `desktop_drop` (native macOS drag and drop).
Widget desktopDropZone({
  required Widget child,
  required Future<void> Function(String? folder) onFolder,
  required VoidCallback onEnter,
  required VoidCallback onLeave,
}) {
  return DropTarget(
    onEnter: (_) => onEnter(),
    onLeave: onLeave,
    onDrop: (details) => onFolder(folderFromDrop(details.items)),
    child: child,
  );
}

/// Folder to load from dropped items: a dropped folder, or the folder of a
/// dropped file.
String? folderFromDrop(List<DropItem> items) {
  for (final item in items) {
    switch (item) {
      case DropDirectoryItem(:final localPath?):
        return localPath;
      case DropFileItem(:final localPath?):
        return File(localPath).parent.path;
      case DropDirectoryItem() || DropFileItem() || DropStringItem():
        continue;
    }
  }
  return null;
}
