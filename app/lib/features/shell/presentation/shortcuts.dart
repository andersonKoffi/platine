import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/library/application/library_controller.dart';
import 'package:platine/features/mixer/application/mixer_controller.dart';
import 'package:platine/features/settings/application/settings_controller.dart';

/// A keyboard shortcut and what it does, for the help screen.
class ShortcutHelp {
  const new(this.keys, this.description);

  final String keys;
  final String description;
}

/// Shortcuts shown by the help screen (`?`).
const List<ShortcutHelp> shortcutHelp = [
  ShortcutHelp('Espace', 'Lecture / pause de la platine audible'),
  ShortcutHelp('Q  /  W', 'Lecture / pause platine A / B'),
  ShortcutHelp('←  /  →', 'Crossfader vers A / B'),
  ShortcutHelp('↑  /  ↓', 'Volume master'),
  ShortcutHelp('A', 'Activer / désactiver l’auto-mix'),
  ShortcutHelp('R', 'Reprendre l’auto-mix suspendu'),
  ShortcutHelp('S', 'Mode aléatoire'),
  ShortcutHelp('L', 'Repeat'),
  ShortcutHelp('1 … 5', 'Fondu de 0, 4, 8, 12 ou 16 s'),
  ShortcutHelp('⌘O', 'Choisir un dossier'),
  ShortcutHelp('?', 'Afficher cette aide'),
];

/// Builds the shortcut bindings of the main screen.
Map<ShortcutActivator, VoidCallback> buildShortcuts(
  BuildContext context,
  WidgetRef ref,
) {
  final gateway = ref.read(engineGatewayProvider);
  final settings = ref.read(mixSettingsControllerProvider.notifier);
  final crossfader = ref.read(crossfaderControllerProvider);
  final master = ref.read(masterGainProvider.notifier);

  void toggle(DeckSide side) => unawaited(gateway.togglePlay(side));
  void preset(int index) =>
      settings.setCrossfadeSeconds(MixSettings.crossfadePresets[index]);

  return {
    const SingleActivator(LogicalKeyboardKey.space): () {
      final session = ref.read(sessionStoreProvider);
      toggle(
        session.activeDeck ??
            (session.crossfader > 0.5 ? DeckSide.b : DeckSide.a),
      );
    },
    const SingleActivator(LogicalKeyboardKey.keyQ): () => toggle(DeckSide.a),
    const SingleActivator(LogicalKeyboardKey.keyW): () => toggle(DeckSide.b),
    const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
        crossfader.nudge(-0.1),
    const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
        crossfader.nudge(0.1),
    const SingleActivator(LogicalKeyboardKey.arrowUp): () => master.nudge(0.05),
    const SingleActivator(LogicalKeyboardKey.arrowDown): () =>
        master.nudge(-0.05),
    const SingleActivator(LogicalKeyboardKey.keyA): () => settings.setAutoMix(
      enabled: !ref.read(mixSettingsControllerProvider).automix,
    ),
    const SingleActivator(LogicalKeyboardKey.keyR): () =>
        unawaited(gateway.resumeAutoMix()),
    const SingleActivator(LogicalKeyboardKey.keyS): () => settings.setShuffle(
      enabled: !ref.read(mixSettingsControllerProvider).shuffle,
    ),
    const SingleActivator(LogicalKeyboardKey.keyL): () => settings.setRepeat(
      enabled: !ref.read(mixSettingsControllerProvider).repeat,
    ),
    const SingleActivator(LogicalKeyboardKey.digit1): () => preset(0),
    const SingleActivator(LogicalKeyboardKey.digit2): () => preset(1),
    const SingleActivator(LogicalKeyboardKey.digit3): () => preset(2),
    const SingleActivator(LogicalKeyboardKey.digit4): () => preset(3),
    const SingleActivator(LogicalKeyboardKey.digit5): () => preset(4),
    const SingleActivator(LogicalKeyboardKey.keyO, meta: true): () =>
        unawaited(ref.read(libraryLoaderProvider).pickAndLoad()),
    const CharacterActivator('?'): () => unawaited(showShortcutHelp(context)),
  };
}

/// The help screen listing every shortcut.
Future<void> showShortcutHelp(BuildContext context) {
  return showDialog<void>(
    context: context,
    builder: (context) {
      final tokens = context.tokens;
      return AlertDialog(
        title: const Text('Raccourcis clavier'),
        content: SizedBox(
          width: 420,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (final shortcut in shortcutHelp)
                Padding(
                  padding: EdgeInsets.symmetric(vertical: tokens.space1),
                  child: Row(
                    children: [
                      SizedBox(
                        width: 96,
                        child: Text(
                          shortcut.keys,
                          style: TextStyle(
                            fontFamily: tokens.displayFamily,
                            color: tokens.neonCyan,
                          ),
                        ),
                      ),
                      Expanded(child: Text(shortcut.description)),
                    ],
                  ),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Fermer'),
          ),
        ],
      );
    },
  );
}
