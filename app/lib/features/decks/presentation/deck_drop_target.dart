import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';

/// Accepts a track dragged from the library and loads it on [side].
///
/// A deck that is playing refuses the drop: replacing the music on air by
/// accident would stop it in the middle of a show.
class DeckDropTarget extends ConsumerWidget {
  const new({required this.side, required this.child, super.key});

  final DeckSide side;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final playing = ref.watch(
      sessionStoreProvider.select((s) => s.deck(side).playing),
    );
    final accent = tokens.deckColor(isDeckA: side == DeckSide.a);
    return DragTarget<TrackInfo>(
      onWillAcceptWithDetails: (_) => !playing,
      onAcceptWithDetails: (details) => unawaited(
        ref.read(engineGatewayProvider).loadTrack(side, details.data.id),
      ),
      builder: (context, candidates, rejected) {
        final hovering = candidates.isNotEmpty || rejected.isNotEmpty;
        if (!hovering) return child;
        final refused = rejected.isNotEmpty;
        final color = refused ? tokens.danger : accent;
        return Stack(
          fit: StackFit.passthrough,
          children: [
            child,
            Positioned.fill(
              child: IgnorePointer(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.16),
                    borderRadius: BorderRadius.circular(tokens.radiusMedium),
                    border: Border.all(color: color, width: 2),
                  ),
                  child: Center(
                    child: Text(
                      refused
                          ? 'Platine ${side.label} en lecture : '
                                'mets-la en pause pour la remplacer'
                          : 'Charger sur la platine ${side.label}',
                      textAlign: TextAlign.center,
                      style: Theme.of(
                        context,
                      ).textTheme.titleMedium?.copyWith(color: color),
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}
