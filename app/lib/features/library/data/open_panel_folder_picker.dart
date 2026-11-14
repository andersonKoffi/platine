import 'package:flutter/services.dart';

/// Channel implemented in `macos/Runner/MainFlutterWindow.swift`.
const MethodChannel folderPickerChannel = MethodChannel('platine/folder_picker');

/// Folder picker using the native macOS open panel (`NSOpenPanel`).
Future<String?> pickFolderWithOpenPanel() =>
    folderPickerChannel.invokeMethod<String>('pickFolder');
