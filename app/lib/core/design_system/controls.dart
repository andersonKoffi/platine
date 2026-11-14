import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Rotary knob. [position] is `0..1`; drag up/down, scroll, or use the
/// arrow keys to turn it; double-click resets it to [defaultPosition].
class Knob extends StatelessWidget {
  const new({
    required this.position,
    required this.onChanged,
    required this.label,
    super.key,
    this.defaultPosition = 0.5,
    this.bipolar = true,
    this.color,
    this.size = 40,
    this.valueText,
    this.onDoubleTap,
  });

  final double position;
  final ValueChanged<double> onChanged;

  /// Accessible name.
  final String label;
  final double defaultPosition;

  /// Draws the value arc from the top (EQ) instead of from the minimum.
  final bool bipolar;
  final Color? color;
  final double size;

  /// Spoken value (defaults to a percentage).
  final String? valueText;

  /// Overrides the reset on double-click (e.g. EQ kill).
  final VoidCallback? onDoubleTap;

  static const double _step = 0.05;

  void _nudge(double delta) => onChanged((position + delta).clamp(0, 1));

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final accent = color ?? tokens.deckA;
    return Semantics(
      slider: true,
      label: label,
      value: valueText ?? '${(position * 100).round()} %',
      increasedValue: '${((position + _step).clamp(0, 1) * 100).round()} %',
      decreasedValue: '${((position - _step).clamp(0, 1) * 100).round()} %',
      onIncrease: () => _nudge(_step),
      onDecrease: () => _nudge(-_step),
      child: FocusableActionDetector(
        mouseCursor: SystemMouseCursors.resizeUpDown,
        shortcuts: const {
          SingleActivator(LogicalKeyboardKey.arrowUp): _KnobIntent(1),
          SingleActivator(LogicalKeyboardKey.arrowDown): _KnobIntent(-1),
        },
        actions: {
          _KnobIntent: CallbackAction<_KnobIntent>(
            onInvoke: (intent) => _nudge(intent.direction * _step),
          ),
        },
        child: Listener(
          onPointerSignal: (event) {
            if (event is PointerScrollEvent) {
              _nudge(-event.scrollDelta.dy / 400);
            }
          },
          child: GestureDetector(
            onDoubleTap: onDoubleTap ?? () => onChanged(defaultPosition),
            onVerticalDragUpdate: (details) =>
                _nudge(-details.delta.dy / 150),
            child: Tooltip(
              message: label,
              child: CustomPaint(
                size: Size.square(size),
                painter: _KnobPainter(
                  position: position.clamp(0, 1),
                  bipolar: bipolar,
                  accent: accent,
                  track: tokens.panelBorder,
                  cap: tokens.panelRaised,
                  pointer: tokens.textPrimary,
                  shadow: tokens.background,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _KnobIntent extends Intent {
  const new(this.direction);

  final int direction;
}

class _KnobPainter extends CustomPainter {
  const new({
    required this.position,
    required this.bipolar,
    required this.accent,
    required this.track,
    required this.cap,
    required this.pointer,
    required this.shadow,
  });

  static const double _sweep = 1.5 * math.pi; // 270°
  static const double _start = 0.75 * math.pi; // from bottom-left

  final double position;
  final bool bipolar;
  final Color accent;
  final Color track;
  final Color cap;
  final Color pointer;
  final Color shadow;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2;
    final arcRect = Rect.fromCircle(center: center, radius: radius - 2);
    final arcPaint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 3
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(arcRect, _start, _sweep, false, arcPaint..color = track);
    final from = bipolar ? 0.5 : 0.0;
    final a = _start + _sweep * math.min(from, position);
    final b = _start + _sweep * math.max(from, position);
    if (b - a > 0.001) {
      canvas.drawArc(arcRect, a, b - a, false, arcPaint..color = accent);
    }
    final capRadius = radius - 7;
    canvas
      ..drawCircle(
        center.translate(0, 1.5),
        capRadius,
        Paint()..color = shadow.withValues(alpha: 0.8),
      )
      ..drawCircle(
        center,
        capRadius,
        Paint()
          ..shader = RadialGradient(
            center: const Alignment(-0.3, -0.4),
            colors: [Color.lerp(cap, Colors.white, 0.12)!, cap],
          ).createShader(Rect.fromCircle(center: center, radius: capRadius)),
      );
    final angle = _start + _sweep * position;
    final direction = Offset(math.cos(angle), math.sin(angle));
    canvas.drawLine(
      center + direction * (capRadius * 0.25),
      center + direction * (capRadius - 2),
      Paint()
        ..color = pointer
        ..strokeWidth = 2.5
        ..strokeCap = StrokeCap.round,
    );
  }

  @override
  bool shouldRepaint(_KnobPainter old) =>
      old.position != position || old.accent != accent;
}

/// Vertical fader (channel volume). [position] is `0..1`, bottom to top.
class VerticalFader extends StatelessWidget {
  const new({
    required this.position,
    required this.onChanged,
    required this.label,
    super.key,
    this.color,
    this.width = 34,
    this.defaultPosition = 0.8,
  });

  final double position;
  final ValueChanged<double> onChanged;
  final String label;
  final Color? color;
  final double width;
  final double defaultPosition;

  static const double _capHeight = 26;
  static const double _step = 0.05;

  void _nudge(double delta) => onChanged((position + delta).clamp(0, 1));

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final accent = color ?? tokens.deckA;
    return Semantics(
      slider: true,
      label: label,
      value: '${(position * 100).round()} %',
      increasedValue: '${((position + _step).clamp(0, 1) * 100).round()} %',
      decreasedValue: '${((position - _step).clamp(0, 1) * 100).round()} %',
      onIncrease: () => _nudge(_step),
      onDecrease: () => _nudge(-_step),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final travel = math.max(1, constraints.maxHeight - _capHeight);
          void jump(double dy) =>
              onChanged((1 - (dy - _capHeight / 2) / travel).clamp(0, 1));
          return Tooltip(
            message: label,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onDoubleTap: () => onChanged(defaultPosition),
              onTapDown: (details) => jump(details.localPosition.dy),
              onVerticalDragUpdate: (details) =>
                  _nudge(-details.delta.dy / travel),
              child: CustomPaint(
                size: Size(width, constraints.maxHeight),
                painter: _FaderPainter(
                  position: position.clamp(0, 1),
                  capHeight: _capHeight,
                  slot: tokens.inset,
                  tick: tokens.panelBorder,
                  cap: tokens.panelRaised,
                  accent: accent,
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

class _FaderPainter extends CustomPainter {
  const new({
    required this.position,
    required this.capHeight,
    required this.slot,
    required this.tick,
    required this.cap,
    required this.accent,
  });

  final double position;
  final double capHeight;
  final Color slot;
  final Color tick;
  final Color cap;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    final midX = size.width / 2;
    final travel = size.height - capHeight;
    for (var i = 0; i <= 10; i++) {
      final y = capHeight / 2 + travel * i / 10;
      final half = i % 5 == 0 ? size.width * 0.45 : size.width * 0.3;
      final paint = Paint()
        ..color = tick
        ..strokeWidth = 1;
      canvas
        ..drawLine(Offset(midX - half, y), Offset(midX - 8, y), paint)
        ..drawLine(Offset(midX + 8, y), Offset(midX + half, y), paint);
    }
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        Rect.fromCenter(
          center: Offset(midX, size.height / 2),
          width: 5,
          height: travel,
        ),
        const Radius.circular(3),
      ),
      Paint()..color = slot,
    );
    final top = travel * (1 - position);
    final capRect = Rect.fromLTWH(
      midX - size.width * 0.42,
      top,
      size.width * 0.84,
      capHeight,
    );
    canvas
      ..drawRRect(
        RRect.fromRectAndRadius(
          capRect.translate(0, 2),
          const Radius.circular(3),
        ),
        Paint()..color = Colors.black54,
      )
      ..drawRRect(
        RRect.fromRectAndRadius(capRect, const Radius.circular(3)),
        Paint()
          ..shader = LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              Color.lerp(cap, Colors.white, 0.18)!,
              cap,
              Color.lerp(cap, Colors.black, 0.3)!,
            ],
          ).createShader(capRect),
      )
      ..drawLine(
        Offset(capRect.left + 3, capRect.center.dy),
        Offset(capRect.right - 3, capRect.center.dy),
        Paint()
          ..color = accent
          ..strokeWidth = 2,
      );
  }

  @override
  bool shouldRepaint(_FaderPainter old) =>
      old.position != position || old.accent != accent;
}

/// Horizontal crossfader, A on the left and B on the right.
class HorizontalFader extends StatelessWidget {
  const new({
    required this.position,
    required this.onChanged,
    required this.label,
    required this.leftColor,
    required this.rightColor,
    super.key,
    this.height = 40,
  });

  final double position;
  final ValueChanged<double> onChanged;
  final String label;
  final Color leftColor;
  final Color rightColor;
  final double height;

  static const double _capWidth = 22;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Semantics(
      slider: true,
      label: label,
      value: '${(position * 100).round()} %',
      increasedValue: '${((position + 0.05).clamp(0, 1) * 100).round()} %',
      decreasedValue: '${((position - 0.05).clamp(0, 1) * 100).round()} %',
      onIncrease: () => onChanged((position + 0.05).clamp(0, 1)),
      onDecrease: () => onChanged((position - 0.05).clamp(0, 1)),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final travel = math.max(1, constraints.maxWidth - _capWidth);
          void jump(double dx) =>
              onChanged(((dx - _capWidth / 2) / travel).clamp(0, 1));
          return GestureDetector(
            behavior: HitTestBehavior.opaque,
            onDoubleTap: () => onChanged(0.5),
            onTapDown: (details) => jump(details.localPosition.dx),
            onHorizontalDragUpdate: (details) =>
                onChanged((position + details.delta.dx / travel).clamp(0, 1)),
            child: CustomPaint(
              size: Size(constraints.maxWidth, height),
              painter: _CrossfaderPainter(
                position: position.clamp(0, 1),
                capWidth: _capWidth,
                slot: tokens.inset,
                tick: tokens.panelBorder,
                cap: tokens.panelRaised,
                left: leftColor,
                right: rightColor,
              ),
            ),
          );
        },
      ),
    );
  }
}

class _CrossfaderPainter extends CustomPainter {
  const new({
    required this.position,
    required this.capWidth,
    required this.slot,
    required this.tick,
    required this.cap,
    required this.left,
    required this.right,
  });

  final double position;
  final double capWidth;
  final Color slot;
  final Color tick;
  final Color cap;
  final Color left;
  final Color right;

  @override
  void paint(Canvas canvas, Size size) {
    final midY = size.height / 2;
    final travel = size.width - capWidth;
    for (var i = 0; i <= 8; i++) {
      final x = capWidth / 2 + travel * i / 8;
      canvas.drawLine(
        Offset(x, midY - size.height * 0.42),
        Offset(x, midY - 7),
        Paint()
          ..color = i == 4 ? left.withValues(alpha: 0.5) : tick
          ..strokeWidth = 1,
      );
    }
    final slotRect = Rect.fromLTWH(capWidth / 2, midY - 3, travel, 6);
    canvas.drawRRect(
      RRect.fromRectAndRadius(slotRect, const Radius.circular(3)),
      Paint()
        ..shader = LinearGradient(
          colors: [
            left.withValues(alpha: 0.5),
            slot,
            right.withValues(alpha: 0.5),
          ],
        ).createShader(slotRect),
    );
    final x = travel * position;
    final capRect = Rect.fromLTWH(x, midY - size.height * 0.4, capWidth, size.height * 0.8);
    canvas
      ..drawRRect(
        RRect.fromRectAndRadius(capRect.translate(0, 2), const Radius.circular(3)),
        Paint()..color = Colors.black54,
      )
      ..drawRRect(
        RRect.fromRectAndRadius(capRect, const Radius.circular(3)),
        Paint()
          ..shader = LinearGradient(
            colors: [
              Color.lerp(cap, Colors.white, 0.18)!,
              cap,
              Color.lerp(cap, Colors.black, 0.3)!,
            ],
          ).createShader(capRect),
      )
      ..drawLine(
        Offset(capRect.center.dx, capRect.top + 3),
        Offset(capRect.center.dx, capRect.bottom - 3),
        Paint()
          ..color = Color.lerp(left, right, position)!
          ..strokeWidth = 2,
      );
  }

  @override
  bool shouldRepaint(_CrossfaderPainter old) => old.position != position;
}
