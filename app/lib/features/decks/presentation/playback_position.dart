import 'package:flutter/scheduler.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Rebuilds [builder] on every frame while [playing], with the position
/// extrapolated from the last engine report (~30 Hz), so that waveforms and
/// jog wheels move smoothly and stop exactly with the music.
class PlaybackPosition extends StatefulWidget {
  const new({
    required this.position,
    required this.updatedAt,
    required this.playing,
    required this.builder,
    super.key,
  });

  final double position;
  final DateTime? updatedAt;
  final bool playing;
  final Widget Function(BuildContext context, double position) builder;

  @override
  State<PlaybackPosition> createState() => _PlaybackPositionState();
}

class _PlaybackPositionState extends State<PlaybackPosition>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => setState(() {}));

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _syncTicker();
  }

  @override
  void didUpdateWidget(PlaybackPosition oldWidget) {
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

  double get _estimated {
    final updatedAt = widget.updatedAt;
    if (!widget.playing || updatedAt == null) return widget.position;
    final elapsed = DateTime.now().difference(updatedAt).inMicroseconds / 1e6;
    return widget.position + elapsed.clamp(0, 0.5);
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _estimated);
}
