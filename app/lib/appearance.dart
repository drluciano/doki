import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';

import 'theme.dart';

/// Which palette the app uses. [system] follows the device's light or dark
/// setting; [oled] is dark with true black behind everything.
enum ThemeChoice { system, light, dark, oled }

/// A color for the focus border, with a shade for light and for dark
/// palettes so it keeps its contrast on both.
class HaloAccent {
  const HaloAccent(this.id, this.label, this.light, this.dark);
  final String id;
  final String label;
  final Color light;
  final Color dark;

  Color resolve(Brightness b) => b == Brightness.dark ? dark : light;

  static const lavender = HaloAccent('lavender', 'Lavender', Color(0xFF7354A3), Color(0xFFC3A8F5));
  static const all = [
    lavender,
    HaloAccent('mint', 'Mint', Color(0xFF1F8A66), Color(0xFF8FE0C2)),
    HaloAccent('sky', 'Sky', Color(0xFF2F78C4), Color(0xFF93C6F6)),
    HaloAccent('peach', 'Peach', Color(0xFFC9602F), Color(0xFFF6B494)),
    HaloAccent('rose', 'Rose', Color(0xFFC2416F), Color(0xFFF5A3C4)),
    HaloAccent('gold', 'Gold', Color(0xFF9A7400), Color(0xFFF0D06E)),
  ];

  static HaloAccent byId(String? id) => all.firstWhere((a) => a.id == id, orElse: () => lavender);
}

/// The person's look-and-feel choices: theme and focus border color.
///
/// Stored as a small JSON file next to the vault (`appearance.json` in the
/// app's support folder) and read once at start. Nothing here touches saves.
/// A missing or unreadable file means the defaults: follow the system,
/// lavender border. Without a [file] (tests) choices live in memory only.
class Appearance extends ChangeNotifier {
  Appearance({this.file, this._theme = ThemeChoice.system, HaloAccent? accent})
    : _accent = accent ?? HaloAccent.lavender;

  final File? file;
  ThemeChoice _theme;
  HaloAccent _accent;

  ThemeChoice get theme => _theme;
  HaloAccent get accent => _accent;

  set theme(ThemeChoice value) {
    if (value == _theme) return;
    _theme = value;
    _changed();
  }

  set accent(HaloAccent value) {
    if (value.id == _accent.id) return;
    _accent = value;
    _changed();
  }

  /// The dark palette for the current choice; used when the theme in effect
  /// is dark (Dark, OLED, or Follow system on a device set to dark).
  DokiPalette get darkPalette => _theme == ThemeChoice.oled ? DokiPalette.oled : DokiPalette.dark;

  ThemeMode get themeMode => switch (_theme) {
    ThemeChoice.system => ThemeMode.system,
    ThemeChoice.light => ThemeMode.light,
    ThemeChoice.dark || ThemeChoice.oled => ThemeMode.dark,
  };

  static Future<Appearance> load(File file) async {
    try {
      final json = jsonDecode(await file.readAsString()) as Map<String, dynamic>;
      return Appearance(
        file: file,
        theme: ThemeChoice.values.asNameMap()[json['theme']] ?? ThemeChoice.system,
        accent: HaloAccent.byId(json['accent'] as String?),
      );
    } catch (_) {
      return Appearance(file: file);
    }
  }

  // Saves run one after another, so two quick changes never write the
  // temporary file at the same time.
  Future<void> _saving = Future.value();

  /// Completes once every change so far has been written.
  Future<void> get saved => _saving;

  void _changed() {
    notifyListeners();
    _saving = _saving.then((_) => _save());
  }

  Future<void> _save() async {
    final f = file;
    if (f == null) return;
    try {
      // Write a sibling and rename it over, so a crash never leaves half a file.
      final tmp = File('${f.path}.tmp');
      await tmp.writeAsString(jsonEncode({'theme': _theme.name, 'accent': _accent.id}));
      await tmp.rename(f.path);
    } catch (_) {
      // Appearance is a convenience; failing to remember it is not an error
      // worth interrupting anyone for.
    }
  }
}

/// Makes [Appearance] available to the settings screen.
class AppearanceScope extends InheritedNotifier<Appearance> {
  const AppearanceScope({super.key, required Appearance appearance, required super.child})
    : super(notifier: appearance);

  static Appearance of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<AppearanceScope>()!.notifier!;
}

/// Keeps [DokiColors.palette] in step with the theme in effect. When the
/// palette changes it marks every widget below for rebuild, so widgets that
/// paint with Doki's colors without reading the theme pick up the change too.
class PaletteSync extends StatefulWidget {
  const PaletteSync({super.key, required this.darkPalette, required this.child});
  final DokiPalette darkPalette;
  final Widget child;

  @override
  State<PaletteSync> createState() => _PaletteSyncState();
}

class _PaletteSyncState extends State<PaletteSync> {
  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final palette = dark ? widget.darkPalette : DokiPalette.light;
    if (!identical(DokiColors.palette, palette)) {
      // Set before the children build in this frame, then make sure the
      // rest of the tree (including const widgets) rebuilds next frame.
      DokiColors.palette = palette;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        void rebuild(Element e) {
          e.markNeedsBuild();
          e.visitChildren(rebuild);
        }

        (context as Element).visitChildren(rebuild);
      });
    }
    return widget.child;
  }
}
