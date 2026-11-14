import 'dart:math' as math;

import 'package:flutter/scheduler.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';

/// Seconds per turn at 33⅓ rpm.
const double secondsPerTurn = 1.8;

/// A spinning record whose angle follows the real playback position.
///
/// Between two position reports (~30 Hz) the position is extrapolated on
/// every frame, so the rotation is smooth and stops exactly with the music.
class Vinyl extends StatefulWidget {
  const new({
    required this.title,
    required this.position,
    required this.updatedAt,
    required this.playing,
    required this.accent,
    required this.level,
    super.key,
  });

  /// Seeds the generated cover.
  final String? title;
  final double position;
  final DateTime? updatedAt;
  final bool playing;
  final Color accent;

  /// Peak level, drives the glow.
  final double level;

  @override
  State<Vinyl> createState() => _VinylState();
}

class _VinylState extends State<Vinyl> with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => setState(() {}));

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _syncTicker();
  }

  @override
  void didUpdateWidget(Vinyl oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncTicker();
  }

  void _syncTicker() {
    final animate = widget.playing && !context.reduceMotion;
    if (animate && !_ticker.isActive) {
      _ticker.start();
    } else if (!animate && _ticker.isActive) {
      _ticker.stop();
    }
  }

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  double get _estimatedPosition {
    final updatedAt = widget.updatedAt;
    if (!widget.playing || updatedAt == null) return widget.position;
    final elapsed = DateTime.now().difference(updatedAt).inMicroseconds / 1e6;
    return widget.position + elapsed.clamp(0, 0.5);
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final angle = _estimatedPosition / secondsPerTurn * 2 * math.pi;
    return Semantics(
      label: widget.playing ? 'Platine en lecture' : 'Platine à l’arrêt',
      child: RepaintBoundary(
        child: AspectRatio(
          aspectRatio: 1,
          child: CustomPaint(
            painter: _VinylPainter(
              angle: angle,
              accent: widget.accent,
              glow: levelToMeter(widget.level),
              cover: coverGradient(widget.title, tokens),
              groove: tokens.glassBorder,
              disc: tokens.surface,
            ),
          ),
        ),
      ),
    );
  }
}

/// Deterministic two-colour gradient derived from the title's hash.
List<Color> coverGradient(String? title, PlatineTokens tokens) {
  if (title == null || title.isEmpty) {
    return [tokens.neonViolet, tokens.neonMagenta];
  }
  final hash = title.codeUnits.fold<int>(17, (h, c) => (h * 31 + c) & 0xFFFFFF);
  final hue = (hash % 360).toDouble();
  return [
    HSLColor.fromAHSL(1, hue, 0.85, 0.55).toColor(),
    HSLColor.fromAHSL(1, (hue + 70) % 360, 0.9, 0.45).toColor(),
  ];
}

class _VinylPainter extends CustomPainter {
  const new({
    required this.angle,
    required this.accent,
    required this.glow,
    required this.cover,
    required this.groove,
    required this.disc,
  });

  final double angle;
  final Color accent;
  final double glow;
  final List<Color> cover;
  final Color groove;
  final Color disc;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2;
    final groovePaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = groove;
    final labelRect = Rect.fromCircle(
      center: Offset.zero,
      radius: radius * 0.36,
    );

    canvas
      // Neon halo pulsing with the level.
      ..drawCircle(
        center,
        radius * 0.97,
        Paint()
          ..color = accent.withValues(alpha: 0.15 + 0.55 * glow)
          ..maskFilter = MaskFilter.blur(BlurStyle.outer, 6 + 18 * glow),
      )
      ..drawCircle(center, radius * 0.95, Paint()..color = disc)
      ..save()
      ..translate(center.dx, center.dy)
      ..rotate(angle);
    for (var r = radius * 0.42; r < radius * 0.93; r += radius * 0.035) {
      canvas.drawCircle(Offset.zero, r, groovePaint);
    }
    canvas
      // Light reflection, rotating with the disc so the motion is visible.
      ..drawArc(
        Rect.fromCircle(center: Offset.zero, radius: radius * 0.8),
        -0.4,
        0.8,
        false,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = radius * 0.25
          ..color = Colors.white.withValues(alpha: 0.05),
      )
      ..drawCircle(
        Offset.zero,
        radius * 0.36,
        Paint()
          ..shader = SweepGradient(colors: [...cover, cover.first])
              .createShader(labelRect),
      )
      ..drawCircle(
        Offset(radius * 0.22, 0),
        radius * 0.03,
        Paint()..color = Colors.white.withValues(alpha: 0.8),
      )
      ..restore()
      ..drawCircle(center, radius * 0.035, Paint()..color = disc)
      ..drawCircle(
        center,
        radius * 0.95,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2
          ..color = accent.withValues(alpha: 0.7),
      );
  }

  @override
  bool shouldRepaint(_VinylPainter old) =>
      old.angle != angle || old.glow != glow || old.cover != cover;
}
