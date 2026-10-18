import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/cover.dart';
import '../widgets/remote_games.dart';
import 'add_game.dart';
import 'art_settings.dart';
import 'game.dart';
import 'system.dart';

String statusText(Game g) => switch (g.status) {
  GameStatus.protected => 'Protected by a checkpoint',
  GameStatus.uncapturedChanges =>
    g.autoCapture ? 'New progress, checkpoint coming soon' : 'New progress not in a checkpoint yet',
  GameStatus.noCheckpoint => 'No checkpoint yet',
  GameStatus.needsRecovery => 'Needs your choice: finish or put back',
  GameStatus.unavailable => errorCopy(g.unavailableCode ?? '').title,
};

IconData statusIcon(Game g) => switch (g.status) {
  GameStatus.protected => Icons.verified_user_outlined,
  GameStatus.uncapturedChanges => Icons.edit_note,
  GameStatus.noCheckpoint => Icons.bookmark_border,
  GameStatus.needsRecovery => Icons.warning_amber_rounded,
  GameStatus.unavailable => Icons.folder_off_outlined,
};

/// The library: every game as a cover, grouped by whole-system folder, with
/// games from other devices to set up underneath.
class LibraryScreen extends StatefulWidget {
  const LibraryScreen({super.key, required this.onOpenIncoming});
  final VoidCallback onOpenIncoming;

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen> {
  /// The game under the mouse or focus, named in the title bar like a
  /// console home screen.
  final _spotlight = ValueNotifier<Game?>(null);

  @override
  void dispose() {
    _spotlight.dispose();
    super.dispose();
  }

  Widget _tile(Game g) => GameTile(
    game: g,
    onSpotlight: (on) {
      if (on) {
        _spotlight.value = g;
      } else if (_spotlight.value?.id == g.id) {
        _spotlight.value = null;
      }
    },
  );

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final text = Theme.of(context).textTheme;
    Future<void> addGame() =>
        Navigator.push(context, MaterialPageRoute<void>(builder: (_) => const AddGameScreen()));
    final nothingHere = state.games.isEmpty && state.systems.isEmpty;
    final offers = state.unmappedGames.isNotEmpty || state.remoteSystems.isNotEmpty;
    final standalone = state.standaloneGames;

    return Scaffold(
      appBar: AppBar(
        title: ValueListenableBuilder<Game?>(
          valueListenable: _spotlight,
          builder: (context, g, _) => Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text('Library'),
              AnimatedSize(
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeOutCubic,
                child: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 200),
                  child: g == null
                      ? const SizedBox(width: double.infinity)
                      : Text(
                          spotlightLine(g, state.system(g.systemId ?? '')?.name),
                          key: ValueKey(g.id),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: text.bodyMedium?.copyWith(color: DokiColors.lavender),
                        ),
                ),
              ),
            ],
          ),
        ),
        backgroundColor: Colors.transparent,
        actions: [
          IconButton(
            tooltip: 'Game art',
            icon: const Icon(Icons.palette_outlined),
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute<void>(builder: (_) => const ArtSettingsScreen()),
            ),
          ),
          IconButton(tooltip: 'Refresh', icon: const Icon(Icons.refresh), onPressed: state.refresh),
        ],
      ),
      floatingActionButton: nothingHere
          ? null
          : FloatingActionButton.extended(
              onPressed: addGame,
              icon: const Icon(Icons.add),
              label: const Text('Add game'),
            ),
      body: nothingHere && offers
          ? ListView(
              padding: const EdgeInsets.all(16),
              children: [
                ContentWidth(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const RemoteSystemsSection(),
                      const RemoteGamesSection(),
                      const SizedBox(height: 16),
                      OutlinedButton.icon(
                        onPressed: addGame,
                        icon: const Icon(Icons.create_new_folder_outlined),
                        label: const Text('Add a different game'),
                      ),
                    ],
                  ),
                ),
              ],
            )
          : nothingHere
          ? EmptyState(
              icon: Icons.videogame_asset_outlined,
              title: 'No games yet',
              body: 'Connect a game\'s save folder, or an emulator\'s whole save folder to track every game in it. Doki only reads it until you choose to use or restore a save.',
              action: FilledButton.icon(
                onPressed: addGame,
                icon: const Icon(Icons.create_new_folder_outlined),
                label: const Text('Add game'),
              ),
            )
          : RefreshIndicator(
              onRefresh: state.refresh,
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 96),
                children: [
                  ContentWidth(
                    maxWidth: 1100,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        if (state.incoming.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 8),
                            child: InkWell(
                              borderRadius: BorderRadius.circular(16),
                              onTap: widget.onOpenIncoming,
                              child: StatusBanner(
                                'Incoming progress is waiting',
                                detail:
                                    '${state.incoming.length} to review. Nothing changes until you choose.',
                              ),
                            ),
                          ),
                        for (final system in state.systems) ...[
                          SystemHeader(system: system, games: state.gamesIn(system.id).length),
                          if (state.gamesIn(system.id).isEmpty)
                            Padding(
                              padding: const EdgeInsets.fromLTRB(4, 0, 4, 16),
                              child: Text(
                                'No games found yet. Each game appears here once the emulator saves it.',
                                style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                              ),
                            )
                          else
                            CoverGrid(games: state.gamesIn(system.id), tile: _tile),
                        ],
                        if (standalone.isNotEmpty) ...[
                          if (state.systems.isNotEmpty)
                            Padding(
                              padding: const EdgeInsets.fromLTRB(4, 16, 4, 0),
                              child: Text('Games', style: text.titleMedium),
                            ),
                          CoverGrid(games: standalone, tile: _tile),
                        ],
                        const RemoteSystemsSection(),
                        const RemoteGamesSection(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
    );
  }
}

/// Title-bar line for the game in the spotlight.
String spotlightLine(Game g, String? systemName) =>
    systemName == null ? '${g.name}  ·  ${statusText(g)}' : '${g.name}  ·  $systemName';

/// A whole-system folder's heading in the library; opens the system.
class SystemHeader extends StatelessWidget {
  const SystemHeader({super.key, required this.system, required this.games});
  final SystemEntry system;
  final int games;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute<void>(builder: (_) => SystemScreen(systemId: system.id)),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 8),
          child: Row(
            children: [
              Icon(Icons.folder_special_outlined, color: DokiColors.lavender),
              const SizedBox(width: 10),
              Flexible(
                child: Text(system.name, style: text.titleMedium, overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Text(
                games == 1 ? '1 game' : '$games games',
                style: text.bodyMedium?.copyWith(color: DokiColors.muted),
              ),
              Icon(Icons.chevron_right, color: DokiColors.muted),
            ],
          ),
        ),
      ),
    );
  }
}

/// One game in the library grid: the cover (the part the focus border
/// wraps, like a console shelf), then its title and status in words.
class GameTile extends StatelessWidget {
  const GameTile({super.key, required this.game, this.onSpotlight});
  final Game game;

  /// Told when the mouse or focus arrives on (true) or leaves the cover.
  final void Function(bool)? onSpotlight;

  @override
  Widget build(BuildContext context) {
    final attention =
        game.status == GameStatus.needsRecovery || game.status == GameStatus.unavailable;
    final color = attention ? DokiColors.attentionInk : DokiColors.muted;
    void open() => Navigator.push(
      context,
      MaterialPageRoute<void>(builder: (_) => GameScreen(gameId: game.id)),
    );
    final incoming = game.incoming > 0 ? '. ${game.incoming} incoming' : '';
    return Semantics(
      button: true,
      label: '${game.name}. ${statusText(game)}$incoming',
      onTap: open,
      excludeSemantics: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          AspectRatio(
            aspectRatio: AppScope.of(context).art.settings.coverAspect,
            child: Material(
              elevation: 2,
              shadowColor: DokiColors.ink.withValues(alpha: 0.25),
              borderRadius: BorderRadius.circular(coverRadius),
              clipBehavior: Clip.antiAlias,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  GameCover(gameId: game.id, name: game.name),
                  if (game.incoming > 0)
                    Positioned(
                      top: 8,
                      right: 8,
                      child: CoverPill(
                        icon: Icons.move_to_inbox,
                        label: '${game.incoming} incoming',
                      ),
                    ),
                  if (attention)
                    Positioned(
                      top: 8,
                      left: 8,
                      child: CoverPill(icon: statusIcon(game), label: 'Needs you', attention: true),
                    ),
                  Material(
                    type: MaterialType.transparency,
                    child: InkWell(
                      borderRadius: BorderRadius.circular(coverRadius),
                      onTap: open,
                      onHover: onSpotlight,
                      onFocusChange: onSpotlight,
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 8),
          GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: open,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  game.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleSmall
                      ?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 2),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Padding(
                      padding: const EdgeInsets.only(top: 1),
                      child: Icon(statusIcon(game), size: 14, color: color),
                    ),
                    const SizedBox(width: 4),
                    Expanded(
                      child: Text(
                        statusText(game),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 12, height: 1.25, color: color),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// A small label on a cover, always an icon and a word.
class CoverPill extends StatelessWidget {
  const CoverPill({super.key, required this.icon, required this.label, this.attention = false});
  final IconData icon;
  final String label;
  final bool attention;

  @override
  Widget build(BuildContext context) {
    final fg = attention ? DokiColors.attentionInk : DokiColors.lavender;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: (attention ? DokiColors.attention : Colors.white).withValues(alpha: 0.94),
        borderRadius: BorderRadius.circular(12),
        boxShadow: [BoxShadow(color: DokiColors.ink.withValues(alpha: 0.12), blurRadius: 6)],
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 14, color: fg),
            const SizedBox(width: 4),
            Text(
              label,
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w700, color: fg),
            ),
          ],
        ),
      ),
    );
  }
}
