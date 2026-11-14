import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

/// Auto-mix side panel: on/off and status, fade length and curve, shuffle
/// and repeat, and the reorderable queue.
class AutoMixPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final settings = ref.watch(mixSettingsControllerProvider);
    final controller = ref.read(mixSettingsControllerProvider.notifier);
    final mode = ref.watch(sessionStoreProvider.select((s) => s.mode));
    return PanelBox(
      padding: EdgeInsets.all(tokens.space3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text(
                'AUTOMIX',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(letterSpacing: 2),
              ),
              const Spacer(),
              PadButton(
                label: '',
                icon: Icons.shuffle_rounded,
                tooltip: 'Aléatoire',
                color: tokens.deckB,
                width: 32,
                height: 26,
                active: settings.shuffle,
                onPressed: () =>
                    controller.setShuffle(enabled: !settings.shuffle),
              ),
              SizedBox(width: tokens.space1),
              PadButton(
                label: '',
                icon: Icons.repeat_rounded,
                tooltip: 'Repeat',
                color: tokens.deckB,
                width: 32,
                height: 26,
                active: settings.repeat,
                onPressed: () =>
                    controller.setRepeat(enabled: !settings.repeat),
              ),
              SizedBox(
                height: 26,
                child: FittedBox(
                  child: Semantics(
                    label: 'Activer l’auto-mix',
                    child: Switch(
                      value: settings.automix,
                      onChanged: (value) =>
                          controller.setAutoMix(enabled: value),
                    ),
                  ),
                ),
              ),
            ],
          ),
          SizedBox(height: tokens.space1),
          _StatusRow(mode: mode),
          SizedBox(height: tokens.space2),
          _CrossfadeControl(
            seconds: settings.crossfadeSeconds,
            onChanged: controller.setCrossfadeSeconds,
          ),
          SizedBox(height: tokens.space2),
          Row(
            children: [
              for (final (curve, label) in [
                (CrossfadeCurveType.equalPower, 'SMOOTH'),
                (CrossfadeCurveType.linear, 'LINÉAIRE'),
                (CrossfadeCurveType.cut, 'CUT'),
              ]) ...[
                Expanded(
                  child: PadButton(
                    label: label,
                    height: 26,
                    tooltip: switch (curve) {
                      CrossfadeCurveType.equalPower =>
                        'Courbe equal-power (volume constant)',
                      CrossfadeCurveType.linear => 'Courbe linéaire',
                      CrossfadeCurveType.cut => 'Coupure franche',
                    },
                    active: settings.curve == curve,
                    onPressed: () => controller.setCurve(curve),
                  ),
                ),
                if (curve != CrossfadeCurveType.cut)
                  SizedBox(width: tokens.space1),
              ],
            ],
          ),
          SizedBox(height: tokens.space3),
          const ControlLabel('File d’attente'),
          SizedBox(height: tokens.space1),
          const Expanded(child: _QueueList()),
        ],
      ),
    );
  }
}

class _StatusRow extends ConsumerWidget {
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
      children: [
        Semantics(
          liveRegion: true,
          label: 'Auto-mix $label',
          excludeSemantics: true,
          child: Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: color,
                  shape: BoxShape.circle,
                  boxShadow: [
                    BoxShadow(color: color.withValues(alpha: 0.6), blurRadius: 6),
                  ],
                ),
              ),
              SizedBox(width: tokens.space2),
              Text(label, style: TextStyle(color: color, fontSize: 12)),
            ],
          ),
        ),
        const Spacer(),
        if (mode == AutoMixMode.suspended)
          PadButton(
            label: 'Reprendre',
            icon: Icons.play_arrow_rounded,
            color: tokens.warning,
            height: 26,
            onPressed: () =>
                unawaited(ref.read(engineGatewayProvider).resumeAutoMix()),
          ),
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            const ControlLabel('Fondu'),
            const Spacer(),
            Text(
              '${seconds.toStringAsFixed(seconds % 1 == 0 ? 0 : 1)} s',
              style: counterStyle(context, size: 14),
            ),
          ],
        ),
        Semantics(
          label: 'Durée du fondu',
          child: SizedBox(
            height: 26,
            child: Slider(
              padding: EdgeInsets.zero,
            value: seconds,
            max: MixSettings.maxCrossfadeSeconds,
            divisions: 60,
            label: '${seconds.toStringAsFixed(1)} s',
            onChanged: onChanged,
            ),
          ),
        ),
        Row(
          children: [
            for (final preset in MixSettings.crossfadePresets) ...[
              Expanded(
                child: PadButton(
                  label: '${preset.round()}',
                  tooltip: 'Fondu de ${preset.round()} secondes',
                  height: 26,
                  active: seconds == preset,
                  onPressed: () => onChanged(preset),
                ),
              ),
              if (preset != MixSettings.crossfadePresets.last)
                SizedBox(width: tokens.space1),
            ],
          ],
        ),
      ],
    );
  }
}

class _QueueList extends ConsumerWidget {
  const new();

  static String _titleOf(List<TrackInfo> library, int id) {
    for (final track in library) {
      if (track.id == id) return track.title;
    }
    return 'Titre $id';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final upcoming = ref.watch(sessionStoreProvider.select((s) => s.upcoming));
    final library = ref.watch(sessionStoreProvider.select((s) => s.library));
    if (upcoming.isEmpty) {
      return Center(
        child: Text(
          'La file est vide.',
          style: TextStyle(color: tokens.textMuted, fontSize: 12),
        ),
      );
    }
    return ReorderableListView.builder(
      itemCount: upcoming.length,
      buildDefaultDragHandles: false,
      onReorderItem: (from, to) =>
          unawaited(ref.read(engineGatewayProvider).moveInQueue(from, to)),
      itemBuilder: (context, index) {
        final id = upcoming[index];
        final title = _titleOf(library, id);
        return Container(
          key: ValueKey(id),
          height: 28,
          margin: const EdgeInsets.only(bottom: 2),
          padding: EdgeInsets.symmetric(horizontal: tokens.space2),
          decoration: BoxDecoration(
            color: tokens.inset,
            borderRadius: BorderRadius.circular(tokens.radiusSmall),
          ),
          child: Row(
            children: [
              SizedBox(
                width: 22,
                child: Text(
                  '${index + 1}',
                  style: counterStyle(context, size: 11, color: tokens.textMuted),
                ),
              ),
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              ReorderableDragStartListener(
                index: index,
                child: Semantics(
                  label: 'Déplacer $title',
                  child: Icon(
                    Icons.drag_indicator_rounded,
                    size: 16,
                    color: tokens.textMuted,
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}
