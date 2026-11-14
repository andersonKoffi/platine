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

/// Width under which the library moves below the decks.
const double wideLayoutMinWidth = 1280;

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
              decoration: BoxDecoration(
                gradient: RadialGradient(
                  center: const Alignment(0, -1.2),
                  radius: 1.4,
                  colors: [tokens.backgroundGlow, tokens.background],
                ),
              ),
              child: Stack(
                children: [
                  Padding(
                    padding: EdgeInsets.all(tokens.space4),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const _Header(),
                        if (startup.hasError)
                          _EngineError(error: startup.error),
                        SizedBox(height: tokens.space3),
                        Expanded(
                          child: hasLibrary
                              ? const _MixLayout()
                              : Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.stretch,
                                  children: [
                                    const AutoMixPanel(),
                                    SizedBox(height: tokens.space3),
                                    Expanded(
                                      child: EmptyLibrary(dragging: _dragging),
                                    ),
                                  ],
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

class _Header extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    return Row(
      children: [
        ShaderMask(
          shaderCallback: tokens.neonGradient.createShader,
          child: Text(
            'Platine',
            style: Theme.of(context).textTheme.displaySmall
                ?.copyWith(color: Colors.white),
          ),
        ),
        const Spacer(),
        Consumer(
          builder: (context, ref, _) => IconButton(
            tooltip: 'Choisir un dossier (⌘O)',
            onPressed: () =>
                unawaited(ref.read(libraryLoaderProvider).pickAndLoad()),
            icon: const Icon(Icons.folder_open_rounded),
          ),
        ),
        IconButton(
          tooltip: 'Raccourcis clavier (?)',
          onPressed: () => unawaited(showShortcutHelp(context)),
          icon: const Icon(Icons.keyboard_rounded),
        ),
      ],
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
      child: GlassPanel(
        glow: tokens.danger,
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

/// Decks and mixer, auto-mix bar, library (beside or below).
class _MixLayout extends StatelessWidget {
  const new();

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final gap = SizedBox(width: tokens.space3, height: tokens.space3);
    final decks = Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Expanded(flex: 5, child: DeckView(side: DeckSide.a)),
        gap,
        const Expanded(flex: 3, child: MixerPanel()),
        gap,
        const Expanded(flex: 5, child: DeckView(side: DeckSide.b)),
      ],
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= wideLayoutMinWidth) {
          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(
                flex: 3,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(child: decks),
                    gap,
                    const AutoMixPanel(),
                  ],
                ),
              ),
              gap,
              const Expanded(flex: 2, child: LibraryPanel()),
            ],
          );
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(flex: 3, child: decks),
            gap,
            const AutoMixPanel(),
            gap,
            const Expanded(flex: 2, child: LibraryPanel()),
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
              style: Theme.of(context).textTheme.displaySmall,
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
            tokens.neonMagenta.withValues(alpha: 0.35 * fade),
            tokens.neonCyan.withValues(alpha: 0.2 * fade),
            Colors.transparent,
          ],
        ),
      ),
    );
  }
}
