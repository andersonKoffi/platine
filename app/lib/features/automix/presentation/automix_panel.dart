import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

/// Auto-mix controls: on/off, fade length, curve, shuffle, repeat, status.
class AutoMixPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final settings = ref.watch(mixSettingsControllerProvider);
    final controller = ref.read(mixSettingsControllerProvider.notifier);
    final mode = ref.watch(sessionStoreProvider.select((s) => s.mode));
    return GlassPanel(
      padding: EdgeInsets.symmetric(
        horizontal: tokens.space4,
        vertical: tokens.space3,
      ),
      child: Wrap(
        spacing: tokens.space5,
        runSpacing: tokens.space3,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          _AutoMixSwitch(
            enabled: settings.automix,
            onChanged: (value) => controller.setAutoMix(enabled: value),
          ),
          _StatusBadge(mode: mode),
          _CrossfadeControl(
            seconds: settings.crossfadeSeconds,
            onChanged: controller.setCrossfadeSeconds,
          ),
          _CurvePicker(curve: settings.curve, onChanged: controller.setCurve),
          _ToggleChip(
            icon: Icons.shuffle_rounded,
            label: 'Aléatoire',
            selected: settings.shuffle,
            onChanged: (value) => controller.setShuffle(enabled: value),
          ),
          _ToggleChip(
            icon: Icons.repeat_rounded,
            label: 'Repeat',
            selected: settings.repeat,
            onChanged: (value) => controller.setRepeat(enabled: value),
          ),
        ],
      ),
    );
  }
}

class _AutoMixSwitch extends StatelessWidget {
  const new({required this.enabled, required this.onChanged});

  final bool enabled;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        ShaderMask(
          shaderCallback: tokens.neonGradient.createShader,
          child: Text(
            'AUTO-MIX',
            style: Theme.of(context).textTheme.titleMedium
                ?.copyWith(color: Colors.white, letterSpacing: 2),
          ),
        ),
        SizedBox(width: tokens.space2),
        Semantics(
          label: 'Activer l’auto-mix',
          child: Switch(value: enabled, onChanged: onChanged),
        ),
      ],
    );
  }
}

class _StatusBadge extends ConsumerWidget {
  const new({required this.mode});

  final AutoMixMode mode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final (label, color) = switch (mode) {
      AutoMixMode.running => ('En marche', tokens.success),
      AutoMixMode.suspended => ('Suspendu', tokens.warning),
      AutoMixMode.off => ('Arrêté', tokens.textMuted),
    };
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Semantics(
          liveRegion: true,
          label: 'Auto-mix $label',
          child: Chip(
            avatar: Icon(Icons.circle, size: 10, color: color),
            label: Text(label),
          ),
        ),
        if (mode == AutoMixMode.suspended) ...[
          SizedBox(width: tokens.space2),
          FilledButton.icon(
            onPressed: () =>
                unawaited(ref.read(engineGatewayProvider).resumeAutoMix()),
            icon: const Icon(Icons.play_circle_outline_rounded),
            label: const Text('Reprendre'),
          ),
        ],
      ],
    );
  }
}

class _CrossfadeControl extends StatelessWidget {
  const new({required this.seconds, required this.onChanged});

  final double seconds;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Fondu', style: TextStyle(color: tokens.textSecondary)),
        SizedBox(
          width: 160,
          child: Semantics(
            label: 'Durée du fondu',
            child: Slider(
              value: seconds,
              max: MixSettings.maxCrossfadeSeconds,
              divisions: 60,
              label: '${seconds.toStringAsFixed(1)} s',
              onChanged: onChanged,
            ),
          ),
        ),
        SizedBox(
          width: 48,
          child: Text(
            '${seconds.toStringAsFixed(seconds % 1 == 0 ? 0 : 1)} s',
            style: counterStyle(context, size: 14),
          ),
        ),
        for (final preset in MixSettings.crossfadePresets)
          Padding(
            padding: EdgeInsets.only(left: tokens.space1),
            child: ChoiceChip(
              label: Text('${preset.round()}'),
              selected: seconds == preset,
              onSelected: (_) => onChanged(preset),
              tooltip: 'Fondu de ${preset.round()} secondes',
            ),
          ),
      ],
    );
  }
}

class _CurvePicker extends StatelessWidget {
  const new({required this.curve, required this.onChanged});

  final CrossfadeCurveType curve;
  final ValueChanged<CrossfadeCurveType> onChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<CrossfadeCurveType>(
      segments: const [
        ButtonSegment(
          value: CrossfadeCurveType.equalPower,
          label: Text('Equal-power'),
        ),
        ButtonSegment(
          value: CrossfadeCurveType.linear,
          label: Text('Linéaire'),
        ),
        ButtonSegment(value: CrossfadeCurveType.cut, label: Text('Cut')),
      ],
      selected: {curve},
      showSelectedIcon: false,
      onSelectionChanged: (selection) => onChanged(selection.first),
    );
  }
}

class _ToggleChip extends StatelessWidget {
  const new({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onChanged,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      avatar: Icon(icon, size: 18),
      label: Text(label),
      selected: selected,
      onSelected: onChanged,
      showCheckmark: false,
    );
  }
}
