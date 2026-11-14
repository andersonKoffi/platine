import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';

/// Seconds per turn at 33⅓ rpm.
const double secondsPerTurn = 1.8;

/// Jog wheel: a platter turning with the music, ringed by the track
/// progress, with the remaining time in its centre. Turning it by hand
/// moves the playhead (like nudging a record).
class JogWheel extends StatefulWidget {
  const new({
    required this.position,
    required this.duration,
    required this.end,
    required this.playing,
    required this.accent,
    required this.enabled,
    required this.onSeek,
    required this.label,
    super.key,
  });

  final double position;
  final double duration;

  /// Last audible second (the remaining time counts down to it).
  final double end;
  final bool playing;
  final Color accent;
  final bool enabled;
  final ValueChanged<double> onSeek;
  final String label;

  @override
  State<JogWheel> createState() => _JogWheelState();
}

class _JogWheelState extends State<JogWheel> {
  /// Seconds added by the hand while dragging (applied on release).
  double _dragSeconds = 0;
  double? _lastAngle;

  double _angleOf(Offset local, Size size) {
    final center = size.center(Offset.zero);
    return math.atan2(local.dy - center.dy, local.dx - center.dx);
  }

  void _onPanUpdate(DragUpdateDetails details, Size size) {
    final angle = _angleOf(details.localPosition, size);
    final last = _lastAngle;
    _lastAngle = angle;
    if (last == null) return;
    var delta = angle - last;
    if (delta > math.pi) delta -= 2 * math.pi;
    if (delta < -math.pi) delta += 2 * math.pi;
    setState(() => _dragSeconds += delta / (2 * math.pi) * secondsPerTurn);
  }

  void _onPanEnd() {
    final target = (widget.position + _dragSeconds).clamp(
      0,
      math.max(0, widget.duration),
    );
    if (_dragSeconds != 0) widget.onSeek(target.toDouble());
    setState(() {
      _dragSeconds = 0;
      _lastAngle = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final position = widget.position + _dragSeconds;
    final remaining = math.max(0, widget.end - position).toDouble();
    final progress = widget.duration > 0
        ? (position / widget.duration).clamp(0.0, 1.0)
        : 0.0;
    return LayoutBuilder(
      builder: (context, constraints) {
        final side = math.min(constraints.maxWidth, constraints.maxHeight);
        final size = Size.square(side);
        return Center(
          child: Semantics(
            label: widget.label,
            value: '${formatClock(remaining)} restantes',
            child: GestureDetector(
              onPanStart: widget.enabled
                  ? (details) =>
                        _lastAngle = _angleOf(details.localPosition, size)
                  : null,
              onPanUpdate: widget.enabled
                  ? (details) => _onPanUpdate(details, size)
                  : null,
              onPanEnd: widget.enabled ? (_) => _onPanEnd() : null,
              child: SizedBox.fromSize(
                size: size,
                child: Stack(
                  alignment: Alignment.center,
                  children: [
                    CustomPaint(
                      size: size,
                      painter: _JogPainter(
                        angle: position / secondsPerTurn * 2 * math.pi,
                        progress: progress,
                        accent: widget.accent,
                        active: widget.playing,
                        ring: tokens.inset,
                        platter: tokens.panelRaised,
                        rim: tokens.panelBorder,
                        marker: tokens.textPrimary,
                      ),
                    ),
                    Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          '-${formatPreciseClock(remaining)}',
                          style: counterStyle(context, size: side * 0.1),
                        ),
                        SizedBox(height: tokens.space1),
                        Text(
                          formatPreciseClock(position),
                          style: counterStyle(
                            context,
                            size: side * 0.065,
                            color: tokens.textSecondary,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class _JogPainter extends CustomPainter {
  const new({
    required this.angle,
    required this.progress,
    required this.accent,
    required this.active,
    required this.ring,
    required this.platter,
    required this.rim,
    required this.marker,
  });

  final double angle;
  final double progress;
  final Color accent;
  final bool active;
  final Color ring;
  final Color platter;
  final Color rim;
  final Color marker;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2;
    // Outer ring with the track progress.
    canvas.drawCircle(center, radius, Paint()..color = ring);
    final ringRect = Rect.fromCircle(center: center, radius: radius - 5);
    final stroke = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 5
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      ringRect,
      -math.pi / 2,
      2 * math.pi,
      false,
      stroke..color = rim,
    );
    if (progress > 0) {
      canvas.drawArc(
        ringRect,
        -math.pi / 2,
        2 * math.pi * progress,
        false,
        stroke..color = accent,
      );
    }
    if (active) {
      canvas.drawCircle(
        center,
        radius - 5,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = 8
          ..color = accent.withValues(alpha: 0.18)
          ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 6),
      );
    }
    // Platter.
    final platterRadius = radius - 14;
    final platterRect = Rect.fromCircle(center: center, radius: platterRadius);
    canvas
      ..drawCircle(
        center,
        platterRadius,
        Paint()
          ..shader = RadialGradient(
            colors: [
              Color.lerp(platter, Colors.black, 0.35)!,
              platter,
              Color.lerp(platter, Colors.white, 0.08)!,
              Color.lerp(platter, Colors.black, 0.2)!,
            ],
            stops: const [0, 0.7, 0.93, 1],
          ).createShader(platterRect),
      )
      ..save()
      ..translate(center.dx, center.dy)
      ..rotate(angle);
    // Rotating grip ticks and the position marker.
    final tickPaint = Paint()
      ..color = Colors.white.withValues(alpha: 0.07)
      ..strokeWidth = 2;
    for (var i = 0; i < 48; i++) {
      final a = i / 48 * 2 * math.pi;
      final direction = Offset(math.cos(a), math.sin(a));
      canvas.drawLine(
        direction * (platterRadius - 10),
        direction * (platterRadius - 3),
        tickPaint,
      );
    }
    canvas
      ..drawLine(
        Offset(0, -platterRadius + 3),
        Offset(0, -platterRadius * 0.62),
        Paint()
          ..color = marker
          ..strokeWidth = 4
          ..strokeCap = StrokeCap.round,
      )
      ..restore()
      // Centre display.
      ..drawCircle(
        center,
        platterRadius * 0.55,
        Paint()..color = Color.lerp(ring, Colors.black, 0.2)!,
      )
      ..drawCircle(
        center,
        platterRadius * 0.55,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1.5
          ..color = accent.withValues(alpha: active ? 0.7 : 0.25),
      );
  }

  @override
  bool shouldRepaint(_JogPainter old) =>
      old.angle != angle ||
      old.progress != progress ||
      old.active != active ||
      old.accent != accent;
}
