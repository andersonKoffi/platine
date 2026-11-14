import 'package:file_selector/file_selector.dart';

/// Folder picker using the native macOS open panel.
Future<String?> pickFolderWithFileSelector() =>
    getDirectoryPath(confirmButtonText: 'Choisir');
