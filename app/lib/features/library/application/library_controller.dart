import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/library/data/open_panel_folder_picker.dart';
import 'package:platine/features/library/domain/folder_picker.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'library_controller.g.dart';

/// How the library list is ordered.
enum LibrarySort { folder, title }

/// Search text and sort order of the library list.
class LibraryQuery {
  const new({this.text = '', this.sort = LibrarySort.folder});

  final String text;
  final LibrarySort sort;
}

@Riverpod(keepAlive: true)
FolderPicker folderPicker(Ref ref) => pickFolderWithOpenPanel;

@Riverpod(keepAlive: true)
class LibraryQueryController extends _$LibraryQueryController {
  @override
  LibraryQuery build() => const LibraryQuery();

  void search(String text) => state = LibraryQuery(text: text, sort: state.sort);

  void sortBy(LibrarySort sort) =>
      state = LibraryQuery(text: state.text, sort: sort);
}

/// Library tracks matching the query, in the chosen order.
@riverpod
List<TrackInfo> visibleTracks(Ref ref) {
  final library = ref.watch(sessionStoreProvider.select((s) => s.library));
  final query = ref.watch(libraryQueryControllerProvider);
  return filterTracks(library, query);
}

/// Pure filtering, shared with tests.
List<TrackInfo> filterTracks(List<TrackInfo> tracks, LibraryQuery query) {
  final needle = query.text.trim().toLowerCase();
  final matching = needle.isEmpty
      ? [...tracks]
      : [
          for (final track in tracks)
            if (track.title.toLowerCase().contains(needle) ||
                (track.artist?.toLowerCase().contains(needle) ?? false) ||
                track.path.toLowerCase().contains(needle))
              track,
        ];
  if (query.sort == LibrarySort.title) {
    matching.sort(
      (a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()),
    );
  }
  return matching;
}

@Riverpod(keepAlive: true)
LibraryLoader libraryLoader(Ref ref) => LibraryLoader(ref);

/// Use case: load a folder (picker or drag and drop) and report problems.
class LibraryLoader {
  new(this._ref);

  final Ref _ref;

  /// Opens the folder dialog and loads the chosen folder.
  Future<void> pickAndLoad({FolderLoadMode? mode}) async {
    final path = await _ref.read(folderPickerProvider)();
    if (path == null) return;
    await load(path, mode: mode);
  }

  /// Loads [path]. [mode] defaults to replacing the queue.
  Future<void> load(String path, {FolderLoadMode? mode}) async {
    final store = _ref.read(sessionStoreProvider.notifier);
    try {
      final result = await _ref
          .read(engineGatewayProvider)
          .loadFolder(path, mode ?? FolderLoadMode.replace);
      if (result.tracks.isEmpty) {
        store.notify('Aucun fichier audio dans ce dossier.');
      } else if (result.skipped.isNotEmpty) {
        store.notify(
          '${result.tracks.length} titres chargés, '
          '${result.skipped.length} éléments illisibles ignorés.',
          details: result.skipped
              .map((entry) => '${entry.path} : ${entry.reason}')
              .join('\n'),
        );
      }
    } on Object catch (error) {
      store.notify('Impossible de lire ce dossier.', details: '$error');
    }
  }
}
