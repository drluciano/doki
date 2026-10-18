import 'package:flutter/material.dart';

import '../main.dart';
import '../models.dart';
import '../screens/add_game.dart';
import '../theme.dart';
import 'common.dart';

/// Games set up on your other devices but not on this one. Tapping one
/// opens setup with the game already chosen, so the only step left is
/// picking where it saves on this device.
class RemoteGamesSection extends StatelessWidget {
  const RemoteGamesSection({super.key, this.onPicked});

  /// Called instead of opening setup, for screens that are setup already.
  final void Function(Unmapped)? onPicked;

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    if (state.unmapped.isEmpty) return const SizedBox.shrink();
    final text = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(4, 8, 4, 8),
          child: Text('On your other devices', style: text.titleMedium),
        ),
        for (final u in state.unmapped)
          Card(
            child: ListTile(
              contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              leading: GameBadge(u.nameHint, size: 40),
              title: Text(u.nameHint),
              subtitle: Text(remoteGameLine(u)),
              trailing: const Icon(Icons.add_circle_outline, color: DokiColors.lavender),
              onTap: () => onPicked != null
                  ? onPicked!(u)
                  : Navigator.push(
                      context,
                      MaterialPageRoute<void>(builder: (_) => AddGameScreen(link: u)),
                    ),
            ),
          ),
      ],
    );
  }
}

String remoteGameLine(Unmapped u) => u.hasProgress
    ? 'Progress arrived from ${u.fromDevice}. Choose where it saves here to review it.'
    : 'Set up on ${u.fromDevice}. Choose where it saves here to keep them in step.';
