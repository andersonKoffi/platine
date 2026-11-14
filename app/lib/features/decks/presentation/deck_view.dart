import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/application/deck_controls.dart';
import 'package:platine/features/decks/presentation/vinyl.dart';

/// One deck: record, title, progress, transport, gain and isolator EQ.
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
    final isActive = ref.watch(
      sessionStoreProvider.select((s) => s.activeDeck == side),
    );
    final accent = tokens.deckColor(isDeckA: side == DeckSide.a);

    return GlassPanel(
      glow: isActive && deck.playing ? accent : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _DeckHeader(side: side, track: track, deck: deck, accent: accent),
          SizedBox(height: tokens.space3),
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: Center(
                    child: Vinyl(
                      title: track?.title,
                      position: deck.position,
                      updatedAt: deck.positionUpdatedAt,
                      playing: deck.playing,
                      accent: accent,
                      level: deck.level,
                    ),
                  ),
                ),
                SizedBox(width: tokens.space3),
                _DeckMixer(side: side, level: deck.level, accent: accent),
              ],
            ),
          ),
          SizedBox(height: tokens.space3),
          DeckProgress(side: side, deck: deck, accent: accent),
          SizedBox(height: tokens.space3),
          _Transport(side: side, deck: deck, accent: accent),
        ],
      ),
    );
  }
}

class _DeckHeader extends StatelessWidget {
  const new({
    required this.side,
    required this.track,
    required this.deck,
    required this.accent,
  });

  final DeckSide side;
  final TrackInfo? track;
  final DeckState deck;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final textTheme = Theme.of(context).textTheme;
    final title = switch ((track, deck.loading)) {
      (_, true) => 'Chargement…',
      (final TrackInfo t, false) => t.title,
      (null, false) => 'Platine vide',
    };
    return Row(
      children: [
        Container(
          width: 36,
          height: 36,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(color: accent, width: 2),
          ),
          child: Text(
            side.label,
            style: textTheme.titleMedium?.copyWith(color: accent),
          ),
        ),
        SizedBox(width: tokens.space3),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
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
                style: TextStyle(color: tokens.textSecondary),
              ),
            ],
          ),
        ),
        Text(
          '— BPM',
          style: counterStyle(context, size: 14),
          semanticsLabel: 'BPM inconnu',
        ),
      ],
    );
  }
}

/// Progress bar of the audible part, with elapsed and remaining times.
/// Clicking it seeks.
class DeckProgress extends ConsumerWidget {
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
    final fraction = duration > 0 ? (deck.position / duration).clamp(0, 1) : 0;
    return Column(
      children: [
        Semantics(
          label: 'Position de la platine ${side.label}',
          value: '${formatClock(deck.position)} sur ${formatClock(duration)}',
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: deck.hasTrack && duration > 0
                ? (details) {
                    final box = context.findRenderObject() as RenderBox?;
                    final width = box?.size.width ?? 1;
                    final target = (details.localPosition.dx / width)
                        .clamp(0, 1)
                        .toDouble();
                    unawaited(
                      ref
                          .read(engineGatewayProvider)
                          .seek(side, target * duration),
                    );
                  }
                : null,
            child: SizedBox(
              height: tokens.space5,
              child: CustomPaint(
                painter: _ProgressPainter(
                  played: fraction.toDouble(),
                  start: duration > 0 ? deck.start / duration : 0,
                  end: duration > 0 ? deck.end / duration : 1,
                  accent: accent,
                  track: tokens.glass,
                  marker: tokens.textMuted,
                  radius: tokens.radiusSmall,
                ),
              ),
            ),
          ),
        ),
        SizedBox(height: tokens.space1),
        Row(
          children: [
            Text(formatClock(deck.position), style: counterStyle(context)),
            const Spacer(),
            Text(
              '-${formatClock(deck.remaining)}',
              style: counterStyle(context).copyWith(color: accent),
            ),
          ],
        ),
      ],
    );
  }
}

class _ProgressPainter extends CustomPainter {
  const new({
    required this.played,
    required this.start,
    required this.end,
    required this.accent,
    required this.track,
    required this.marker,
    required this.radius,
  });

  final double played;
  final double start;
  final double end;
  final Color accent;
  final Color track;
  final Color marker;
  final double radius;

  @override
  void paint(Canvas canvas, Size size) {
    final bar = Rect.fromLTWH(0, size.height / 3, size.width, size.height / 3);
    final markerPaint = Paint()
      ..color = marker
      ..strokeWidth = 2;
    canvas
      ..drawRRect(
        RRect.fromRectAndRadius(bar, Radius.circular(radius)),
        Paint()..color = track,
      )
      ..drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTWH(bar.left, bar.top, bar.width * played, bar.height),
          Radius.circular(radius),
        ),
        Paint()..color = accent,
      )
      // Audible part boundaries (silences are trimmed by the auto-mix).
      ..drawLine(
        Offset(size.width * start, 0),
        Offset(size.width * start, size.height),
        markerPaint,
      )
      ..drawLine(
        Offset(size.width * end, 0),
        Offset(size.width * end, size.height),
        markerPaint,
      );
  }

  @override
  bool shouldRepaint(_ProgressPainter old) =>
      old.played != played || old.start != start || old.end != end;
}

class _Transport extends ConsumerWidget {
  const new({required this.side, required this.deck, required this.accent});

  final DeckSide side;
  final DeckState deck;
  final Color accent;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final gateway = ref.read(engineGatewayProvider);
    final enabled = deck.hasTrack;
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        NeonIconButton(
          icon: Icons.first_page_rounded,
          label: 'Cue platine ${side.label} (début audible)',
          color: accent,
          size: 48,
          onPressed: enabled
              ? () => unawaited(gateway.seek(side, deck.start))
              : null,
        ),
        SizedBox(width: tokens.space4),
        NeonIconButton(
          icon: deck.playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
          label: deck.playing
              ? 'Pause platine ${side.label}'
              : 'Lecture platine ${side.label}',
          color: accent,
          size: 72,
          active: deck.playing,
          onPressed: enabled
              ? () => unawaited(gateway.togglePlay(side))
              : null,
        ),
      ],
    );
  }
}

/// Gain, three isolator bands with kill buttons, and the deck's VU meter.
class _DeckMixer extends ConsumerWidget {
  const new({required this.side, required this.level, required this.accent});

  final DeckSide side;
  final double level;
  final Color accent;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final controls = ref.watch(deckControlsControllerProvider(side));
    final controller = ref.read(deckControlsControllerProvider(side).notifier);
    return Row(
      children: [
        _VerticalFader(
          label: 'Gain',
          semantics: 'Gain platine ${side.label}',
          value: controls.gain,
          color: accent,
          onChanged: controller.setGain,
        ),
        for (final band in EqBandType.values.reversed)
          _VerticalFader(
            label: _bandLabel(band),
            semantics: 'Égaliseur ${_bandLabel(band)} platine ${side.label}',
            value: controls.band(band),
            color: tokens.neonViolet,
            onChanged: (value) => controller.setBand(band, value),
            killed: controls.band(band) == 0,
            onKill: () => controller.toggleKill(band),
          ),
        SizedBox(width: tokens.space2),
        VuMeter(level: level, label: 'Niveau platine ${side.label}'),
      ],
    );
  }

  static String _bandLabel(EqBandType band) => switch (band) {
    EqBandType.low => 'Bas',
    EqBandType.mid => 'Méd',
    EqBandType.high => 'Aigu',
  };
}

class _VerticalFader extends StatelessWidget {
  const new({
    required this.label,
    required this.semantics,
    required this.value,
    required this.color,
    required this.onChanged,
    this.killed = false,
    this.onKill,
  });

  final String label;
  final String semantics;
  final double value;
  final Color color;
  final ValueChanged<double> onChanged;
  final bool killed;
  final VoidCallback? onKill;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return SizedBox(
      width: tokens.minTouchTarget,
      child: Column(
        children: [
          Expanded(
            child: RotatedBox(
              quarterTurns: 3,
              child: Semantics(
                label: semantics,
                child: SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    activeTrackColor: color,
                  ),
                  child: Slider(
                    value: value,
                    max: DeckControls.maxLevel,
                    onChanged: onChanged,
                  ),
                ),
              ),
            ),
          ),
          if (onKill != null)
            Tooltip(
              message: killed ? 'Rétablir $label' : 'Couper $label',
              child: InkWell(
                onTap: onKill,
                borderRadius: BorderRadius.circular(tokens.radiusSmall),
                child: Padding(
                  padding: EdgeInsets.all(tokens.space1),
                  child: Text(
                    label,
                    style: TextStyle(
                      fontSize: 12,
                      color: killed ? tokens.danger : tokens.textSecondary,
                      fontWeight: killed ? FontWeight.w700 : FontWeight.w400,
                    ),
                  ),
                ),
              ),
            )
          else
            Padding(
              padding: EdgeInsets.all(tokens.space1),
              child: Text(
                label,
                style: TextStyle(fontSize: 12, color: tokens.textSecondary),
              ),
            ),
        ],
      ),
    );
  }
}
