import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/theme.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Frosted glass panel used for every section of the screen.
class GlassPanel extends StatelessWidget {
  const new({required this.child, super.key, this.padding, this.glow});

  final Widget child;
  final EdgeInsetsGeometry? padding;

  /// Optional neon colour for the border and shadow.
  final Color? glow;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final radius = BorderRadius.circular(tokens.radiusLarge);
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: radius,
        boxShadow: [
          if (glow != null)
            BoxShadow(
              color: glow!.withValues(alpha: 0.25),
              blurRadius: tokens.space6,
            ),
        ],
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: BackdropFilter(
          filter: ui.ImageFilter.blur(
            sigmaX: tokens.blurSigma,
            sigmaY: tokens.blurSigma,
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: tokens.glass,
              borderRadius: radius,
              border: Border.all(
                color: glow?.withValues(alpha: 0.5) ?? tokens.glassBorder,
              ),
            ),
            child: Padding(
              padding: padding ?? EdgeInsets.all(tokens.space4),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

/// Large round button with a neon glow (play, cue...).
class NeonIconButton extends StatelessWidget {
  const new({
    required this.icon,
    required this.label,
    required this.onPressed,
    super.key,
    this.color,
    this.size = 56,
    this.active = false,
  });

  final IconData icon;

  /// Accessible label and tooltip.
  final String label;
  final VoidCallback? onPressed;
  final Color? color;
  final double size;

  /// Highlighted state (e.g. playing).
  final bool active;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final accent = color ?? tokens.neonCyan;
    final enabled = onPressed != null;
    return Semantics(
      button: true,
      label: label,
      enabled: enabled,
      child: Tooltip(
        message: label,
        child: AnimatedContainer(
          duration: tokens.durationMedium,
          curve: tokens.curveStandard,
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: active ? accent.withValues(alpha: 0.22) : tokens.glass,
            border: Border.all(
              color: enabled ? accent : tokens.glassBorder,
              width: 2,
            ),
            boxShadow: [
              if (active)
                BoxShadow(
                  color: accent.withValues(alpha: 0.6),
                  blurRadius: tokens.space5,
                ),
            ],
          ),
          child: Material(
            type: MaterialType.transparency,
            shape: const CircleBorder(),
            clipBehavior: Clip.antiAlias,
            child: InkWell(
              onTap: onPressed,
              child: Icon(
                icon,
                size: size * 0.45,
                color: enabled ? tokens.textPrimary : tokens.textMuted,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Vertical peak meter, green to red.
class VuMeter extends StatelessWidget {
  const new({
    required this.level,
    required this.label,
    super.key,
    this.width = 10,
  });

  /// Linear peak (1 = 0 dBFS).
  final double level;
  final String label;
  final double width;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Semantics(
      label: label,
      value: '${(levelToMeter(level) * 100).round()} %',
      child: RepaintBoundary(
        child: CustomPaint(
          size: Size(width, double.infinity),
          painter: _VuPainter(
            fill: levelToMeter(level),
            background: tokens.glass,
            low: tokens.success,
            mid: tokens.warning,
            high: tokens.danger,
            radius: tokens.radiusSmall,
          ),
        ),
      ),
    );
  }
}

/// Maps a linear peak to a 0..1 meter position over -48..0 dBFS.
double levelToMeter(double level) {
  if (level <= 0) return 0;
  final db = 20 * math.log(level) / math.ln10;
  return ((db + 48) / 48).clamp(0, 1);
}

class _VuPainter extends CustomPainter {
  const new({
    required this.fill,
    required this.background,
    required this.low,
    required this.mid,
    required this.high,
    required this.radius,
  });

  final double fill;
  final Color background;
  final Color low;
  final Color mid;
  final Color high;
  final double radius;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final rounded = RRect.fromRectAndRadius(rect, Radius.circular(radius));
    canvas
      ..drawRRect(rounded, Paint()..color = background)
      ..save()
      ..clipRRect(rounded);
    final top = size.height * (1 - fill);
    final gradient = LinearGradient(
      begin: Alignment.bottomCenter,
      end: Alignment.topCenter,
      colors: [low, low, mid, high],
      stops: const [0, 0.6, 0.85, 1],
    ).createShader(rect);
    canvas
      ..drawRect(
        Rect.fromLTRB(0, top, size.width, size.height),
        Paint()..shader = gradient,
      )
      ..restore();
  }

  @override
  bool shouldRepaint(_VuPainter old) => old.fill != fill;
}

/// Text style for counters (tabular figures, display font).
TextStyle counterStyle(BuildContext context, {double size = 16}) {
  final tokens = context.tokens;
  return TextStyle(
    fontFamily: tokens.displayFamily,
    fontSize: size,
    fontWeight: FontWeight.w500,
    color: tokens.textPrimary,
    fontFeatures: tabularFigures,
  );
}

/// `m:ss` with tabular figures in mind.
String formatClock(double seconds) {
  final total = seconds.isFinite && seconds > 0 ? seconds.floor() : 0;
  final minutes = total ~/ 60;
  final rest = (total % 60).toString().padLeft(2, '0');
  return '$minutes:$rest';
}
