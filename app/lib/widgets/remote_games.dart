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
              subtitle: Text(remoteGameLine(u, thisDevice: state.deviceName)),
              trailing: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.add_circle_outline, color: DokiColors.lavender),
                  PopupMenuButton<String>(
                    tooltip: 'More',
                    onSelected: (_) => confirmForgetGame(context, u),
                    itemBuilder: (_) => [
                      PopupMenuItem(
                        value: 'forget',
                        child: Text(
                          u.fromDevice == state.deviceName
                              ? 'Delete from this device'
                              : 'Don\'t set up on this device',
                        ),
                      ),
                    ],
                  ),
                ],
              ),
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

String remoteGameLine(Unmapped u, {String? thisDevice}) => u.fromDevice == thisDevice
    ? 'Removed from this device. Set it up again to bring back its history.'
    : u.hasProgress
    ? 'Progress arrived from ${u.fromDevice}. Choose where it saves here to review it.'
    : 'Set up on ${u.fromDevice}. Choose where it saves here to keep them in step.';

/// Ask, then delete a game that is not set up here, with its checkpoints,
/// from this device. Other devices keep theirs; Doki stops listing it.
Future<void> confirmForgetGame(BuildContext context, Unmapped u) async {
  final state = AppScope.read(context);
  final removedHere = u.fromDevice == state.deviceName;
  final ok = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(removedHere ? 'Delete ${u.nameHint}?' : 'Hide ${u.nameHint} on this device?'),
      content: Text(
        removedHere
            ? 'Its checkpoints are deleted from this device and it is no longer listed here. '
                  'Your save folder is not touched, and your other devices keep their copies.'
            : 'It stays set up on ${u.fromDevice}. This device stops listing it and deletes any '
                  'of its progress that already arrived here.',
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: Text(removedHere ? 'Delete' : 'Hide'),
        ),
      ],
    ),
  );
  if (ok == true && context.mounted) {
    await guarded(context, () => state.forgetGame(u.gameId));
  }
}
