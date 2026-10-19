/// User-facing wording. Follows the vocabulary in docs/design.md: Checkpoint,
/// Separate progress, Incoming progress, Use this save, Restore a checkpoint.
library;

import 'backend.dart';
import 'models.dart';

/// What happened, whether playable files changed, and what to do next.
class ErrorCopy {
  const ErrorCopy(this.title, this.next);
  final String title;
  final String next;
}

ErrorCopy errorCopy(String code) => switch (code) {
  'SourceUnavailable' => const ErrorCopy(
    'Save location unavailable',
    'Reconnect the storage or choose the folder again.',
  ),
  'PermissionLost' => const ErrorCopy(
    'Folder access needs updating',
    'Choose the folder again to allow access.',
  ),
  'SourceChanging' => const ErrorCopy(
    'Waiting for the save to finish',
    'Close the game completely, then try again.',
  ),
  'BundleIncomplete' => const ErrorCopy(
    'This folder has no save files',
    'Check that you chose the save folder.',
  ),
  'VaultFull' => const ErrorCopy('Doki storage is full', 'Free some space on this device.'),
  'SaveLocationFull' => const ErrorCopy(
    'The save folder is full',
    'Free some space where the save is stored.',
  ),
  'IntegrityFailed' => const ErrorCopy(
    'A received copy is damaged',
    'Sync again. Your current save is untouched.',
  ),
  'LocalChangedSinceReview' => const ErrorCopy(
    'You played since reviewing',
    'Your newer progress was kept as a checkpoint. Review again.',
  ),
  'BackupUnverified' => const ErrorCopy(
    'Could not confirm a backup',
    'Nothing was replaced. Try again in a moment.',
  ),
  'StrategyUnsupported' => const ErrorCopy(
    'This folder cannot be updated safely',
    'Doki will not replace files here. Your checkpoints stay safe in History.',
  ),
  'InstallInterrupted' || 'RecoveryRequired' => const ErrorCopy(
    'Finish or undo the last change',
    'Open the game and choose Finish or Put back.',
  ),
  'RecoveryFailed' => const ErrorCopy(
    'Recovery could not finish',
    'The game stays protected and your checkpoints are safe in History. Try again.',
  ),
  'Busy' => const ErrorCopy('Doki is busy with this game', 'Wait a moment and try again.'),
  'NotFound' => const ErrorCopy('Not found', 'Refresh and try again.'),
  'InvalidInput' => const ErrorCopy('That did not look right', 'Check what you entered.'),
  'UnsupportedVersion' => const ErrorCopy(
    'Made by a newer Doki',
    'Update Doki on this device to read it.',
  ),
  _ => const ErrorCopy('Something went wrong', 'Try again. Your checkpoints are kept.'),
};

/// Whether the playable save was changed, in words (never color alone).
String filesChangedLine(DokiError e) => e.saveMayBeMixed
    ? 'Your save may be part old, part new until you finish or put it back. Every version is kept in Doki.'
    : 'Your playable save was not changed.';

String relationHeadline(Relation r, String fromDevice) => switch (r) {
  Relation.none => 'This is the first save for this game on this device.',
  Relation.same => 'This is already your current save.',
  Relation.ahead => '$fromDevice continued from your current save.',
  Relation.behind => 'This is older than your current save.',
  Relation.divergent => 'You played on both devices. Choose which save to continue. Both versions will stay in your history.',
  Relation.unrelated =>
    'These saves started separately. Choose which one to continue. Both will stay in your history.',
  Relation.unknown => 'Some history has not arrived yet, so Doki cannot tell how these relate.',
};

String formatWhen(DateTime t) {
  String two(int n) => n.toString().padLeft(2, '0');
  return '${t.year}-${two(t.month)}-${two(t.day)} ${two(t.hour)}:${two(t.minute)}';
}

String formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / 1024 / 1024).toStringAsFixed(1)} MB';
}

String checkpointTitle(Checkpoint c) {
  if (c.kind == 'choice') return c.note ?? 'Choice';
  if (c.kind == 'backup') return 'Backup before a change';
  return c.note?.isNotEmpty == true ? c.note! : 'Checkpoint ${shortId(c.id)}';
}

String sizeSummary(Checkpoint c) =>
    '${c.files.length} ${c.files.length == 1 ? 'file' : 'files'}, ${formatBytes(c.totalBytes)}';
