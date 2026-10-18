import 'package:flutter/material.dart';

import 'motion.dart';

/// One set of Doki colors. The light set is the design tokens from
/// docs/design.md: warm cream, white cards, ink text, lavender accent, mint
/// secondary. The dark sets keep the same roles with soft, low-glare tones;
/// OLED uses true black behind everything so the screen's pixels turn off.
class DokiPalette {
  const DokiPalette({
    required this.brightness,
    required this.cream,
    required this.surface,
    required this.ink,
    required this.muted,
    required this.lavender,
    required this.lavenderSoft,
    required this.mint,
    required this.mintInk,
    required this.attention,
    required this.attentionInk,
    required this.error,
    required this.errorSoft,
  });

  final Brightness brightness;

  /// Page background.
  final Color cream;

  /// Cards, fields and navigation.
  final Color surface;
  final Color ink;
  final Color muted;
  final Color lavender;
  final Color lavenderSoft;
  final Color mint;
  final Color mintInk;
  final Color attention;
  final Color attentionInk;
  final Color error;
  final Color errorSoft;

  static const light = DokiPalette(
    brightness: Brightness.light,
    cream: Color(0xFFFAF7F2),
    surface: Color(0xFFFFFFFF),
    ink: Color(0xFF272335),
    muted: Color(0xFF655D70),
    lavender: Color(0xFF7354A3),
    lavenderSoft: Color(0xFFEDE6F7),
    mint: Color(0xFFD8F1E8),
    mintInk: Color(0xFF1F6B55),
    attention: Color(0xFFFFF0D6),
    attentionInk: Color(0xFF7A4B00),
    error: Color(0xFFB3261E),
    errorSoft: Color(0xFFFBE4E2),
  );

  static const dark = DokiPalette(
    brightness: Brightness.dark,
    cream: Color(0xFF17151C),
    surface: Color(0xFF221F29),
    ink: Color(0xFFEDE9F3),
    muted: Color(0xFFADA5B8),
    lavender: Color(0xFFC3A8F5),
    lavenderSoft: Color(0xFF372D4A),
    mint: Color(0xFF1C3B32),
    mintInk: Color(0xFF94DEC4),
    attention: Color(0xFF3E2F14),
    attentionInk: Color(0xFFF3C47E),
    error: Color(0xFFF2B8B5),
    errorSoft: Color(0xFF4A2523),
  );

  static const oled = DokiPalette(
    brightness: Brightness.dark,
    cream: Color(0xFF000000),
    surface: Color(0xFF111015),
    ink: Color(0xFFEDE9F3),
    muted: Color(0xFFA59DB0),
    lavender: Color(0xFFC3A8F5),
    lavenderSoft: Color(0xFF2B2339),
    mint: Color(0xFF142E26),
    mintInk: Color(0xFF94DEC4),
    attention: Color(0xFF312410),
    attentionInk: Color(0xFFF3C47E),
    error: Color(0xFFF2B8B5),
    errorSoft: Color(0xFF3D1E1C),
  );
}

/// The colors of the theme in use, for widgets that paint with Doki's own
/// tokens. `DokiApp` sets [palette] to match the active theme and rebuilds
/// the tree when it changes, so these are never stale for more than a frame.
class DokiColors {
  static DokiPalette palette = DokiPalette.light;

  static Color get cream => palette.cream;
  static Color get surface => palette.surface;
  static Color get ink => palette.ink;
  static Color get muted => palette.muted;
  static Color get lavender => palette.lavender;
  static Color get lavenderSoft => palette.lavenderSoft;
  static Color get mint => palette.mint;
  static Color get mintInk => palette.mintInk;
  static Color get attention => palette.attention;
  static Color get attentionInk => palette.attentionInk;
  static Color get error => palette.error;
  static Color get errorSoft => palette.errorSoft;
}

/// The Material theme for one palette. Widgets use Doki's colors through
/// [DokiColors]; this maps them onto Material's roles.
ThemeData dokiTheme([DokiPalette p = DokiPalette.light]) {
  final scheme = ColorScheme.fromSeed(
    seedColor: p.lavender,
    brightness: p.brightness,
    primary: p.lavender,
    surface: p.cream,
    onSurface: p.ink,
    secondaryContainer: p.mint,
    onSecondaryContainer: p.mintInk,
    error: p.error,
  );
  final base = ThemeData(colorScheme: scheme, useMaterial3: true);
  return base.copyWith(
    scaffoldBackgroundColor: p.cream,
    // Soft lavender tints under the focus border instead of Material's grey.
    focusColor: p.lavender.withValues(alpha: 0.08),
    hoverColor: p.lavender.withValues(alpha: 0.05),
    // One transition on every platform, so screens feel the same everywhere.
    pageTransitionsTheme: PageTransitionsTheme(
      builders: {for (final p in TargetPlatform.values) p: const DokiPageTransitionsBuilder()},
    ),
    textTheme: base.textTheme.apply(bodyColor: p.ink, displayColor: p.ink),
    cardTheme: CardThemeData(
      color: p.surface,
      elevation: 1,
      shadowColor: p.ink.withValues(alpha: 0.12),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      margin: const EdgeInsets.symmetric(vertical: 6),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(minimumSize: const Size(48, 48)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: p.surface,
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(16)),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: p.surface,
      indicatorColor: p.lavenderSoft,
    ),
    navigationRailTheme: NavigationRailThemeData(
      backgroundColor: p.surface,
      indicatorColor: p.lavenderSoft,
    ),
  );
}
