import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/library/application/library_controller.dart';

/// Library with instant search, plus the reorderable play queue.
class LibraryPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    return GlassPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _LibraryToolbar(),
          SizedBox(height: tokens.space3),
          const Expanded(flex: 3, child: _TrackList()),
          Divider(color: tokens.glassBorder, height: tokens.space5),
          Text('À suivre', style: Theme.of(context).textTheme.titleMedium),
          SizedBox(height: tokens.space2),
          const Expanded(flex: 2, child: _QueueList()),
        ],
      ),
    );
  }
}

class _LibraryToolbar extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final query = ref.watch(libraryQueryControllerProvider);
    final controller = ref.read(libraryQueryControllerProvider.notifier);
    final count = ref.watch(
      sessionStoreProvider.select((s) => s.library.length),
    );
    return Row(
      children: [
        Expanded(
          child: TextField(
            onChanged: controller.search,
            decoration: InputDecoration(
              prefixIcon: const Icon(Icons.search_rounded),
              hintText: 'Rechercher parmi $count titres',
              isDense: true,
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(tokens.radiusMedium),
              ),
            ),
          ),
        ),
        SizedBox(width: tokens.space2),
        SegmentedButton<LibrarySort>(
          segments: const [
            ButtonSegment(
              value: LibrarySort.folder,
              icon: Icon(Icons.folder_outlined),
              tooltip: 'Ordre du dossier',
            ),
            ButtonSegment(
              value: LibrarySort.title,
              icon: Icon(Icons.sort_by_alpha_rounded),
              tooltip: 'Trier par titre',
            ),
          ],
          selected: {query.sort},
          showSelectedIcon: false,
          onSelectionChanged: (selection) => controller.sortBy(selection.first),
        ),
        SizedBox(width: tokens.space2),
        IconButton.filledTonal(
          tooltip: 'Ajouter un dossier à la file',
          onPressed: () => unawaited(
            ref
                .read(libraryLoaderProvider)
                .pickAndLoad(mode: FolderLoadMode.append),
          ),
          icon: const Icon(Icons.playlist_add_rounded),
        ),
      ],
    );
  }
}

class _TrackList extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final tracks = ref.watch(visibleTracksProvider);
    final nowPlaying = ref.watch(
      sessionStoreProvider.select((s) {
        final active = s.activeDeck;
        return active == null ? null : s.deck(active).trackId;
      }),
    );
    final next = ref.watch(sessionStoreProvider.select((s) => s.nextTrackId));
    if (tracks.isEmpty) {
      return Center(
        child: Text(
          'Aucun titre ne correspond.',
          style: TextStyle(color: tokens.textMuted),
        ),
      );
    }
    return ListView.builder(
      itemCount: tracks.length,
      itemExtent: 52,
      itemBuilder: (context, index) => _TrackRow(
        track: tracks[index],
        isPlaying: tracks[index].id == nowPlaying,
        isNext: tracks[index].id == next,
      ),
    );
  }
}

class _TrackRow extends ConsumerWidget {
  const new({
    required this.track,
    required this.isPlaying,
    required this.isNext,
  });

  final TrackInfo track;
  final bool isPlaying;
  final bool isNext;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final highlight = isPlaying
        ? tokens.neonCyan
        : (isNext ? tokens.neonViolet : null);
    final gateway = ref.read(engineGatewayProvider);
    return Semantics(
      label: [
        track.title,
        if (isPlaying) 'en cours',
        if (isNext) 'à suivre',
      ].join(', '),
      child: AnimatedContainer(
        duration: tokens.durationMedium,
        margin: EdgeInsets.symmetric(vertical: tokens.space1 / 2),
        decoration: BoxDecoration(
          color: highlight?.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(tokens.radiusMedium),
          border: Border.all(color: highlight ?? Colors.transparent),
        ),
        child: Row(
          children: [
            SizedBox(width: tokens.space3),
            Icon(
              isPlaying
                  ? Icons.graphic_eq_rounded
                  : (isNext ? Icons.skip_next_rounded : Icons.music_note),
              color: highlight ?? tokens.textMuted,
              size: 20,
            ),
            SizedBox(width: tokens.space3),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    track.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    track.artist ?? _folderOf(track.path),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: tokens.textMuted, fontSize: 12),
                  ),
                ],
              ),
            ),
            for (final side in DeckSide.values)
              IconButton(
                tooltip: 'Charger sur la platine ${side.label}',
                onPressed: () => unawaited(gateway.loadTrack(side, track.id)),
                icon: Text(
                  side.label,
                  style: TextStyle(
                    color: tokens.deckColor(isDeckA: side == DeckSide.a),
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            SizedBox(width: tokens.space1),
          ],
        ),
      ),
    );
  }

  static String _folderOf(String path) {
    final parts = path.split('/')..removeLast();
    return parts.isEmpty ? '' : parts.last;
  }
}

class _QueueList extends ConsumerWidget {
  const new();

  static String _titleOf(List<TrackInfo> library, int id) {
    for (final track in library) {
      if (track.id == id) return track.title;
    }
    return 'Titre $id';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final upcoming = ref.watch(sessionStoreProvider.select((s) => s.upcoming));
    final library = ref.watch(sessionStoreProvider.select((s) => s.library));
    if (upcoming.isEmpty) {
      return Center(
        child: Text(
          'La file est vide.',
          style: TextStyle(color: tokens.textMuted),
        ),
      );
    }
    return ReorderableListView.builder(
      itemCount: upcoming.length,
      buildDefaultDragHandles: false,
      onReorderItem: (from, to) =>
          unawaited(ref.read(engineGatewayProvider).moveInQueue(from, to)),
      itemBuilder: (context, index) {
        final id = upcoming[index];
        final title = _titleOf(library, id);
        return ListTile(
          key: ValueKey(id),
          dense: true,
          leading: Text('${index + 1}', style: counterStyle(context, size: 14)),
          title: Text(title, maxLines: 1, overflow: TextOverflow.ellipsis),
          trailing: ReorderableDragStartListener(
            index: index,
            child: Semantics(
              label: 'Déplacer $title',
              child: const Icon(Icons.drag_indicator_rounded),
            ),
          ),
        );
      },
    );
  }
}
