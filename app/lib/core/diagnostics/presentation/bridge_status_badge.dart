import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:platine/core/diagnostics/application/diagnostics_providers.dart';

/// Small badge showing whether the Rust engine answers.
class BridgeStatusBadge extends ConsumerWidget {
  const BridgeStatusBadge({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(bridgeStatusProvider);
    final colors = Theme.of(context).colorScheme;
    final (label, color) = switch (status) {
      BridgeStatus.connected => ('Engine: pong', colors.primary),
      BridgeStatus.unexpectedAnswer => (
        'Engine: unexpected answer',
        colors.error,
      ),
    };
    return Semantics(
      label: label,
      child: Chip(
        avatar: Icon(Icons.circle, size: 12, color: color),
        label: Text(label),
      ),
    );
  }
}
