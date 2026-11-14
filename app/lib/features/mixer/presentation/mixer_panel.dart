import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/controls.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/decks/application/deck_controls.dart';
import 'package:platine/features/mixer/application/mixer_controller.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

/// Knob position (`0..1`, neutral in the middle) of an isolator level
/// (`0..2`, neutral 1).
double eqToKnob(double level) =>
    level <= 1 ? level * 0.5 : 0.5 + (level - 1) / (DeckControls.maxLevel - 1) * 0.5;

/// Inverse of [eqToKnob].
double knobToEq(double position) => position <= 0.5
    ? position * 2
    : 1 + (position - 0.5) * 2 * (DeckControls.maxLevel - 1);

/// Fader position of a deck gain: unity at 80 %, +6 dB at the top.
double gainToFader(double gain) =>
    gain <= 1 ? gain * 0.8 : 0.8 + (gain - 1) / (DeckControls.maxLevel - 1) * 0.2;

/// Inverse of [gainToFader].
double faderToGain(double position) => position <= 0.8
    ? position / 0.8
    : 1 + (position - 0.8) / 0.2 * (DeckControls.maxLevel - 1);

/// Two-channel mixer: isolator EQ, channel faders with meters, master and
/// crossfader.
class MixerPanel extends ConsumerWidget {
  const new({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final master = ref.watch(masterGainProvider);
    final level = ref.watch(
      sessionStoreProvider.select((s) => s.levels.master),
    );
    return PanelBox(
      padding: EdgeInsets.symmetric(
        horizontal: tokens.space2,
        vertical: tokens.space3,
      ),
      child: Column(
        children: [
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Expanded(child: _ChannelStrip(side: DeckSide.a)),
                Container(width: 1, color: tokens.panelBorder),
                SizedBox(
                  width: 64,
                  child: Column(
                    children: [
                      const ControlLabel('Master'),
                      SizedBox(height: tokens.space1),
                      Knob(
                        position: master / MasterGain.max,
                        defaultPosition: 1 / MasterGain.max,
                        bipolar: false,
                        color: tokens.textPrimary,
                        label: 'Volume master',
                        onChanged: (p) => ref
                            .read(masterGainProvider.notifier)
                            .set(p * MasterGain.max),
                      ),
                      SizedBox(height: tokens.space3),
                      Expanded(
                        child: VuMeter(
                          level: level,
                          label: 'Niveau master',
                          width: 14,
                        ),
                      ),
                    ],
                  ),
                ),
                Container(width: 1, color: tokens.panelBorder),
                const Expanded(child: _ChannelStrip(side: DeckSide.b)),
              ],
            ),
          ),
          SizedBox(height: tokens.space3),
          const Crossfader(),
        ],
      ),
    );
  }
}

/// EQ knobs (double-click kills a band), meter and volume fader of a deck.
class _ChannelStrip extends ConsumerWidget {
  const new({required this.side});

  final DeckSide side;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final controls = ref.watch(deckControlsControllerProvider(side));
    final controller = ref.read(deckControlsControllerProvider(side).notifier);
    final level = ref.watch(
      sessionStoreProvider.select((s) => s.levels.of(side)),
    );
    final accent = tokens.deckColor(isDeckA: side == DeckSide.a);
    final meter = VuMeter(level: level, label: 'Niveau platine ${side.label}');
    final fader = VerticalFader(
      position: gainToFader(controls.gain),
      label: 'Volume platine ${side.label}',
      color: accent,
      onChanged: (p) => controller.setGain(faderToGain(p)),
    );
    return Column(
      children: [
        for (final band in EqBandType.values.reversed) ...[
          _EqKnob(
            side: side,
            band: band,
            level: controls.band(band),
            onChanged: (p) => controller.setBand(band, knobToEq(p)),
            onKill: () => controller.toggleKill(band),
          ),
          SizedBox(height: tokens.space1),
        ],
        SizedBox(height: tokens.space2),
        Expanded(
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: side == DeckSide.a
                ? [meter, SizedBox(width: tokens.space2), fader]
                : [fader, SizedBox(width: tokens.space2), meter],
          ),
        ),
        SizedBox(height: tokens.space1),
        ControlLabel(side.label, color: accent),
      ],
    );
  }
}

class _EqKnob extends StatelessWidget {
  const new({
    required this.side,
    required this.band,
    required this.level,
    required this.onChanged,
    required this.onKill,
  });

  final DeckSide side;
  final EqBandType band;
  final double level;
  final ValueChanged<double> onChanged;
  final VoidCallback onKill;

  String get _name => switch (band) {
    EqBandType.low => 'Low',
    EqBandType.mid => 'Mid',
    EqBandType.high => 'High',
  };

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final killed = level == 0;
    final color = switch (band) {
      EqBandType.low => tokens.waveLow,
      EqBandType.mid => tokens.waveMid,
      EqBandType.high => tokens.waveHigh,
    };
    return Column(
      children: [
        Tooltip(
          message: killed ? 'Rétablir $_name' : 'Couper $_name',
          child: InkWell(
            onTap: onKill,
            borderRadius: BorderRadius.circular(tokens.radiusSmall),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
              child: ControlLabel(
                killed ? '$_name kill' : _name,
                color: killed ? tokens.danger : null,
              ),
            ),
          ),
        ),
        Knob(
          position: eqToKnob(level),
          label: 'Égaliseur $_name platine ${side.label}',
          color: killed ? tokens.danger : color,
          size: 38,
          onChanged: onChanged,
          onDoubleTap: onKill,
        ),
      ],
    );
  }
}

/// Crossfader. It glides on its own during automatic transitions.
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
              child: HorizontalFader(
                position: value,
                label: 'Crossfader',
                leftColor: tokens.deckA,
                rightColor: tokens.deckB,
                onChanged: controller.move,
              ),
            ),
            _SideLabel(side: DeckSide.b, onTap: controller.toDeck),
          ],
        ),
        ControlLabel(
          session.transition != null ? 'Fondu automatique…' : 'Crossfader',
          color: session.transition != null ? tokens.textPrimary : null,
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
          width: 26,
          height: 32,
          child: Center(
            child: Text(
              side.label,
              style: Theme.of(context).textTheme.titleMedium
                  ?.copyWith(color: color),
            ),
          ),
        ),
      ),
    );
  }
}
