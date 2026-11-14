import 'package:material_ui/material_ui.dart';

/// Design tokens of Platine ("club at night"): colours, gradients, spacing,
/// radii, typography and motion. Widgets read them with `context.tokens`;
/// no raw values live in widgets.
@immutable
class PlatineTokens extends ThemeExtension<PlatineTokens> {
  const new({
    required this.background,
    required this.backgroundGlow,
    required this.surface,
    required this.glass,
    required this.glassBorder,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.neonMagenta,
    required this.neonCyan,
    required this.neonViolet,
    required this.success,
    required this.warning,
    required this.danger,
    required this.space1,
    required this.space2,
    required this.space3,
    required this.space4,
    required this.space5,
    required this.space6,
    required this.radiusSmall,
    required this.radiusMedium,
    required this.radiusLarge,
    required this.blurSigma,
    required this.durationFast,
    required this.durationMedium,
    required this.durationSlow,
    required this.curveStandard,
    required this.curveEmphasized,
    required this.displayFamily,
    required this.minTouchTarget,
  });

  /// The dark neon theme.
  static const PlatineTokens neon = PlatineTokens(
    background: Color(0xFF07060D),
    backgroundGlow: Color(0xFF1B1035),
    surface: Color(0xFF12101E),
    glass: Color(0x1FFFFFFF),
    glassBorder: Color(0x33FFFFFF),
    textPrimary: Color(0xFFF5F3FF),
    textSecondary: Color(0xFFC4BFDD),
    textMuted: Color(0xFF8D88A8),
    neonMagenta: Color(0xFFFF3DCB),
    neonCyan: Color(0xFF2EE6FF),
    neonViolet: Color(0xFF8B5CFF),
    success: Color(0xFF3DFFA8),
    warning: Color(0xFFFFC43D),
    danger: Color(0xFFFF5470),
    space1: 4,
    space2: 8,
    space3: 12,
    space4: 16,
    space5: 24,
    space6: 32,
    radiusSmall: 8,
    radiusMedium: 16,
    radiusLarge: 28,
    blurSigma: 18,
    durationFast: Duration(milliseconds: 120),
    durationMedium: Duration(milliseconds: 260),
    durationSlow: Duration(milliseconds: 600),
    curveStandard: Curves.easeOutCubic,
    curveEmphasized: Curves.easeOutBack,
    displayFamily: 'SpaceGrotesk',
    minTouchTarget: 44,
  );

  final Color background;
  final Color backgroundGlow;
  final Color surface;
  final Color glass;
  final Color glassBorder;
  final Color textPrimary;
  final Color textSecondary;
  final Color textMuted;
  final Color neonMagenta;
  final Color neonCyan;
  final Color neonViolet;
  final Color success;
  final Color warning;
  final Color danger;

  final double space1;
  final double space2;
  final double space3;
  final double space4;
  final double space5;
  final double space6;

  final double radiusSmall;
  final double radiusMedium;
  final double radiusLarge;
  final double blurSigma;

  final Duration durationFast;
  final Duration durationMedium;
  final Duration durationSlow;
  final Curve curveStandard;
  final Curve curveEmphasized;

  /// Font of titles and counters.
  final String displayFamily;

  /// Minimum size of interactive targets (Fitts's law).
  final double minTouchTarget;

  /// Accent of each deck: cyan for A, magenta for B.
  Color deckColor({required bool isDeckA}) => isDeckA ? neonCyan : neonMagenta;

  /// Signature gradient (magenta → violet → cyan).
  LinearGradient get neonGradient =>
      LinearGradient(colors: [neonMagenta, neonViolet, neonCyan]);

  @override
  PlatineTokens copyWith({Color? neonMagenta, Color? neonCyan}) {
    return PlatineTokens(
      background: background,
      backgroundGlow: backgroundGlow,
      surface: surface,
      glass: glass,
      glassBorder: glassBorder,
      textPrimary: textPrimary,
      textSecondary: textSecondary,
      textMuted: textMuted,
      neonMagenta: neonMagenta ?? this.neonMagenta,
      neonCyan: neonCyan ?? this.neonCyan,
      neonViolet: neonViolet,
      success: success,
      warning: warning,
      danger: danger,
      space1: space1,
      space2: space2,
      space3: space3,
      space4: space4,
      space5: space5,
      space6: space6,
      radiusSmall: radiusSmall,
      radiusMedium: radiusMedium,
      radiusLarge: radiusLarge,
      blurSigma: blurSigma,
      durationFast: durationFast,
      durationMedium: durationMedium,
      durationSlow: durationSlow,
      curveStandard: curveStandard,
      curveEmphasized: curveEmphasized,
      displayFamily: displayFamily,
      minTouchTarget: minTouchTarget,
    );
  }

  @override
  PlatineTokens lerp(covariant PlatineTokens? other, double t) {
    if (other == null) return this;
    return copyWith(
      neonMagenta: Color.lerp(neonMagenta, other.neonMagenta, t),
      neonCyan: Color.lerp(neonCyan, other.neonCyan, t),
    );
  }
}

extension PlatineContext on BuildContext {
  PlatineTokens get tokens =>
      Theme.of(this).extension<PlatineTokens>() ?? PlatineTokens.neon;

  /// `true` when the user asked the system to reduce motion.
  bool get reduceMotion => MediaQuery.maybeDisableAnimationsOf(this) ?? false;
}
