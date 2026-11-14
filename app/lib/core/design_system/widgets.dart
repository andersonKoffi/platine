import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/theme.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Flat graphite panel used for every section of the screen.
class PanelBox extends StatelessWidget {
  const new({
    required this.child,
    super.key,
    this.padding,
    this.accent,
    this.color,
  });

  final Widget child;
  final EdgeInsetsGeometry? padding;

  /// Optional accent drawn as a thin top border (deck colour).
  final Color? accent;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color ?? tokens.panel,
        borderRadius: BorderRadius.circular(tokens.radiusMedium),
        border: Border.all(color: tokens.panelBorder),
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [
            Color.lerp(color ?? tokens.panel, Colors.white, 0.025)!,
            color ?? tokens.panel,
          ],
        ),
      ),
      child: Stack(
        children: [
          if (accent != null)
            Positioned(
              left: tokens.radiusMedium,
              right: tokens.radiusMedium,
              top: 0,
              child: Container(height: 2, color: accent),
            ),
          Padding(
            padding: padding ?? EdgeInsets.all(tokens.space3),
            child: child,
          ),
        ],
      ),
    );
  }
}

/// Small uppercase caption above a control ("HIGH", "GAIN"...).
class ControlLabel extends StatelessWidget {
  const new(this.text, {super.key, this.color});

  final String text;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return Text(
      text.toUpperCase(),
      maxLines: 1,
      overflow: TextOverflow.clip,
      style: Theme.of(
        context,
      ).textTheme.labelSmall?.copyWith(color: color),
    );
  }
}

/// Rectangular rubber pad, lit with [color] when [active].
class PadButton extends StatelessWidget {
  const new({
    required this.label,
    required this.onPressed,
    super.key,
    this.tooltip,
    this.icon,
    this.color,
    this.active = false,
    this.height = 30,
    this.width,
  });

  final String label;
  final String? tooltip;
  final IconData? icon;
  final VoidCallback? onPressed;
  final Color? color;
  final bool active;
  final double height;
  final double? width;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final accent = color ?? tokens.deckA;
    final enabled = onPressed != null;
    final foreground = !enabled
        ? tokens.textMuted
        : active
        ? tokens.background
        : accent;
    final content = Row(
      mainAxisSize: MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (icon != null) Icon(icon, size: 16, color: foreground),
        if (icon != null && label.isNotEmpty) SizedBox(width: tokens.space1),
        if (label.isNotEmpty)
          Text(
            label,
            maxLines: 1,
            style: TextStyle(
              fontFamily: tokens.displayFamily,
              fontWeight: FontWeight.w700,
              fontSize: 11,
              letterSpacing: 0.8,
              color: foreground,
            ),
          ),
      ],
    );
    final button = Semantics(
      button: true,
      toggled: active,
      enabled: enabled,
      label: tooltip ?? label,
      excludeSemantics: true,
      child: AnimatedContainer(
        duration: tokens.durationFast,
        height: height,
        width: width,
        decoration: BoxDecoration(
          color: active ? accent : tokens.panelRaised,
          borderRadius: BorderRadius.circular(tokens.radiusSmall),
          border: Border.all(
            color: enabled ? accent.withValues(alpha: 0.6) : tokens.panelBorder,
          ),
          boxShadow: [
            if (active)
              BoxShadow(color: accent.withValues(alpha: 0.45), blurRadius: 10),
          ],
        ),
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            borderRadius: BorderRadius.circular(tokens.radiusSmall),
            onTap: onPressed,
            child: Padding(
              padding: EdgeInsets.symmetric(
                horizontal: label.isEmpty ? tokens.space1 / 2 : tokens.space2,
              ),
              child: Center(child: content),
            ),
          ),
        ),
      ),
    );
    final message = tooltip;
    return message == null ? button : Tooltip(message: message, child: button);
  }
}

/// Large round transport button (CUE, PLAY), like on a controller.
class RoundTransportButton extends StatelessWidget {
  const new({
    required this.icon,
    required this.label,
    required this.onPressed,
    super.key,
    this.text,
    this.color,
    this.size = 54,
    this.active = false,
  });

  final IconData icon;

  /// Accessible label and tooltip.
  final String label;

  /// Optional caption drawn under the icon ("CUE").
  final String? text;
  final VoidCallback? onPressed;
  final Color? color;
  final double size;

  /// Lit state (e.g. playing).
  final bool active;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final accent = color ?? tokens.deckA;
    final enabled = onPressed != null;
    final ring = enabled ? accent : tokens.panelBorder;
    return Semantics(
      button: true,
      label: label,
      enabled: enabled,
      excludeSemantics: true,
      child: Tooltip(
        message: label,
        child: AnimatedContainer(
          duration: tokens.durationMedium,
          width: size,
          height: size,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            gradient: RadialGradient(
              colors: [
                Color.lerp(tokens.panelRaised, Colors.white, 0.06)!,
                tokens.panelRaised,
              ],
            ),
            border: Border.all(
              color: active ? ring : ring.withValues(alpha: 0.55),
              width: 2.5,
            ),
            boxShadow: [
              BoxShadow(color: tokens.background, blurRadius: 4),
              if (active)
                BoxShadow(color: accent.withValues(alpha: 0.6), blurRadius: 16),
            ],
          ),
          child: Material(
            type: MaterialType.transparency,
            shape: const CircleBorder(),
            clipBehavior: Clip.antiAlias,
            child: InkWell(
              onTap: onPressed,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    icon,
                    size: size * 0.38,
                    color: !enabled
                        ? tokens.textMuted
                        : active
                        ? accent
                        : tokens.textPrimary,
                  ),
                  if (text != null)
                    Text(
                      text!,
                      style: TextStyle(
                        fontFamily: tokens.displayFamily,
                        fontSize: 9,
                        fontWeight: FontWeight.w700,
                        letterSpacing: 1,
                        color: tokens.textSecondary,
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Segmented LED peak meter (green, amber, red), bottom to top.
class VuMeter extends StatelessWidget {
  const new({
    required this.level,
    required this.label,
    super.key,
    this.width = 8,
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
            background: tokens.inset,
            off: tokens.panelRaised,
            low: tokens.success,
            mid: tokens.warning,
            high: tokens.danger,
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
    required this.off,
    required this.low,
    required this.mid,
    required this.high,
  });

  static const int segments = 18;

  final double fill;
  final Color background;
  final Color off;
  final Color low;
  final Color mid;
  final Color high;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRRect(
      RRect.fromRectAndRadius(Offset.zero & size, const Radius.circular(2)),
      Paint()..color = background,
    );
    const gap = 2.0;
    final segment = (size.height - gap * (segments + 1)) / segments;
    if (segment <= 0) return;
    final lit = (fill * segments).round();
    for (var i = 0; i < segments; i++) {
      final top = size.height - gap - (i + 1) * (segment + gap) + gap;
      final ratio = (i + 1) / segments;
      final color = ratio > 0.88
          ? high
          : ratio > 0.7
          ? mid
          : low;
      canvas.drawRect(
        Rect.fromLTWH(1.5, top, size.width - 3, segment),
        Paint()..color = i < lit ? color : off,
      );
    }
  }

  @override
  bool shouldRepaint(_VuPainter old) => old.fill != fill;
}

/// Text style for counters (tabular figures).
TextStyle counterStyle(BuildContext context, {double size = 16, Color? color}) {
  final tokens = context.tokens;
  return TextStyle(
    fontFamily: tokens.displayFamily,
    fontSize: size,
    fontWeight: FontWeight.w700,
    color: color ?? tokens.textPrimary,
    fontFeatures: tabularFigures,
  );
}

/// `m:ss` clock.
String formatClock(double seconds) {
  final total = seconds.isFinite && seconds > 0 ? seconds.floor() : 0;
  final minutes = total ~/ 60;
  final rest = (total % 60).toString().padLeft(2, '0');
  return '$minutes:$rest';
}

/// `m:ss.t` clock with tenths, for deck counters.
String formatPreciseClock(double seconds) {
  final value = seconds.isFinite && seconds > 0 ? seconds : 0.0;
  final tenths = (value * 10).floor();
  final minutes = tenths ~/ 600;
  final secs = ((tenths % 600) ~/ 10).toString().padLeft(2, '0');
  return '$minutes:$secs.${tenths % 10}';
}
