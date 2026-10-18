import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/remote_games.dart';
import 'add_game.dart';
import 'game.dart';

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

class LibraryScreen extends StatelessWidget {
  const LibraryScreen({super.key, required this.onOpenIncoming});
  final VoidCallback onOpenIncoming;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    Future<void> addGame() =>
        Navigator.push(context, MaterialPageRoute<void>(builder: (_) => const AddGameScreen()));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Library'),
        backgroundColor: Colors.transparent,
        actions: [
          IconButton(tooltip: 'Refresh', icon: const Icon(Icons.refresh), onPressed: state.refresh),
        ],
      ),
      floatingActionButton: state.games.isEmpty
          ? null
          : FloatingActionButton.extended(
              onPressed: addGame,
              icon: const Icon(Icons.add),
              label: const Text('Add game'),
            ),
      body: state.games.isEmpty && state.unmapped.isNotEmpty
          ? ListView(
              padding: const EdgeInsets.all(16),
              children: [
                ContentWidth(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
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
          : state.games.isEmpty
          ? EmptyState(
              icon: Icons.videogame_asset_outlined,
              title: 'No games yet',
              body: 'Connect a game\'s save folder. Doki only reads it until you choose to use or restore a save.',
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
                    child: Column(
                      children: [
                        if (state.incoming.isNotEmpty)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 8),
                            child: InkWell(
                              borderRadius: BorderRadius.circular(16),
                              onTap: onOpenIncoming,
                              child: StatusBanner(
                                'Incoming progress is waiting',
                                detail:
                                    '${state.incoming.length} to review. Nothing changes until you choose.',
                              ),
                            ),
                          ),
                        for (final g in state.games) GameCard(game: g),
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

class GameCard extends StatelessWidget {
  const GameCard({super.key, required this.game});
  final Game game;

  @override
  Widget build(BuildContext context) {
    final attention =
        game.status == GameStatus.needsRecovery || game.status == GameStatus.unavailable;
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: () => Navigator.push(
          context,
          MaterialPageRoute<void>(builder: (_) => GameScreen(gameId: game.id)),
        ),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              GameBadge(game.name),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(game.name, style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 4),
                    Row(
                      children: [
                        Icon(
                          statusIcon(game),
                          size: 18,
                          color: attention ? DokiColors.attentionInk : DokiColors.muted,
                        ),
                        const SizedBox(width: 6),
                        Flexible(
                          child: Text(
                            statusText(game),
                            style: TextStyle(
                              color: attention ? DokiColors.attentionInk : DokiColors.muted,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              if (game.incoming > 0)
                Chip(
                  avatar: const Icon(Icons.move_to_inbox, size: 18),
                  label: Text('${game.incoming} incoming'),
                  backgroundColor: DokiColors.lavenderSoft,
                ),
            ],
          ),
        ),
      ),
    );
  }
}
