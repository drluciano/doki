import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import 'common.dart';

/// "Keep checkpoints for": how long this device keeps checkpoint files.
/// Forever unless the user picks a time; choosing a shorter time shows what
/// would be deleted before anything is.
class RetentionCard extends StatelessWidget {
  const RetentionCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    return Card(
      child: ListTile(
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        leading: const Icon(Icons.auto_delete_outlined, color: DokiColors.lavender),
        title: const Text('Keep checkpoints for'),
        subtitle: Text(
          state.settings.keepDays == null
              ? 'Forever. Every checkpoint stays on this device.'
              : '${retentionLabel(state.settings.keepDays)}. Older ones are deleted from this device; '
                    'the newest $keepNewest of each game, current saves and pinned ones always stay.',
        ),
        trailing: Text(
          retentionLabel(state.settings.keepDays),
          style: const TextStyle(color: DokiColors.lavender, fontWeight: FontWeight.w600),
        ),
        onTap: () => chooseRetention(context),
      ),
    );
  }
}

/// Matches the engine's `KEEP_NEWEST`.
const keepNewest = 10;

Future<void> chooseRetention(BuildContext context) async {
  final state = AppScope.read(context);
  final days = await showDialog<int?>(
    context: context,
    builder: (context) => SimpleDialog(
      title: const Text('Keep checkpoints for'),
      children: [
        for (final d in retentionChoices)
          SimpleDialogOption(
            onPressed: () => Navigator.pop(context, d ?? 0),
            child: Row(
              children: [
                Icon(
                  d == state.settings.keepDays
                      ? Icons.radio_button_checked
                      : Icons.radio_button_unchecked,
                  color: DokiColors.lavender,
                ),
                const SizedBox(width: 12),
                Text(retentionLabel(d)),
              ],
            ),
          ),
      ],
    ),
  );
  // Forever comes back as 0 so that null can mean "closed without choosing".
  if (days == null || !context.mounted) return;
  final keepDays = days == 0 ? null : days;
  if (keepDays == state.settings.keepDays) return;
  if (keepDays != null) {
    final preview = await guarded(
      context,
      () => state.backend.call('retention_preview', {'keep_days': keepDays}),
    ) as Map<String, dynamic>?;
    if (preview == null || !context.mounted) return;
    final count = preview['checkpoints'] as int;
    if (count > 0) {
      final ok = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text('Delete $count old checkpoint${count == 1 ? '' : 's'}?'),
          content: Text(
            'Keeping ${retentionLabel(keepDays).toLowerCase()} deletes $count checkpoint'
            '${count == 1 ? '' : 's'} from this device now, freeing '
            '${formatBytes(preview['bytes'] as int)}. Your other devices keep their copies. '
            'Current saves, pinned checkpoints and the newest $keepNewest of each game are always kept.',
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Delete'),
            ),
          ],
        ),
      );
      if (ok != true || !context.mounted) return;
    }
  }
  await guarded(context, () => state.updateSettings({'keep_days': keepDays}));
}
