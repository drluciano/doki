import 'package:flutter/material.dart';

import 'motion.dart';

/// Proposed tokens from docs/design.md: warm cream, white cards, ink text,
/// lavender accent, mint secondary. Soft corners and restrained shadows.
class DokiColors {
  static const cream = Color(0xFFFAF7F2);
  static const ink = Color(0xFF272335);
  static const muted = Color(0xFF655D70);
  static const lavender = Color(0xFF7354A3);
  static const lavenderSoft = Color(0xFFEDE6F7);
  static const mint = Color(0xFFD8F1E8);
  static const mintInk = Color(0xFF1F6B55);
  static const attention = Color(0xFFFFF0D6);
  static const attentionInk = Color(0xFF7A4B00);
  static const error = Color(0xFFB3261E);
  static const errorSoft = Color(0xFFFBE4E2);
}

ThemeData dokiTheme() {
  final scheme = ColorScheme.fromSeed(
    seedColor: DokiColors.lavender,
    primary: DokiColors.lavender,
    surface: DokiColors.cream,
    onSurface: DokiColors.ink,
    secondaryContainer: DokiColors.mint,
    onSecondaryContainer: DokiColors.mintInk,
    error: DokiColors.error,
  );
  final base = ThemeData(colorScheme: scheme, useMaterial3: true);
  return base.copyWith(
    scaffoldBackgroundColor: DokiColors.cream,
    // Soft lavender tints under the focus border instead of Material's grey.
    focusColor: DokiColors.lavender.withValues(alpha: 0.08),
    hoverColor: DokiColors.lavender.withValues(alpha: 0.05),
    // One transition on every platform, so screens feel the same everywhere.
    pageTransitionsTheme: PageTransitionsTheme(
      builders: {for (final p in TargetPlatform.values) p: const DokiPageTransitionsBuilder()},
    ),
    textTheme: base.textTheme.apply(bodyColor: DokiColors.ink, displayColor: DokiColors.ink),
    cardTheme: CardThemeData(
      color: Colors.white,
      elevation: 1,
      shadowColor: DokiColors.ink.withValues(alpha: 0.12),
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
      fillColor: Colors.white,
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(16)),
    ),
    navigationBarTheme: const NavigationBarThemeData(
      backgroundColor: Colors.white,
      indicatorColor: DokiColors.lavenderSoft,
    ),
    navigationRailTheme: const NavigationRailThemeData(
      backgroundColor: Colors.white,
      indicatorColor: DokiColors.lavenderSoft,
    ),
  );
}
