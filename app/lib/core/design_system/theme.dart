import 'package:material_ui/material_ui.dart';
import 'package:platine/core/design_system/tokens.dart';

/// Builds the app theme from the design tokens.
ThemeData buildPlatineTheme([PlatineTokens tokens = PlatineTokens.neon]) {
  final scheme =
      ColorScheme.fromSeed(
        seedColor: tokens.deckA,
        brightness: Brightness.dark,
      ).copyWith(
        primary: tokens.deckA,
        secondary: tokens.deckB,
        tertiary: tokens.neonViolet,
        surface: tokens.panel,
        onSurface: tokens.textPrimary,
        error: tokens.danger,
        outline: tokens.panelBorder,
      );
  final base = ThemeData(
    colorScheme: scheme,
    scaffoldBackgroundColor: tokens.background,
    fontFamily: tokens.displayFamily,
    visualDensity: VisualDensity.compact,
    useMaterial3: true,
  );
  final display = TextStyle(
    fontFamily: tokens.displayFamily,
    color: tokens.textPrimary,
    fontWeight: FontWeight.w700,
  );
  final label = TextStyle(
    fontFamily: tokens.displayFamily,
    color: tokens.textMuted,
    fontWeight: FontWeight.w700,
    fontSize: 10,
    letterSpacing: 1.2,
  );
  return base.copyWith(
    extensions: [tokens],
    textTheme: base.textTheme
        .apply(
          bodyColor: tokens.textPrimary,
          displayColor: tokens.textPrimary,
          fontFamily: tokens.displayFamily,
        )
        .copyWith(
          displaySmall: display.copyWith(fontSize: 30, letterSpacing: -0.5),
          headlineSmall: display.copyWith(fontSize: 20),
          titleMedium: display.copyWith(fontSize: 15),
          titleSmall: display.copyWith(fontSize: 13),
          labelSmall: label,
          bodyMedium: TextStyle(
            fontFamily: tokens.displayFamily,
            fontSize: 13,
            color: tokens.textPrimary,
          ),
          bodySmall: TextStyle(
            fontFamily: tokens.displayFamily,
            fontSize: 12,
            color: tokens.textSecondary,
          ),
        ),
    dividerTheme: DividerThemeData(
      color: tokens.panelBorder,
      thickness: 1,
      space: 1,
    ),
    sliderTheme: SliderThemeData(
      trackHeight: 3,
      activeTrackColor: tokens.deckA,
      inactiveTrackColor: tokens.panelBorder,
      thumbColor: tokens.textPrimary,
      overlayColor: tokens.deckA.withValues(alpha: 0.16),
      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 7),
    ),
    inputDecorationTheme: InputDecorationTheme(
      isDense: true,
      filled: true,
      fillColor: tokens.inset,
      contentPadding: EdgeInsets.symmetric(
        horizontal: tokens.space3,
        vertical: tokens.space2,
      ),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        borderSide: BorderSide(color: tokens.panelBorder),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        borderSide: BorderSide(color: tokens.panelBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        borderSide: BorderSide(color: tokens.deckA),
      ),
      hintStyle: TextStyle(color: tokens.textMuted, fontSize: 13),
    ),
    tooltipTheme: TooltipThemeData(
      waitDuration: tokens.durationSlow,
      decoration: BoxDecoration(
        color: tokens.panelRaised,
        borderRadius: BorderRadius.circular(tokens.radiusSmall),
        border: Border.all(color: tokens.panelBorder),
      ),
      textStyle: TextStyle(
        color: tokens.textPrimary,
        fontFamily: tokens.displayFamily,
        fontSize: 12,
      ),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: tokens.panelRaised,
      contentTextStyle: TextStyle(
        color: tokens.textPrimary,
        fontFamily: tokens.displayFamily,
      ),
      actionTextColor: tokens.deckA,
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: tokens.panel,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(tokens.radiusMedium),
        side: BorderSide(color: tokens.panelBorder),
      ),
    ),
  );
}

/// Tabular figures, for counters that must not jitter.
const List<FontFeature> tabularFigures = [FontFeature.tabularFigures()];
