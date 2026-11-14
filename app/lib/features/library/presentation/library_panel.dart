import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/library/application/library_controller.dart';

/// Browser: sources on the left, the track table on the right.
class LibraryPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    return PanelBox(
      padding: EdgeInsets.zero,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(width: 190, child: _Sources()),
          Container(width: 1, color: tokens.panelBorder),
          const Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _LibraryToolbar(),
                _TableHeader(),
                Expanded(child: _TrackTable()),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Sources extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final count = ref.watch(
      sessionStoreProvider.select((s) => s.library.length),
    );
    final loader = ref.read(libraryLoaderProvider);
    return Padding(
      padding: EdgeInsets.all(tokens.space2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Padding(
            padding: EdgeInsets.fromLTRB(6, 4, 0, 8),
            child: ControlLabel('Sources'),
          ),
          _SourceTile(
            icon: Icons.library_music_rounded,
            label: 'Bibliothèque',
            trailing: '$count',
            selected: true,
          ),
          const Spacer(),
          PadButton(
            label: 'OUVRIR',
            icon: Icons.folder_open_rounded,
            tooltip: 'Choisir un dossier (⌘O)',
            color: tokens.textSecondary,
            onPressed: () => unawaited(loader.pickAndLoad()),
          ),
          SizedBox(height: tokens.space2),
          PadButton(
            label: 'AJOUTER',
            icon: Icons.playlist_add_rounded,
            tooltip: 'Ajouter un dossier à la file',
            color: tokens.textSecondary,
            onPressed: () =>
                unawaited(loader.pickAndLoad(mode: FolderLoadMode.append)),
          ),
        ],
      ),
    );
  }
}

class _SourceTile extends StatelessWidget {
  const new({
    required this.icon,
    required this.label,
    required this.trailing,
    required this.selected,
  });

  final IconData icon;
  final String label;
  final String trailing;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Container(
      height: 30,
      padding: EdgeInsets.symmetric(horizontal: tokens.space2),
      decoration: BoxDecoration(
        color: selected ? tokens.panelRaised : null,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
      ),
      child: Row(
        children: [
          Icon(icon, size: 16, color: tokens.deckA),
          SizedBox(width: tokens.space2),
          Expanded(
            child: Text(label, style: Theme.of(context).textTheme.bodyMedium),
          ),
          Text(trailing, style: Theme.of(context).textTheme.bodySmall),
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
    return Padding(
      padding: EdgeInsets.all(tokens.space2),
      child: Row(
        children: [
          Expanded(
            child: SizedBox(
              height: 32,
              child: TextField(
                onChanged: controller.search,
                style: Theme.of(context).textTheme.bodyMedium,
                decoration: InputDecoration(
                  prefixIcon: Icon(
                    Icons.search_rounded,
                    size: 18,
                    color: tokens.textMuted,
                  ),
                  hintText: 'Rechercher parmi $count titres',
                ),
              ),
            ),
          ),
          SizedBox(width: tokens.space2),
          PadButton(
            label: 'DOSSIER',
            tooltip: 'Ordre du dossier',
            color: tokens.textSecondary,
            active: query.sort == LibrarySort.folder,
            onPressed: () => controller.sortBy(LibrarySort.folder),
          ),
          SizedBox(width: tokens.space1),
          PadButton(
            label: 'A→Z',
            tooltip: 'Trier par titre',
            color: tokens.textSecondary,
            active: query.sort == LibrarySort.title,
            onPressed: () => controller.sortBy(LibrarySort.title),
          ),
        ],
      ),
    );
  }
}

/// Column widths of the track table.
const double _numberWidth = 44;
const double _formatWidth = 56;
const double _loadWidth = 72;

class _TableHeader extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Container(
      height: 24,
      color: tokens.panelRaised,
      child: const Row(
        children: [
          SizedBox(
            width: _numberWidth,
            child: Center(child: ControlLabel('#')),
          ),
          Expanded(flex: 5, child: ControlLabel('Titre')),
          Expanded(flex: 3, child: ControlLabel('Artiste')),
          Expanded(flex: 3, child: ControlLabel('Dossier')),
          SizedBox(width: _formatWidth, child: ControlLabel('Format')),
          SizedBox(
            width: _loadWidth,
            child: Center(child: ControlLabel('Charger')),
          ),
        ],
      ),
    );
  }
}

class _TrackTable extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final tracks = ref.watch(visibleTracksProvider);
    final onDecks = ref.watch(
      sessionStoreProvider.select(
        (s) => (s.deckA.trackId, s.deckB.trackId, s.activeDeck),
      ),
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
    final (trackA, trackB, active) = onDecks;
    final playing = switch (active) {
      DeckSide.a => trackA,
      DeckSide.b => trackB,
      null => null,
    };
    return ListView.builder(
      itemCount: tracks.length,
      itemExtent: 30,
      itemBuilder: (context, index) {
        final track = tracks[index];
        final deck = track.id == trackA
            ? DeckSide.a
            : track.id == trackB
            ? DeckSide.b
            : null;
        return _TrackRow(
          index: index,
          track: track,
          deck: deck,
          isPlaying: track.id == playing,
          isNext: track.id == next,
        );
      },
    );
  }
}

class _TrackRow extends ConsumerWidget {
  const new({
    required this.index,
    required this.track,
    required this.deck,
    required this.isPlaying,
    required this.isNext,
  });

  final int index;
  final TrackInfo track;

  /// Deck holding this track, if any.
  final DeckSide? deck;
  final bool isPlaying;
  final bool isNext;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final textTheme = Theme.of(context).textTheme;
    final deckColor = deck == null
        ? null
        : tokens.deckColor(isDeckA: deck == DeckSide.a);
    final gateway = ref.read(engineGatewayProvider);
    final background = isPlaying
        ? deckColor!.withValues(alpha: 0.22)
        : deckColor != null
        ? deckColor.withValues(alpha: 0.1)
        : index.isOdd
        ? tokens.panelRaised.withValues(alpha: 0.35)
        : null;
    final titleColor = isPlaying ? tokens.textPrimary : null;
    final row = Semantics(
      label: [
        track.title,
        if (isPlaying) 'en cours',
        if (isNext) 'à suivre',
      ].join(', '),
      child: GestureDetector(
        onDoubleTap: () => unawaited(
          gateway.loadTrack(
            ref.read(sessionStoreProvider).activeDeck?.other ?? DeckSide.a,
            track.id,
          ),
        ),
        child: Container(
          color: background,
          child: Row(
            children: [
              SizedBox(
                width: _numberWidth,
                child: Center(
                  child: isPlaying || isNext || deck != null
                      ? Icon(
                          isPlaying
                              ? Icons.graphic_eq_rounded
                              : isNext
                              ? Icons.skip_next_rounded
                              : Icons.album_rounded,
                          size: 16,
                          color: deckColor ?? tokens.textSecondary,
                        )
                      : Text(
                          '${index + 1}',
                          style: counterStyle(
                            context,
                            size: 11,
                            color: tokens.textMuted,
                          ),
                        ),
                ),
              ),
              Expanded(
                flex: 5,
                child: Text(
                  track.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodyMedium?.copyWith(
                    color: titleColor,
                    fontWeight: isPlaying ? FontWeight.w700 : null,
                  ),
                ),
              ),
              Expanded(
                flex: 3,
                child: Text(
                  track.artist ?? '—',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall,
                ),
              ),
              Expanded(
                flex: 3,
                child: Text(
                  folderOf(track.path),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(color: tokens.textMuted),
                ),
              ),
              SizedBox(
                width: _formatWidth,
                child: Text(formatOf(track.path), style: textTheme.labelSmall),
              ),
              SizedBox(
                width: _loadWidth,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                  children: [
                    for (final side in DeckSide.values)
                      _LoadButton(
                        side: side,
                        onPressed: () =>
                            unawaited(gateway.loadTrack(side, track.id)),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
    return Draggable<TrackInfo>(
      data: track,
      dragAnchorStrategy: pointerDragAnchorStrategy,
      feedback: _DragChip(track: track),
      childWhenDragging: Opacity(opacity: 0.4, child: row),
      child: row,
    );
  }
}

class _LoadButton extends StatelessWidget {
  const new({required this.side, required this.onPressed});

  final DeckSide side;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final color = tokens.deckColor(isDeckA: side == DeckSide.a);
    return Tooltip(
      message: 'Charger sur la platine ${side.label}',
      child: InkWell(
        onTap: onPressed,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        child: Container(
          width: 24,
          height: 20,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(tokens.radiusSmall),
            border: Border.all(color: color.withValues(alpha: 0.6)),
          ),
          child: Text(
            side.label,
            style: TextStyle(
              color: color,
              fontWeight: FontWeight.w700,
              fontSize: 11,
            ),
          ),
        ),
      ),
    );
  }
}

/// Name of the folder containing [path].
String folderOf(String path) {
  final parts = path.split('/')..removeLast();
  return parts.isEmpty ? '' : parts.last;
}

/// File extension of [path], upper case ("MP3").
String formatOf(String path) {
  final name = path.split('/').last;
  final dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.substring(dot + 1).toUpperCase();
}

/// Label following the pointer while a track is dragged onto a deck.
class _DragChip extends StatelessWidget {
  const new({required this.track});

  final TrackInfo track;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Material(
      color: Colors.transparent,
      child: Container(
        padding: EdgeInsets.symmetric(
          horizontal: tokens.space3,
          vertical: tokens.space2,
        ),
        decoration: BoxDecoration(
          color: tokens.panelRaised,
          borderRadius: BorderRadius.circular(tokens.radiusSmall),
          border: Border.all(color: tokens.textSecondary),
          boxShadow: const [BoxShadow(blurRadius: 12, color: Colors.black54)],
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.music_note_rounded, size: 16, color: tokens.textPrimary),
            SizedBox(width: tokens.space2),
            Text(track.title, style: Theme.of(context).textTheme.bodyMedium),
          ],
        ),
      ),
    );
  }
}
