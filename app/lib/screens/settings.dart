import 'package:flutter/material.dart';

import '../appearance.dart';
import '../motion.dart';
import '../theme.dart';
import '../widgets/common.dart';
import 'help.dart';

/// Appearance choices and the how-tos. Nothing on this screen touches saves.
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final appearance = AppearanceScope.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('Settings'), backgroundColor: Colors.transparent),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
        children: [
          ContentWidth(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const _SectionTitle('Appearance'),
                Entrance(
                  child: Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Theme', style: Theme.of(context).textTheme.titleMedium),
                          const SizedBox(height: 12),
                          Wrap(
                            spacing: 12,
                            runSpacing: 12,
                            children: [
                              for (final choice in ThemeChoice.values)
                                ThemeTile(
                                  choice: choice,
                                  selected: appearance.theme == choice,
                                  onTap: () => appearance.theme = choice,
                                ),
                            ],
                          ),
                          const SizedBox(height: 24),
                          Text('Border color', style: Theme.of(context).textTheme.titleMedium),
                          const SizedBox(height: 4),
                          Text(
                            'The soft border that shows where you are with a mouse, keyboard or controller.',
                            style: Theme.of(context).textTheme.bodyMedium
                                ?.copyWith(color: DokiColors.muted),
                          ),
                          const SizedBox(height: 12),
                          Wrap(
                            spacing: 10,
                            runSpacing: 10,
                            children: [
                              for (final accent in HaloAccent.all)
                                AccentSwatch(
                                  accent: accent,
                                  selected: appearance.accent.id == accent.id,
                                  onTap: () => appearance.accent = accent,
                                ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                const _SectionTitle('Help and how-tos'),
                for (var i = 0; i < helpTopics.length; i++)
                  Entrance(
                    delay: Duration(milliseconds: 30 * (i + 1)),
                    child: HelpTopicCard(topic: helpTopics[i]),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 16, 4, 8),
      child: Semantics(
        header: true,
        child: Text(
          text,
          style: Theme.of(context).textTheme.labelLarge?.copyWith(color: DokiColors.muted),
        ),
      ),
    );
  }
}

String themeLabel(ThemeChoice c) => switch (c) {
  ThemeChoice.system => 'Follow system',
  ThemeChoice.light => 'Light',
  ThemeChoice.dark => 'Dark',
  ThemeChoice.oled => 'OLED',
};

/// A small preview of a theme: its page, a card and the accent, drawn with
/// that theme's own colors. Follow system shows light and dark side by side.
class ThemeTile extends StatelessWidget {
  const ThemeTile({super.key, required this.choice, required this.selected, required this.onTap});
  final ThemeChoice choice;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final reduced = DokiMotion.reduced(context);
    Widget preview(DokiPalette p) => Expanded(
      child: ColoredBox(
        color: p.cream,
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                height: 22,
                decoration: BoxDecoration(color: p.surface, borderRadius: BorderRadius.circular(7)),
              ),
              const SizedBox(height: 6),
              Container(
                width: 26,
                height: 8,
                decoration: BoxDecoration(
                  color: p.lavender,
                  borderRadius: BorderRadius.circular(4),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    final palettes = switch (choice) {
      ThemeChoice.system => [DokiPalette.light, DokiPalette.dark],
      ThemeChoice.light => [DokiPalette.light],
      ThemeChoice.dark => [DokiPalette.dark],
      ThemeChoice.oled => [DokiPalette.oled],
    };
    return Semantics(
      button: true,
      selected: selected,
      label: '${themeLabel(choice)} theme',
      onTap: onTap,
      excludeSemantics: true,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(18),
          onTap: onTap,
          child: SizedBox(
            width: 112,
            child: Padding(
              padding: const EdgeInsets.all(6),
              child: Column(
                children: [
                  AnimatedContainer(
                    duration: reduced ? Duration.zero : DokiMotion.quick,
                    curve: DokiMotion.enter,
                    height: 64,
                    clipBehavior: Clip.antiAlias,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: selected
                            ? DokiColors.lavender
                            : DokiColors.muted.withValues(alpha: 0.3),
                        width: selected ? 2.5 : 1,
                      ),
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [for (final p in palettes) preview(p)],
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    themeLabel(choice),
                    style: TextStyle(
                      color: selected ? DokiColors.ink : DokiColors.muted,
                      fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
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

/// A round color choice for the focus border, with a check when chosen.
class AccentSwatch extends StatelessWidget {
  const AccentSwatch({
    super.key,
    required this.accent,
    required this.selected,
    required this.onTap,
  });
  final HaloAccent accent;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final color = accent.resolve(Theme.of(context).brightness);
    final reduced = DokiMotion.reduced(context);
    final onColor = ThemeData.estimateBrightnessForColor(color) == Brightness.dark
        ? Colors.white
        : Colors.black;
    return Semantics(
      button: true,
      selected: selected,
      label: '${accent.label} border',
      onTap: onTap,
      excludeSemantics: true,
      child: Material(
        color: color,
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: onTap,
          child: SizedBox.square(
            dimension: 44,
            child: AnimatedScale(
              scale: selected ? 1 : 0,
              duration: reduced ? Duration.zero : DokiMotion.quick,
              curve: Curves.easeOutBack,
              child: Icon(Icons.check, color: onColor, size: 22),
            ),
          ),
        ),
      ),
    );
  }
}
