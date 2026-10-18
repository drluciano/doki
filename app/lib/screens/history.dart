import 'package:flutter/material.dart';

import '../main.dart';
import '../widgets/common.dart';
import 'game.dart';
import 'library.dart';

/// Pick a game to see its history.
class HistoryScreen extends StatelessWidget {
  const HistoryScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('History'), backgroundColor: Colors.transparent),
      body: state.games.isEmpty
          ? const EmptyState(
              icon: Icons.history,
              title: 'No history yet',
              body: 'Add a game and create a checkpoint to start its history.',
            )
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                ContentWidth(
                  child: Column(
                    children: [
                      for (final g in state.games)
                        Card(
                          child: ListTile(
                            contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                            leading: GameBadge(g.name, size: 40),
                            title: Text(g.name),
                            subtitle: Text(statusText(g)),
                            trailing: const Icon(Icons.chevron_right),
                            onTap: () => Navigator.push(
                              context,
                              MaterialPageRoute<void>(builder: (_) => GameScreen(gameId: g.id)),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
    );
  }
}
