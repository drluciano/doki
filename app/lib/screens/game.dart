import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import 'edit_game.dart';
import 'library.dart';
import 'review.dart';

/// One game: its status, Create checkpoint, incoming progress and history.
class GameScreen extends StatefulWidget {
  const GameScreen({super.key, required this.gameId});
  final String gameId;

  @override
  State<GameScreen> createState() => _GameScreenState();
}

class _GameScreenState extends State<GameScreen> {
  List<HistoryEntry>? _history;
  Map<String, dynamic>? _recovery;
  Object? _error;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final state = AppScope.read(context);
    try {
      final h = await state.backend.call('history', {'game': widget.gameId}) as List;
      final r = await state.backend.call('recovery_status', {'game': widget.gameId});
      if (!mounted) return;
      setState(() {
        _history = [for (final e in h) HistoryEntry(e as Map<String, dynamic>)].reversed.toList();
        _recovery = r as Map<String, dynamic>?;
        _error = null;
      });
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  Future<void> _reload() async {
    await AppScope.read(context).refresh();
    await _load();
  }

  Future<void> _capture(Game game) async {
    final note = TextEditingController();
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Create checkpoint'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Close ${game.name} in the emulator first, so the save is complete.'),
            const SizedBox(height: 16),
            TextField(
              controller: note,
              decoration: const InputDecoration(
                labelText: 'Label (optional)',
                hintText: 'e.g. Before the gym',
              ),
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('The game is closed'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    final state = AppScope.read(context);
    final result = await withProgress(context, 'Copying and verifying the save…', () {
      return state.backend.call('capture', {
        'game': game.id,
        if (note.text.trim().isNotEmpty) 'note': note.text.trim(),
      });
    });
    if (result == null || !mounted) return;
    final created = result['outcome'] == 'created';
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          created
              ? 'Checkpoint created. Verified copy kept.'
              : 'No changes since checkpoint ${shortId(result['checkpoint'] as String)}.',
        ),
      ),
    );
    await _reload();
  }

  Future<void> _recover(String action) async {
    final state = AppScope.read(context);
    final label = action == 'finish' ? 'Finishing the change…' : 'Putting your save back…';
    final r = await withProgress(
      context,
      label,
      () => state.backend.call('recover', {'game': widget.gameId, 'action': action}),
    );
    if (r == null || !mounted) return;
    final msg = switch (r['outcome']) {
      'finished' => 'Finished. The chosen save is now in place.',
      'put_back' => 'Put back. Your save from before the change is in place.',
      _ => 'Nothing had been changed yet. Your save is as it was.',
    };
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
    await _reload();
  }

  Future<void> _togglePin(HistoryEntry e) async {
    final state = AppScope.read(context);
    await guarded(
      context,
      () => state.backend.call(e.pinned ? 'unpin' : 'pin', {
        'game': widget.gameId,
        'checkpoint': e.checkpoint.id,
      }),
    );
    await _load();
  }

  void _openReview(String checkpoint, {String? fromDevice}) {
    Navigator.push(
      context,
      MaterialPageRoute<void>(
        builder: (_) =>
            ReviewScreen(gameId: widget.gameId, checkpointId: checkpoint, fromDevice: fromDevice),
      ),
    ).then((_) => _load());
  }

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final game = state.game(widget.gameId);
    if (game == null) {
      return Scaffold(
        appBar: AppBar(),
        body: const EmptyState(
          icon: Icons.search_off,
          title: 'Game not found',
          body: 'It may have been removed.',
        ),
      );
    }
    final incoming = state.incoming.where((i) => i.gameId == game.id).toList();
    final text = Theme.of(context).textTheme;
    final blocked = _recovery != null;

    return Scaffold(
      appBar: AppBar(
        title: Text(game.name),
        actions: [
          PopupMenuButton<String>(
            tooltip: 'Game actions',
            onSelected: (choice) async {
              if (choice == 'edit') {
                await Navigator.push(
                  context,
                  MaterialPageRoute<void>(builder: (_) => EditGameScreen(game: game)),
                );
              } else if (choice == 'remove' && await confirmRemoveGame(context, game)) {
                if (context.mounted) Navigator.pop(context);
              }
            },
            itemBuilder: (_) => const [
              PopupMenuItem(
                value: 'edit',
                child: ListTile(leading: Icon(Icons.edit_outlined), title: Text('Edit game')),
              ),
              PopupMenuItem(
                value: 'remove',
                child: ListTile(
                  leading: Icon(Icons.remove_circle_outline),
                  title: Text('Remove from this device'),
                ),
              ),
            ],
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _reload,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            ContentWidth(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      GameBadge(game.name, size: 56),
                      const SizedBox(width: 16),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(game.name, style: text.titleLarge),
                            Text(
                              statusText(game),
                              style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  if (blocked)
                    _RecoveryCard(
                      status: _recovery!,
                      onFinish: () => _recover('finish'),
                      onPutBack: () => _recover('put_back'),
                    ),
                  if (game.status == GameStatus.unavailable)
                    StatusBanner.attention(
                      statusText(game),
                      detail: errorCopy(game.unavailableCode ?? '').next,
                    ),
                  if (!blocked) ...[
                    SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: game.status == GameStatus.unavailable
                            ? null
                            : () => _capture(game),
                        icon: const Icon(Icons.bookmark_add_outlined),
                        label: Text(game.autoCapture ? 'Checkpoint now' : 'Create checkpoint'),
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      game.autoCapture
                          ? 'Automatic checkpoints are on: Doki saves one when the save stops changing.'
                          : 'Automatic checkpoints are off. Turn them on in Edit game.',
                      style: text.bodySmall?.copyWith(color: DokiColors.muted),
                    ),
                  ],
                  if (incoming.isNotEmpty) ...[
                    const SizedBox(height: 24),
                    Text('Incoming progress', style: text.titleMedium),
                    for (final i in incoming)
                      Card(
                        child: ListTile(
                          contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                          leading: const Icon(Icons.move_to_inbox, color: DokiColors.lavender),
                          title: Text('From ${i.fromDevice}'),
                          subtitle: Text('Saved ${formatWhen(i.checkpoint.capturedAt)}'),
                          trailing: const Icon(Icons.chevron_right),
                          onTap: blocked
                              ? null
                              : () => _openReview(i.checkpoint.id, fromDevice: i.fromDevice),
                        ),
                      ),
                  ],
                  const SizedBox(height: 24),
                  Text('History', style: text.titleMedium),
                  const SizedBox(height: 4),
                  if (_error != null)
                    StatusBanner.error('Could not load history', detail: '$_error'),
                  if (_history == null && _error == null)
                    const Padding(
                      padding: EdgeInsets.all(24),
                      child: Center(child: CircularProgressIndicator()),
                    ),
                  if (_history != null && _history!.isEmpty)
                    Text(
                      'No checkpoints yet. Create one to protect this save.',
                      style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                    ),
                  for (final e in _history ?? <HistoryEntry>[])
                    HistoryTile(
                      entry: e,
                      onRestore: blocked || e.isHead ? null : () => _openReview(e.checkpoint.id),
                      onTogglePin: () => _togglePin(e),
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

class HistoryTile extends StatelessWidget {
  const HistoryTile({super.key, required this.entry, this.onRestore, this.onTogglePin});
  final HistoryEntry entry;
  final VoidCallback? onRestore;
  final VoidCallback? onTogglePin;

  @override
  Widget build(BuildContext context) {
    final c = entry.checkpoint;
    final text = Theme.of(context).textTheme;
    Widget tag(String label, Color bg, Color fg, IconData icon) => Padding(
      padding: const EdgeInsets.only(right: 6, top: 6),
      child: Chip(
        visualDensity: VisualDensity.compact,
        avatar: Icon(icon, size: 16, color: fg),
        label: Text(label, style: TextStyle(color: fg)),
        backgroundColor: bg,
        side: BorderSide.none,
      ),
    );
    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              c.kind == 'choice'
                  ? Icons.call_split
                  : (c.kind == 'backup' ? Icons.shield_outlined : Icons.bookmark_outline),
              color: DokiColors.lavender,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(checkpointTitle(c), style: text.titleSmall),
                  Text(
                    '${c.deviceName} · ${formatWhen(c.capturedAt)} · ${sizeSummary(c)}',
                    style: text.bodySmall?.copyWith(color: DokiColors.muted, fontSize: 14),
                  ),
                  Wrap(
                    children: [
                      if (entry.isHead)
                        tag(
                          'Current save',
                          DokiColors.mint,
                          DokiColors.mintInk,
                          Icons.play_arrow_rounded,
                        ),
                      if (entry.setAside)
                        tag(
                          'Separate progress',
                          DokiColors.attention,
                          DokiColors.attentionInk,
                          Icons.call_split,
                        ),
                      if (entry.pinned)
                        tag('Pinned', DokiColors.lavenderSoft, DokiColors.lavender, Icons.push_pin),
                    ],
                  ),
                ],
              ),
            ),
            PopupMenuButton<String>(
              tooltip: 'Checkpoint actions',
              onSelected: (v) => v == 'restore' ? onRestore?.call() : onTogglePin?.call(),
              itemBuilder: (_) => [
                PopupMenuItem(
                  value: 'restore',
                  enabled: onRestore != null,
                  child: const Text('Restore this checkpoint'),
                ),
                PopupMenuItem(
                  value: 'pin',
                  child: Text(entry.pinned ? 'Unpin' : 'Pin (keep forever)'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// Persistent card shown while an install was interrupted. The user picks
/// Finish or Put back; Doki never decides for them.
class _RecoveryCard extends StatelessWidget {
  const _RecoveryCard({required this.status, required this.onFinish, required this.onPutBack});
  final Map<String, dynamic> status;
  final VoidCallback onFinish;
  final VoidCallback onPutBack;

  @override
  Widget build(BuildContext context) {
    final files = status['files'] as List;
    final complete = status['complete'] == true;
    final other = files.where((f) => (f as List)[1] == 'other').length;
    return Card(
      color: DokiColors.attention,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const StatusBanner.attention('A change to this save was interrupted'),
            const SizedBox(height: 12),
            Text(
              complete ? 'All files are already in place. Finish to record it.' : 'The save may be part old, part new. Both versions are safe in Doki. Choose how to continue.',
            ),
            if (other > 0) ...[
              const SizedBox(height: 8),
              Text('$other file(s) changed since. Doki will keep them as a checkpoint first.'),
            ],
            const SizedBox(height: 16),
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                FilledButton(onPressed: onFinish, child: const Text('Finish the change')),
                OutlinedButton(
                  onPressed: complete ? null : onPutBack,
                  child: const Text('Put my save back'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
