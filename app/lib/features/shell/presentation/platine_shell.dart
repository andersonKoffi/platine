import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';
import 'package:platine/core/design_system/widgets.dart';
import 'package:platine/core/engine/application/engine_providers.dart';
import 'package:platine/core/engine/application/session_state.dart';
import 'package:platine/core/engine/domain/models.dart';
import 'package:platine/features/automix/presentation/automix_panel.dart';
import 'package:platine/features/decks/presentation/deck_view.dart';
import 'package:platine/features/library/application/library_controller.dart';
import 'package:platine/features/library/presentation/empty_state.dart';
import 'package:platine/features/library/presentation/library_panel.dart';
import 'package:platine/features/mixer/presentation/mixer_panel.dart';
import 'package:platine/features/settings/application/settings_controller.dart';
import 'package:platine/features/shell/application/drop_zone_provider.dart';
import 'package:platine/features/shell/presentation/shortcuts.dart';

/// Main screen: drop target, shortcuts, layout, notices.
class PlatineShell extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<PlatineShell> createState() => _PlatineShellState();
}

class _PlatineShellState extends ConsumerState<PlatineShell>
    with SingleTickerProviderStateMixin {
  bool _dragging = false;

  /// Light burst played when the first folder arrives.
  late final AnimationController _celebration = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1400),
  );

  @override
  void dispose() {
    _celebration.dispose();
    super.dispose();
  }

  Future<void> _onFolder(String? folder) async {
    setState(() => _dragging = false);
    if (folder == null) {
      ref
          .read(sessionStoreProvider.notifier)
          .notify('Dépose un dossier (ou un fichier) audio.');
      return;
    }
    final hasLibrary = ref.read(sessionStoreProvider).library.isNotEmpty;
    final mode = hasLibrary ? await _askMode() : FolderLoadMode.replace;
    if (mode == null) return;
    await ref.read(libraryLoaderProvider).load(folder, mode: mode);
  }

  Future<FolderLoadMode?> _askMode() {
    return showDialog<FolderLoadMode>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Nouveau dossier'),
        content: const Text('Que faire de la file actuelle ?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Annuler'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(FolderLoadMode.replace),
            child: const Text('Remplacer la file'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(FolderLoadMode.append),
            child: const Text('Ajouter à la file'),
          ),
        ],
      ),
    );
  }

  void _showNotice(Notice? notice) {
    if (notice == null || !mounted) return;
    final details = notice.details;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(notice.message),
          action: details == null
              ? null
              : SnackBarAction(
                  label: 'Détails',
                  onPressed: () => unawaited(_showDetails(details)),
                ),
        ),
      );
  }

  Future<void> _showDetails(String details) {
    return showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Détails'),
        content: SingleChildScrollView(child: SelectableText(details)),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Fermer'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    ref
      ..listen(
        sessionStoreProvider.select((s) => s.notice),
        (_, notice) => _showNotice(notice),
      )
      ..listen(sessionStoreProvider.select((s) => s.library.isNotEmpty), (
        before,
        now,
      ) {
        if (before == false && now && !context.reduceMotion) {
          _celebration.forward(from: 0);
        }
      });
    final startup = ref.watch(engineStartupProvider);
    final hasLibrary = ref.watch(
      sessionStoreProvider.select((s) => s.library.isNotEmpty),
    );

    return Scaffold(
      body: ref.watch(dropZoneProvider)(
        onEnter: () => setState(() => _dragging = true),
        onLeave: () => setState(() => _dragging = false),
        onFolder: _onFolder,
        child: CallbackShortcuts(
          bindings: buildShortcuts(context, ref),
          child: Focus(
            autofocus: true,
            child: DecoratedBox(
              decoration: BoxDecoration(color: tokens.background),
              child: Stack(
                children: [
                  Padding(
                    padding: EdgeInsets.all(tokens.space2),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const _TopBar(),
                        if (startup.hasError)
                          _EngineError(error: startup.error),
                        SizedBox(height: tokens.space2),
                        Expanded(
                          child: _MixLayout(
                            browser: hasLibrary
                                ? const LibraryPanel()
                                : EmptyLibrary(dragging: _dragging),
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (_dragging && hasLibrary) _DropOverlay(tokens: tokens),
                  Positioned.fill(
                    child: IgnorePointer(
                      child: AnimatedBuilder(
                        animation: _celebration,
                        builder: (context, _) =>
                            _Celebration(t: _celebration.value, tokens: tokens),
                      ),
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

class _TopBar extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    final textTheme = Theme.of(context).textTheme;
    final mode = ref.watch(sessionStoreProvider.select((s) => s.mode));
    final transition = ref.watch(
      sessionStoreProvider.select((s) => s.transition != null),
    );
    final (status, color) = switch (mode) {
      AutoMixMode.running => (
        transition ? 'AUTOMIX · FONDU EN COURS' : 'AUTOMIX · EN MARCHE',
        tokens.success,
      ),
      AutoMixMode.suspended => ('AUTOMIX · SUSPENDU', tokens.warning),
      AutoMixMode.off => ('AUTOMIX · ARRÊTÉ', tokens.textMuted),
    };
    return SizedBox(
      height: 34,
      child: Row(
        children: [
          SizedBox(width: tokens.space2),
          ShaderMask(
            shaderCallback: tokens.neonGradient.createShader,
            child: Text(
              'Platine',
              style: textTheme.headlineSmall?.copyWith(
                color: Colors.white,
                letterSpacing: 1,
              ),
            ),
          ),
          SizedBox(width: tokens.space5),
          Container(
            padding: EdgeInsets.symmetric(
              horizontal: tokens.space2,
              vertical: 3,
            ),
            decoration: BoxDecoration(
              color: tokens.inset,
              borderRadius: BorderRadius.circular(tokens.radiusSmall),
              border: Border.all(color: tokens.panelBorder),
            ),
            child: Row(
              children: [
                Icon(Icons.circle, size: 8, color: color),
                SizedBox(width: tokens.space2),
                Text(
                  status,
                  style: textTheme.labelSmall?.copyWith(color: color),
                ),
              ],
            ),
          ),
          const Spacer(),
          IconButton(
            tooltip: 'Choisir un dossier (⌘O)',
            onPressed: () =>
                unawaited(ref.read(libraryLoaderProvider).pickAndLoad()),
            icon: Icon(
              Icons.folder_open_rounded,
              size: 20,
              color: tokens.textSecondary,
            ),
          ),
          IconButton(
            tooltip: 'Raccourcis clavier (?)',
            onPressed: () => unawaited(showShortcutHelp(context)),
            icon: Icon(
              Icons.keyboard_rounded,
              size: 20,
              color: tokens.textSecondary,
            ),
          ),
        ],
      ),
    );
  }
}

class _EngineError extends ConsumerWidget {
  const new({required this.error});

  final Object? error;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tokens = context.tokens;
    return Padding(
      padding: EdgeInsets.only(top: tokens.space3),
      child: PanelBox(
        accent: tokens.danger,
        child: Row(
          children: [
            Icon(Icons.volume_off_rounded, color: tokens.danger),
            SizedBox(width: tokens.space3),
            Expanded(
              child: Text(
                'Sortie audio indisponible : $error',
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            TextButton(
              onPressed: () => ref.invalidate(engineStartupProvider),
              child: const Text('Réessayer'),
            ),
          ],
        ),
      ),
    );
  }
}

/// Waveforms on top, decks around the mixer, browser and auto-mix below.
class _MixLayout extends StatelessWidget {
  const new({required this.browser});

  final Widget browser;

  /// Width of the mixer and of the auto-mix panel.
  static const double centerWidth = 300;

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final gap = SizedBox(width: tokens.space2, height: tokens.space2);
    return LayoutBuilder(
      builder: (context, constraints) {
        final waveHeight = (constraints.maxHeight * 0.085).clamp(44.0, 84.0);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            PanelBox(
              padding: EdgeInsets.all(tokens.space1),
              child: Column(
                children: [
                  SizedBox(
                    height: waveHeight,
                    child: const DeckWaveform(side: DeckSide.a),
                  ),
                  SizedBox(height: tokens.space1),
                  SizedBox(
                    height: waveHeight,
                    child: const DeckWaveform(side: DeckSide.b),
                  ),
                ],
              ),
            ),
            gap,
            Expanded(
              flex: 11,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Expanded(child: DeckView(side: DeckSide.a)),
                  gap,
                  const SizedBox(width: centerWidth, child: MixerPanel()),
                  gap,
                  const Expanded(child: DeckView(side: DeckSide.b)),
                ],
              ),
            ),
            gap,
            Expanded(
              flex: 8,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(child: browser),
                  gap,
                  const SizedBox(width: centerWidth, child: AutoMixPanel()),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}

class _DropOverlay extends StatelessWidget {
  const new({required this.tokens});

  final PlatineTokens tokens;

  @override
  Widget build(BuildContext context) {
    return Positioned.fill(
      child: IgnorePointer(
        child: ColoredBox(
          color: tokens.background.withValues(alpha: 0.7),
          child: Center(
            child: Text(
              'Dépose pour ajouter ou remplacer la file',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
          ),
        ),
      ),
    );
  }
}

class _Celebration extends StatelessWidget {
  const new({required this.t, required this.tokens});

  final double t;
  final PlatineTokens tokens;

  @override
  Widget build(BuildContext context) {
    if (t == 0 || t == 1) return const SizedBox.shrink();
    final fade = 1 - t;
    return DecoratedBox(
      decoration: BoxDecoration(
        gradient: RadialGradient(
          radius: 0.2 + 1.2 * t,
          colors: [
            tokens.deckB.withValues(alpha: 0.3 * fade),
            tokens.deckA.withValues(alpha: 0.2 * fade),
            Colors.transparent,
          ],
        ),
      ),
    );
  }
}
