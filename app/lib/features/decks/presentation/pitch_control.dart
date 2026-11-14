import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/controls.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/application/deck_controls.dart';

/// Pitch fader of a deck: range selector, fader (up = faster, double-click
/// resets) and the current speed change.
class PitchControl extends ConsumerWidget {
  const new({required this.side, required this.accent, super.key});

  final DeckSide side;
  final Color accent;

  static const double width = 52;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final controls = ref.watch(deckControlsControllerProvider(side));
    final controller = ref.read(deckControlsControllerProvider(side).notifier);
    final value = formatPitch(controls.pitch);
    return SizedBox(
      width: width,
      child: Column(
        children: [
          PadButton(
            label: '±${controls.pitchRange.percent.round()}',
            tooltip: 'Plage du pitch platine ${side.label}',
            color: accent,
            height: 24,
            width: width,
            onPressed: controller.cycleRange,
          ),
          SizedBox(height: tokens.space1),
          PadButton(
            label: 'KEY',
            tooltip: 'Keylock platine ${side.label} : garder la tonalité',
            color: accent,
            height: 24,
            width: width,
            active: controls.keylock,
            onPressed: controller.toggleKeylock,
          ),
          SizedBox(height: tokens.space1),
          const ControlLabel('+'),
          Expanded(
            child: VerticalFader(
              position: controls.pitchPosition,
              onChanged: controller.setPitchPosition,
              label: 'Pitch platine ${side.label}',
              semanticValue: value,
              color: accent,
              defaultPosition: 0.5,
              jumpOnTap: false,
            ),
          ),
          const ControlLabel('−'),
          SizedBox(height: tokens.space1),
          GestureDetector(
            onTap: controller.resetPitch,
            child: Tooltip(
              message: 'Remettre le pitch à 0',
              child: Text(
                value,
                style: counterStyle(
                  context,
                  size: 12,
                  color: controls.pitch == 0 ? tokens.textMuted : accent,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// `+2.35%`, `−1.00%`, `0.00%`.
String formatPitch(double percent) {
  if (percent == 0) return '0.00%';
  final sign = percent > 0 ? '+' : '−';
  return '$sign${percent.abs().toStringAsFixed(2)}%';
}
