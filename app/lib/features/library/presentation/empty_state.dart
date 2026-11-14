import 'dart:async';
import 'dart:math' as math;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/features/library/application/library_controller.dart';

/// Welcome screen: a big drop zone with an animated record.
class EmptyLibrary extends ConsumerStatefulWidget {
  const new({required this.dragging, super.key});

  /// A folder is being dragged over the window.
  final bool dragging;

  @override
  ConsumerState<EmptyLibrary> createState() => _EmptyLibraryState();
}

class _EmptyLibraryState extends ConsumerState<EmptyLibrary>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulse = AnimationController(
    vsync: this,
    duration: const Duration(seconds: 3),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (context.reduceMotion) {
      _pulse.stop();
    } else if (!_pulse.isAnimating) {
      _pulse.repeat();
    }
  }

  @override
  void dispose() {
    _pulse.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final textTheme = Theme.of(context).textTheme;
    return PanelBox(
      accent: widget.dragging ? tokens.deckA : null,
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: 120,
              height: 120,
              child: AnimatedBuilder(
                animation: _pulse,
                builder: (context, _) => CustomPaint(
                  painter: _PulsePainter(
                    t: _pulse.value,
                    colors: tokens.neonGradient.colors,
                    active: widget.dragging,
                  ),
                  child: Center(
                    child: Icon(
                      widget.dragging
                          ? Icons.download_rounded
                          : Icons.album_rounded,
                      size: 44,
                      color: tokens.textPrimary,
                    ),
                  ),
                ),
              ),
            ),
            SizedBox(height: tokens.space3),
            Text(
              widget.dragging
                  ? 'Lâche, on s’occupe du reste !'
                  : 'Dépose un dossier de musique ici',
              style: textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            SizedBox(height: tokens.space2),
            Text(
              'Platine charge le titre 1 sur la platine A, le titre 2 sur la '
              'platine B, et enchaîne tout seul.',
              style: TextStyle(color: tokens.textSecondary),
              textAlign: TextAlign.center,
            ),
            SizedBox(height: tokens.space4),
            FilledButton.icon(
              style: FilledButton.styleFrom(
                minimumSize: Size(0, tokens.minTouchTarget + tokens.space2),
                padding: EdgeInsets.symmetric(horizontal: tokens.space5),
              ),
              onPressed: () =>
                  unawaited(ref.read(libraryLoaderProvider).pickAndLoad()),
              icon: const Icon(Icons.folder_open_rounded),
              label: const Text('Choisir un dossier  (⌘O)'),
            ),
          ],
        ),
      ),
    );
  }
}

class _PulsePainter extends CustomPainter {
  const new({required this.t, required this.colors, required this.active});

  final double t;
  final List<Color> colors;
  final bool active;

  @override
  void paint(Canvas canvas, Size size) {
    final center = size.center(Offset.zero);
    final radius = size.shortestSide / 2;
    for (var ring = 0; ring < 3; ring++) {
      final phase = (t + ring / 3) % 1;
      final color = colors[ring % colors.length];
      canvas.drawCircle(
        center,
        radius * (0.45 + 0.55 * phase),
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = active ? 4 : 2
          ..color = color.withValues(alpha: (1 - phase) * 0.8),
      );
    }
    canvas.drawCircle(
      center,
      radius * 0.42,
      Paint()
        ..shader = SweepGradient(
          colors: [...colors, colors.first],
          transform: GradientRotation(t * 2 * math.pi),
        ).createShader(Rect.fromCircle(center: center, radius: radius * 0.42)),
    );
  }

  @override
  bool shouldRepaint(_PulsePainter old) => old.t != t || old.active != active;
}
