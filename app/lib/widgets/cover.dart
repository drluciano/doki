import 'package:flutter/material.dart';

import '../art.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';

/// Corner radius of library covers. The focus border follows it.
const coverRadius = 16.0;

/// A game's cover: its art, or a soft gradient with its initials.
class GameCover extends StatelessWidget {
  const GameCover({super.key, required this.gameId, required this.name});
  final String gameId;
  final String name;

  @override
  Widget build(BuildContext context) {
    final art = AppScope.of(context).art;
    return ListenableBuilder(
      listenable: art,
      builder: (context, _) {
        final file = art.image(gameId, ArtKind.cover);
        return AnimatedSwitcher(
          duration: const Duration(milliseconds: 280),
          child: file == null
              ? CoverPlaceholder(name: name, key: const ValueKey('placeholder'))
              : Image.file(
                  file,
                  key: ValueKey(file.path),
                  fit: BoxFit.cover,
                  width: double.infinity,
                  height: double.infinity,
                  cacheWidth: 600,
                  errorBuilder: (_, _, _) => CoverPlaceholder(name: name),
                ),
        );
      },
    );
  }
}

/// Stand-in cover: a pastel gradient picked from the name and the initials
/// in a bubble. The name is in the caption under the cover.
class CoverPlaceholder extends StatelessWidget {
  const CoverPlaceholder({super.key, required this.name});
  final String name;

  static const _gradients = [
    [Color(0xFFEDE6F7), Color(0xFFD9CCF0)],
    [Color(0xFFD8F1E8), Color(0xFFBFE6D8)],
    [Color(0xFFFFF0D6), Color(0xFFFFDFB0)],
    [Color(0xFFFBE4EC), Color(0xFFF4C9D8)],
    [Color(0xFFE2EEFB), Color(0xFFC8DDF5)],
  ];

  @override
  Widget build(BuildContext context) {
    final words = name.trim().split(RegExp(r'\s+')).where((w) => w.isNotEmpty).toList();
    final initials = words.isEmpty
        ? '?'
        : words.take(2).map((w) => w.characters.first.toUpperCase()).join();
    final colors = _gradients[name.hashCode.abs() % _gradients.length];
    return ExcludeSemantics(
      child: DecoratedBox(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: colors,
          ),
        ),
        child: LayoutBuilder(
          builder: (context, c) {
            final bubble = (c.biggest.shortestSide * 0.42).clamp(28.0, 96.0);
            return Stack(
              fit: StackFit.expand,
              children: [
                Center(
                  child: Container(
                    width: bubble,
                    height: bubble,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: Colors.white.withValues(alpha: 0.7),
                      shape: BoxShape.circle,
                    ),
                    child: Text(
                      initials,
                      style: TextStyle(
                        fontSize: bubble / 2.6,
                        fontWeight: FontWeight.w800,
                        color: DokiColors.ink,
                      ),
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

/// A grid of game tiles sized to the width: covers keep their shape
/// ([ArtSettings.coverAspect]) with the title and status underneath.
class CoverGrid extends StatelessWidget {
  const CoverGrid({super.key, required this.games, required this.tile});
  final List<Game> games;
  final Widget Function(Game) tile;

  /// Room under each cover for the title and a two-line status.
  static const captionHeight = 64.0;

  @override
  Widget build(BuildContext context) {
    final art = AppScope.of(context).art;
    return ListenableBuilder(
      listenable: art,
      builder: (context, _) => LayoutBuilder(
        builder: (context, c) {
          final aspect = art.settings.coverAspect;
          final target = aspect < 1 ? 150.0 : 240.0;
          const gap = 16.0;
          final columns = ((c.maxWidth + gap) / (target + gap)).floor().clamp(2, 8);
          final width = (c.maxWidth - gap * (columns - 1)) / columns;
          return GridView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            padding: const EdgeInsets.symmetric(vertical: 8),
            gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: columns,
              crossAxisSpacing: gap,
              mainAxisSpacing: 12,
              mainAxisExtent: width / aspect + captionHeight,
            ),
            itemCount: games.length,
            itemBuilder: (context, i) => tile(games[i]),
          );
        },
      ),
    );
  }
}
