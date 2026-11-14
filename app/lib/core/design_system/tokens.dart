import 'package:material_ui/material_ui.dart';

/// Design tokens of Platine: a dark "hardware controller" look inspired by
/// pro DJ software (graphite panels, blue deck A, orange deck B, RGB
/// waveforms). Widgets read them with `context.tokens`; no raw values live
/// in widgets.
@immutable
class PlatineTokens extends ThemeExtension<PlatineTokens> {
  const new({
    required this.background,
    required this.backgroundGlow,
    required this.surface,
    required this.panel,
    required this.panelRaised,
    required this.panelBorder,
    required this.inset,
    required this.glass,
    required this.glassBorder,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.deckA,
    required this.deckB,
    required this.neonMagenta,
    required this.neonCyan,
    required this.neonViolet,
    required this.waveLow,
    required this.waveMid,
    required this.waveHigh,
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

  /// The default dark theme.
  static const PlatineTokens neon = PlatineTokens(
    background: Color(0xFF050506),
    backgroundGlow: Color(0xFF101116),
    surface: Color(0xFF15161B),
    panel: Color(0xFF16171C),
    panelRaised: Color(0xFF22242B),
    panelBorder: Color(0xFF2C2E36),
    inset: Color(0xFF0B0B0E),
    glass: Color(0x14FFFFFF),
    glassBorder: Color(0x26FFFFFF),
    textPrimary: Color(0xFFEDEEF2),
    textSecondary: Color(0xFFB3B6C2),
    textMuted: Color(0xFF6E7180),
    deckA: Color(0xFF2E9BFF),
    deckB: Color(0xFFFF6A2B),
    neonMagenta: Color(0xFFFF3DCB),
    neonCyan: Color(0xFF2EE6FF),
    neonViolet: Color(0xFF8B5CFF),
    waveLow: Color(0xFFFF3A3A),
    waveMid: Color(0xFF3DFF7A),
    waveHigh: Color(0xFF3D8BFF),
    success: Color(0xFF3DFF8A),
    warning: Color(0xFFFFC43D),
    danger: Color(0xFFFF4D5E),
    space1: 4,
    space2: 8,
    space3: 12,
    space4: 16,
    space5: 24,
    space6: 32,
    radiusSmall: 4,
    radiusMedium: 8,
    radiusLarge: 12,
    blurSigma: 18,
    durationFast: Duration(milliseconds: 120),
    durationMedium: Duration(milliseconds: 260),
    durationSlow: Duration(milliseconds: 600),
    curveStandard: Curves.easeOutCubic,
    curveEmphasized: Curves.easeOutBack,
    displayFamily: 'SpaceGrotesk',
    minTouchTarget: 32,
  );

  final Color background;
  final Color backgroundGlow;
  final Color surface;

  /// Flat panel of a section (deck, mixer, browser).
  final Color panel;

  /// Raised elements on a panel (buttons, knob caps, headers).
  final Color panelRaised;
  final Color panelBorder;

  /// Recessed areas (waveforms, meters, fader slots).
  final Color inset;
  final Color glass;
  final Color glassBorder;
  final Color textPrimary;
  final Color textSecondary;
  final Color textMuted;

  /// Accents of the two decks.
  final Color deckA;
  final Color deckB;
  final Color neonMagenta;
  final Color neonCyan;
  final Color neonViolet;

  /// Colours of the low, mid and high bands in waveforms.
  final Color waveLow;
  final Color waveMid;
  final Color waveHigh;
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

  /// Font of the whole interface.
  final String displayFamily;

  /// Minimum size of interactive targets (Fitts's law, pointer input).
  final double minTouchTarget;

  /// Accent of each deck.
  Color deckColor({required bool isDeckA}) => isDeckA ? deckA : deckB;

  /// Brand gradient (deck A → deck B).
  LinearGradient get neonGradient => LinearGradient(colors: [deckA, deckB]);

  @override
  PlatineTokens copyWith({Color? deckA, Color? deckB}) {
    return PlatineTokens(
      background: background,
      backgroundGlow: backgroundGlow,
      surface: surface,
      panel: panel,
      panelRaised: panelRaised,
      panelBorder: panelBorder,
      inset: inset,
      glass: glass,
      glassBorder: glassBorder,
      textPrimary: textPrimary,
      textSecondary: textSecondary,
      textMuted: textMuted,
      deckA: deckA ?? this.deckA,
      deckB: deckB ?? this.deckB,
      neonMagenta: neonMagenta,
      neonCyan: neonCyan,
      neonViolet: neonViolet,
      waveLow: waveLow,
      waveMid: waveMid,
      waveHigh: waveHigh,
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
      deckA: Color.lerp(deckA, other.deckA, t),
      deckB: Color.lerp(deckB, other.deckB, t),
    );
  }
}

extension PlatineContext on BuildContext {
  PlatineTokens get tokens =>
      Theme.of(this).extension<PlatineTokens>() ?? PlatineTokens.neon;

  /// `true` when the user asked the system to reduce motion.
  bool get reduceMotion => MediaQuery.maybeDisableAnimationsOf(this) ?? false;
}
