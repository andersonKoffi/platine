import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/presentation/deck_drop_target.dart';
import 'package:platine/features/decks/presentation/jog_wheel.dart';
import 'package:platine/features/decks/presentation/playback_position.dart';
import 'package:platine/features/decks/presentation/waveform_view.dart';

/// One deck: track info, overview waveform, jog wheel and transport.
/// Deck B is the mirror image of deck A.
class DeckView extends ConsumerWidget {
  const new({required this.side, super.key});

  final DeckSide side;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final deck = ref.watch(sessionStoreProvider.select((s) => s.deck(side)));
    final track = ref.watch(
      sessionStoreProvider.select((s) => s.track(s.deck(side).trackId)),
    );
    final role = ref.watch(
      sessionStoreProvider.select((s) => _roleOf(s, side)),
    );
    final accent = tokens.deckColor(isDeckA: side == DeckSide.a);
    final gateway = ref.read(engineGatewayProvider);

    final transport = Column(
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        RoundTransportButton(
          icon: Icons.first_page_rounded,
          text: 'CUE',
          label: 'Cue platine ${side.label} (début audible)',
          color: accent,
          size: 52,
          onPressed: deck.hasTrack
              ? () => unawaited(gateway.seek(side, deck.start))
              : null,
        ),
        SizedBox(height: tokens.space3),
        RoundTransportButton(
          icon: deck.playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
          label: deck.playing
              ? 'Pause platine ${side.label}'
              : 'Lecture platine ${side.label}',
          color: accent,
          size: 64,
          active: deck.playing,
          onPressed: deck.hasTrack
              ? () => unawaited(gateway.togglePlay(side))
              : null,
        ),
      ],
    );

    final jog = Expanded(
      child: PlaybackPosition(
        position: deck.position,
        updatedAt: deck.positionUpdatedAt,
        playing: deck.playing,
        builder: (context, position) => JogWheel(
          position: position,
          duration: deck.duration,
          end: deck.end,
          playing: deck.playing,
          accent: accent,
          enabled: deck.hasTrack,
          label: 'Jog platine ${side.label}',
          onSeek: (target) => unawaited(gateway.seek(side, target)),
        ),
      ),
    );

    final mirrored = side == DeckSide.b;
    return DeckDropTarget(
      side: side,
      child: PanelBox(
      accent: deck.playing ? accent : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _DeckHeader(
            side: side,
            track: track,
            deck: deck,
            accent: accent,
            role: role,
          ),
          SizedBox(height: tokens.space2),
          DeckOverview(side: side, deck: deck, accent: accent),
          SizedBox(height: tokens.space3),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: mirrored
                  ? [jog, SizedBox(width: tokens.space3), transport]
                  : [transport, SizedBox(width: tokens.space3), jog],
            ),
          ),
        ],
      ),
      ),
    );
  }

  static _DeckRole _roleOf(SessionState state, DeckSide side) {
    final transition = state.transition;
    if (transition != null) {
      return transition.to == side ? _DeckRole.incoming : _DeckRole.outgoing;
    }
    if (state.activeDeck == side) return _DeckRole.onAir;
    if (state.deck(side).trackId != null) return _DeckRole.next;
    return _DeckRole.idle;
  }
}

enum _DeckRole {
  idle(''),
  onAir('ON AIR'),
  next('SUIVANT'),
  incoming('ENTRÉE'),
  outgoing('SORTIE');

  new(this.label);

  final String label;
}

class _DeckHeader extends StatelessWidget {
  const new({
    required this.side,
    required this.track,
    required this.deck,
    required this.accent,
    required this.role,
  });

  final DeckSide side;
  final TrackInfo? track;
  final DeckState deck;
  final Color accent;
  final _DeckRole role;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final textTheme = Theme.of(context).textTheme;
    final title = switch ((track, deck.loading)) {
      (_, true) => 'Chargement…',
      (final TrackInfo t, false) => t.title,
      (null, false) => 'Platine vide',
    };
    final badge = Container(
      width: 34,
      height: 34,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: accent,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
      ),
      child: Text(
        side.label,
        style: textTheme.titleMedium?.copyWith(
          color: tokens.background,
          fontSize: 20,
        ),
      ),
    );
    final info = Expanded(
      child: Column(
        crossAxisAlignment: side == DeckSide.a
            ? CrossAxisAlignment.start
            : CrossAxisAlignment.end,
        children: [
          Text(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: textTheme.titleMedium,
          ),
          Text(
            track?.artist ?? (track == null ? ' ' : 'Artiste inconnu'),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: textTheme.bodySmall,
          ),
        ],
      ),
    );
    final bpm = Container(
      padding: EdgeInsets.symmetric(
        horizontal: tokens.space2,
        vertical: tokens.space1,
      ),
      decoration: BoxDecoration(
        color: tokens.inset,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        border: Border.all(color: tokens.panelBorder),
      ),
      child: Column(
        children: [
          Text(
            '---.-',
            style: counterStyle(context, size: 18, color: accent),
            semanticsLabel: 'BPM inconnu',
          ),
          const ControlLabel('BPM'),
        ],
      ),
    );
    final roleChip = AnimatedOpacity(
      opacity: role == _DeckRole.idle ? 0 : 1,
      duration: tokens.durationMedium,
      child: Container(
        padding: EdgeInsets.symmetric(horizontal: tokens.space2, vertical: 2),
        decoration: BoxDecoration(
          color: role == _DeckRole.onAir || role == _DeckRole.incoming
              ? accent.withValues(alpha: 0.2)
              : tokens.panelRaised,
          borderRadius: BorderRadius.circular(tokens.radiusSmall),
          border: Border.all(
            color: role == _DeckRole.onAir ? accent : tokens.panelBorder,
          ),
        ),
        child: Text(
          role.label,
          style: textTheme.labelSmall?.copyWith(
            color: role == _DeckRole.onAir ? accent : tokens.textSecondary,
          ),
        ),
      ),
    );
    final gap = SizedBox(width: tokens.space3);
    final children = side == DeckSide.a
        ? [badge, gap, info, gap, roleChip, gap, bpm]
        : [bpm, gap, roleChip, gap, info, gap, badge];
    return Row(children: children);
  }
}

/// Overview of the whole track. Clicking it seeks.
class DeckOverview extends ConsumerWidget {
  const new({
    required this.side,
    required this.deck,
    required this.accent,
    super.key,
  });

  final DeckSide side;
  final DeckState deck;
  final Color accent;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final duration = deck.duration;
    return Semantics(
      label: 'Position de la platine ${side.label}',
      value: '${formatClock(deck.position)} sur ${formatClock(duration)}',
      child: LayoutBuilder(
        builder: (context, constraints) => GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: deck.hasTrack && duration > 0
              ? (details) {
                  final target =
                      (details.localPosition.dx / constraints.maxWidth)
                          .clamp(0, 1)
                          .toDouble();
                  unawaited(
                    ref
                        .read(engineGatewayProvider)
                        .seek(side, target * duration),
                  );
                }
              : null,
          child: CustomPaint(
            size: Size(constraints.maxWidth, 36),
            painter: OverviewWaveformPainter(
              data: deck.waveform,
              position: deck.position,
              duration: duration,
              start: deck.start,
              end: deck.end,
              palette: WaveformPalette.fromTokens(tokens, accent),
            ),
          ),
        ),
      ),
    );
  }
}

/// Large scrolling waveform of a deck, for the top of the screen.
class DeckWaveform extends ConsumerWidget {
  const new({required this.side, super.key});

  final DeckSide side;

  /// Seconds shown across the whole width.
  static const double secondsVisible = 12;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final deck = ref.watch(sessionStoreProvider.select((s) => s.deck(side)));
    final accent = tokens.deckColor(isDeckA: side == DeckSide.a);
    final palette = WaveformPalette.fromTokens(tokens, accent);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          width: 28,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: deck.playing ? accent : tokens.panelRaised,
            borderRadius: BorderRadius.horizontal(
              left: Radius.circular(tokens.radiusSmall),
            ),
          ),
          child: Text(
            side.label,
            style: Theme.of(context).textTheme.titleMedium
                ?.copyWith(color: deck.playing ? tokens.background : accent),
          ),
        ),
        Expanded(
          child: ClipRRect(
            borderRadius: BorderRadius.horizontal(
              right: Radius.circular(tokens.radiusSmall),
            ),
            child: RepaintBoundary(
              child: PlaybackPosition(
                position: deck.position,
                updatedAt: deck.positionUpdatedAt,
                playing: deck.playing,
                builder: (context, position) => CustomPaint(
                  painter: ScrollingWaveformPainter(
                    data: deck.waveform,
                    position: position,
                    palette: palette,
                    secondsVisible: secondsVisible,
                    start: deck.start,
                    end: deck.end,
                  ),
                  child: const SizedBox.expand(),
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}
