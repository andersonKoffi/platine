import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Builds the app theme from the design tokens.
ThemeData buildPlatineTheme([PlatineTokens tokens = PlatineTokens.neon]) {
  final scheme = ColorScheme.fromSeed(
    seedColor: tokens.neonViolet,
    brightness: Brightness.dark,
  ).copyWith(
    primary: tokens.neonCyan,
    secondary: tokens.neonMagenta,
    tertiary: tokens.neonViolet,
    surface: tokens.surface,
    onSurface: tokens.textPrimary,
    error: tokens.danger,
  );
  final base = ThemeData(
    colorScheme: scheme,
    scaffoldBackgroundColor: tokens.background,
    useMaterial3: true,
  );
  final display = TextStyle(
    fontFamily: tokens.displayFamily,
    color: tokens.textPrimary,
    fontWeight: FontWeight.w700,
  );
  return base.copyWith(
    extensions: [tokens],
    textTheme: base.textTheme
        .apply(bodyColor: tokens.textPrimary, displayColor: tokens.textPrimary)
        .copyWith(
          displaySmall: display.copyWith(fontSize: 34, letterSpacing: -0.5),
          headlineSmall: display.copyWith(fontSize: 22),
          titleMedium: display.copyWith(fontSize: 16),
        ),
    sliderTheme: SliderThemeData(
      trackHeight: 4,
      activeTrackColor: tokens.neonCyan,
      inactiveTrackColor: tokens.glassBorder,
      thumbColor: tokens.textPrimary,
      overlayColor: tokens.neonCyan.withValues(alpha: 0.16),
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: tokens.durationSlow,
      decoration: BoxDecoration(
        color: tokens.surface,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        border: Border.all(color: tokens.glassBorder),
      ),
      textStyle: TextStyle(color: tokens.textPrimary),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: tokens.surface,
      contentTextStyle: TextStyle(color: tokens.textPrimary),
      actionTextColor: tokens.neonCyan,
    ),
  );
}

/// Tabular figures, for counters that must not jitter.
const List<FontFeature> tabularFigures = [FontFeature.tabularFigures()];
