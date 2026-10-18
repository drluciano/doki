import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/remote_games.dart';
import 'add_game.dart';
import 'review.dart';

/// Only progress waiting for a decision. Nothing here changes a save.
class IncomingScreen extends StatelessWidget {
  const IncomingScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final empty = state.incoming.isEmpty && state.unmapped.isEmpty;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Incoming'),
        backgroundColor: Colors.transparent,
        actions: [
          IconButton(tooltip: 'Refresh', icon: const Icon(Icons.refresh), onPressed: state.refresh),
        ],
      ),
      body: empty
          ? EmptyState(
              icon: Icons.inbox_outlined,
              title: 'Nothing waiting',
              body: state.lastSync == null
                  ? 'Progress from your other devices appears here after a sync. It never replaces your save on its own.'
                  : 'Last checked ${formatWhen(state.lastSync!)}. Progress from your other devices appears here.',
            )
          : RefreshIndicator(
              onRefresh: state.refresh,
              child: ListView(
                padding: const EdgeInsets.all(16),
                children: [
                  ContentWidth(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        for (final u in state.unmapped)
                          Card(
                            child: ListTile(
                              contentPadding: const EdgeInsets.symmetric(
                                horizontal: 16,
                                vertical: 8,
                              ),
                              leading: GameBadge(u.nameHint, size: 40),
                              title: Text(u.nameHint),
                              subtitle: Text(remoteGameLine(u, thisDevice: state.deviceName)),
                              trailing: const Icon(Icons.chevron_right),
                              onTap: () => Navigator.push(
                                context,
                                MaterialPageRoute<void>(builder: (_) => AddGameScreen(link: u)),
                              ),
                            ),
                          ),
                        for (final i in state.incoming)
                          Card(
                            child: ListTile(
                              contentPadding: const EdgeInsets.symmetric(
                                horizontal: 16,
                                vertical: 8,
                              ),
                              leading: GameBadge(state.game(i.gameId)?.name ?? '?', size: 40),
                              title: Text(state.game(i.gameId)?.name ?? 'Game'),
                              subtitle: Text(
                                'From ${i.fromDevice} · saved ${formatWhen(i.checkpoint.capturedAt)}',
                              ),
                              trailing: Icon(Icons.chevron_right, color: DokiColors.muted),
                              onTap: () => Navigator.push(
                                context,
                                MaterialPageRoute<void>(
                                  builder: (_) => ReviewScreen(
                                    gameId: i.gameId,
                                    checkpointId: i.checkpoint.id,
                                    fromDevice: i.fromDevice,
                                  ),
                                ),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
    );
  }
}
