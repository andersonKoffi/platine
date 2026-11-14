import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/mixer/application/mixer_controller.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

/// Crossfader, master volume and master VU meter.
class MixerPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final master = ref.watch(masterGainProvider);
    final level = ref.watch(sessionStoreProvider.select((s) => s.levels.master));
    return GlassPanel(
      child: Column(
        children: [
          Text('Mixer', style: Theme.of(context).textTheme.titleMedium),
          SizedBox(height: tokens.space3),
          Expanded(
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Column(
                  children: [
                    Expanded(
                      child: RotatedBox(
                        quarterTurns: 3,
                        child: Semantics(
                          label: 'Volume master',
                          child: Slider(
                            value: master,
                            max: MasterGain.max,
                            onChanged: ref
                                .read(masterGainProvider.notifier)
                                .set,
                          ),
                        ),
                      ),
                    ),
                    Text(
                      'Master',
                      style: TextStyle(color: tokens.textSecondary),
                    ),
                  ],
                ),
                SizedBox(width: tokens.space3),
                VuMeter(level: level, label: 'Niveau master', width: 14),
              ],
            ),
          ),
          SizedBox(height: tokens.space4),
          const Crossfader(),
        ],
      ),
    );
  }
}

/// Large crossfader. It glides on its own during automatic transitions.
class Crossfader extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<Crossfader> createState() => _CrossfaderState();
}

class _CrossfaderState extends ConsumerState<Crossfader>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => setState(() {}));

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  void _syncTicker(TransitionView? transition) {
    final animate = transition != null && !context.reduceMotion;
    if (animate && !_ticker.isActive) {
      _ticker.start();
    } else if (!animate && _ticker.isActive) {
      _ticker.stop();
    }
  }

  double _displayed(SessionState session, double fadeSeconds) {
    final transition = session.transition;
    if (transition == null) return session.crossfader;
    final from = transition.from.crossfaderSide;
    final to = transition.to.crossfaderSide;
    if (fadeSeconds <= 0 || context.reduceMotion) return to;
    final elapsed =
        DateTime.now().difference(transition.startedAt).inMilliseconds / 1000;
    final progress = (elapsed / fadeSeconds).clamp(0, 1).toDouble();
    return from + (to - from) * progress;
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final session = ref.watch(sessionStoreProvider);
    final fade = ref.watch(
      mixSettingsControllerProvider.select((s) => s.crossfadeSeconds),
    );
    _syncTicker(session.transition);
    final value = _displayed(session, fade);
    final controller = ref.read(crossfaderControllerProvider);
    return Column(
      children: [
        Row(
          children: [
            _SideLabel(side: DeckSide.a, onTap: controller.toDeck),
            Expanded(
              child: SliderTheme(
                data: SliderTheme.of(context).copyWith(
                  trackHeight: 8,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 14,
                  ),
                ),
                child: Semantics(
                  label: 'Crossfader',
                  child: Slider(
                    value: value,
                    onChanged: controller.move,
                  ),
                ),
              ),
            ),
            _SideLabel(side: DeckSide.b, onTap: controller.toDeck),
          ],
        ),
        Text(
          session.transition != null ? 'Fondu automatique…' : 'Crossfader',
          style: TextStyle(color: tokens.textSecondary),
        ),
      ],
    );
  }
}

class _SideLabel extends StatelessWidget {
  const new({required this.side, required this.onTap});

  final DeckSide side;
  final ValueChanged<DeckSide> onTap;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final color = tokens.deckColor(isDeckA: side == DeckSide.a);
    return Tooltip(
      message: 'Crossfader tout sur ${side.label}',
      child: InkWell(
        onTap: () => onTap(side),
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        child: SizedBox(
          width: tokens.minTouchTarget,
          height: tokens.minTouchTarget,
          child: Center(
            child: Text(
              side.label,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: color,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
