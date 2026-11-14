import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/engine/domain/models.dart';

/// Colours of the three waveform bands.
@immutable
class WaveformPalette {
  const new({
    required this.low,
    required this.mid,
    required this.high,
    required this.background,
    required this.playhead,
    required this.accent,
  });

  factory fromTokens(PlatineTokens tokens, Color accent) => WaveformPalette(
    low: tokens.waveLow,
    mid: tokens.waveMid,
    high: tokens.waveHigh,
    background: tokens.inset,
    playhead: tokens.textPrimary,
    accent: accent,
  );

  final Color low;
  final Color mid;
  final Color high;
  final Color background;
  final Color playhead;
  final Color accent;

  /// Colour of a point: the bands mixed by their energy (RGB waveform).
  Color mix(int low, int mid, int high) {
    final wl = low * low.toDouble();
    final wm = mid * mid.toDouble();
    final wh = high * high.toDouble();
    final total = wl + wm + wh;
    if (total == 0) return this.low.withValues(alpha: 0);
    double channel(double Function(Color c) of) =>
        (of(this.low) * wl + of(this.mid) * wm + of(this.high) * wh) / total;
    return Color.from(
      alpha: 1,
      red: channel((c) => c.r),
      green: channel((c) => c.g),
      blue: channel((c) => c.b),
    );
  }
}

/// Peak of the three bands over `[from, to)` (indices clamped).
(int, int, int) bandPeaks(WaveformData data, int from, int to) {
  final start = math.max(0, from);
  final end = math.min(data.length, math.max(to, from + 1));
  var low = 0;
  var mid = 0;
  var high = 0;
  for (var i = start; i < end; i++) {
    low = math.max(low, data.low[i]);
    mid = math.max(mid, data.mid[i]);
    high = math.max(high, data.high[i]);
  }
  return (low, mid, high);
}

/// Large scrolling waveform centred on the playhead (top of the screen).
class ScrollingWaveformPainter extends CustomPainter {
  const new({
    required this.data,
    required this.position,
    required this.palette,
    required this.secondsVisible,
    required this.start,
    required this.end,
    this.mirrored = true,
  });

  final WaveformData? data;
  final double position;
  final WaveformPalette palette;
  final double secondsVisible;

  /// Audible start and end, marked on the waveform.
  final double start;
  final double end;

  /// Draws the waveform symmetric around the middle line.
  final bool mirrored;

  static const double _column = 2;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = palette.background);
    final midY = size.height / 2;
    final pxPerSecond = size.width / secondsVisible;
    final centerX = size.width / 2;
    final waveform = data;
    // Time grid: a light tick every second.
    final tick = Paint()
      ..color = palette.playhead.withValues(alpha: 0.06)
      ..strokeWidth = 1;
    final firstSecond = (position - secondsVisible / 2).floor();
    for (var s = firstSecond; s <= position + secondsVisible / 2; s++) {
      final x = centerX + (s - position) * pxPerSecond;
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), tick);
    }
    if (waveform != null && waveform.length > 0) {
      final pps = waveform.pointsPerSecond;
      final paint = Paint();
      for (double x = 0; x < size.width; x += _column) {
        final t0 = position + (x - centerX) / pxPerSecond;
        final t1 = position + (x + _column - centerX) / pxPerSecond;
        final i0 = (t0 * pps).floor();
        final i1 = (t1 * pps).ceil();
        if (i1 <= 0 || i0 >= waveform.length) continue;
        final (low, mid, high) = bandPeaks(waveform, i0, i1);
        final peak = math.max(low, math.max(mid, high)) / 255;
        if (peak == 0) continue;
        final past = x + _column <= centerX;
        final color = palette.mix(low, mid, high);
        paint.color = past ? color.withValues(alpha: 0.45) : color;
        final h = peak * (mirrored ? midY : size.height) * 0.95;
        final rect = mirrored
            ? Rect.fromLTRB(x, midY - h, x + _column - 0.5, midY + h)
            : Rect.fromLTRB(x, size.height - h, x + _column - 0.5, size.height);
        canvas.drawRect(rect, paint);
        // Bright core for the highs.
        if (high > 40 && mirrored) {
          final core = high / 255 * midY * 0.35;
          paint.color = Colors.white.withValues(alpha: past ? 0.18 : 0.35);
          canvas.drawRect(
            Rect.fromLTRB(x, midY - core, x + _column - 0.5, midY + core),
            paint,
          );
        }
      }
    }
    // Audible start / end markers.
    final marker = Paint()
      ..color = palette.accent.withValues(alpha: 0.8)
      ..strokeWidth = 2;
    for (final t in [start, end]) {
      if (end <= 0) break;
      final x = centerX + (t - position) * pxPerSecond;
      if (x >= 0 && x <= size.width) {
        canvas.drawLine(Offset(x, 0), Offset(x, size.height), marker);
      }
    }
    // Playhead.
    canvas
      ..drawRect(
        Rect.fromLTWH(centerX - 1, 0, 2, size.height),
        Paint()..color = palette.playhead,
      )
      ..drawPath(
        Path()
          ..moveTo(centerX - 5, 0)
          ..lineTo(centerX + 5, 0)
          ..lineTo(centerX, 6)
          ..close(),
        Paint()..color = palette.accent,
      );
  }

  @override
  bool shouldRepaint(ScrollingWaveformPainter old) =>
      old.position != position ||
      !identical(old.data, data) ||
      old.secondsVisible != secondsVisible ||
      old.palette != palette;
}

/// Whole-track overview with the played part dimmed.
class OverviewWaveformPainter extends CustomPainter {
  const new({
    required this.data,
    required this.position,
    required this.duration,
    required this.palette,
    required this.start,
    required this.end,
  });

  final WaveformData? data;
  final double position;
  final double duration;
  final WaveformPalette palette;
  final double start;
  final double end;

  @override
  void paint(Canvas canvas, Size size) {
    final background = RRect.fromRectAndRadius(
      Offset.zero & size,
      const Radius.circular(3),
    );
    canvas
      ..drawRRect(background, Paint()..color = palette.background)
      ..save()
      ..clipRRect(background);
    final waveform = data;
    final total = duration > 0 ? duration : (waveform?.duration ?? 0);
    final playedX = total > 0 ? size.width * (position / total) : 0.0;
    if (waveform != null && waveform.length > 0 && total > 0) {
      final pointsPerPixel = waveform.length / size.width;
      final paint = Paint();
      final midY = size.height / 2;
      for (double x = 0; x < size.width; x += 1) {
        final i0 = (x * pointsPerPixel).floor();
        final i1 = ((x + 1) * pointsPerPixel).ceil();
        final (low, mid, high) = bandPeaks(waveform, i0, i1);
        final peak = math.max(low, math.max(mid, high)) / 255;
        if (peak == 0) continue;
        final color = palette.mix(low, mid, high);
        paint.color = x < playedX ? color.withValues(alpha: 0.35) : color;
        final h = peak * midY * 0.92;
        canvas.drawRect(Rect.fromLTRB(x, midY - h, x + 1, midY + h), paint);
      }
    } else {
      canvas.drawRect(
        Rect.fromLTWH(0, size.height / 2 - 0.5, size.width, 1),
        Paint()..color = palette.playhead.withValues(alpha: 0.15),
      );
    }
    if (total > 0) {
      final shade = Paint()..color = Colors.black.withValues(alpha: 0.55);
      canvas
        ..drawRect(
          Rect.fromLTWH(0, 0, size.width * (start / total), size.height),
          shade,
        )
        ..drawRect(
          Rect.fromLTRB(size.width * (end / total), 0, size.width, size.height),
          shade,
        )
        ..drawRect(
          Rect.fromLTWH(playedX - 1, 0, 2, size.height),
          Paint()..color = palette.playhead,
        );
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(OverviewWaveformPainter old) =>
      old.position != position ||
      !identical(old.data, data) ||
      old.duration != duration;
}
