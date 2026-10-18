import 'package:flutter/material.dart';

import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// Compare a checkpoint with the current save, then choose. Used for both
/// incoming progress and Restore a checkpoint. Opening a review never
/// changes the playable save; only "Back up and use this save" does.
class ReviewScreen extends StatefulWidget {
  const ReviewScreen({
    super.key,
    required this.gameId,
    required this.checkpointId,
    this.fromDevice,
  });
  final String gameId;
  final String checkpointId;
  final String? fromDevice;

  @override
  State<ReviewScreen> createState() => _ReviewScreenState();
}

class _ReviewScreenState extends State<ReviewScreen> {
  Review? _review;
  Checkpoint? _current;
  Map<String, dynamic>? _done;
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    final state = AppScope.read(context);
    final ok = await guarded(context, () async {
      final r = Review(
        await state.backend.call('review', {
          'game': widget.gameId,
          'checkpoint': widget.checkpointId,
        }) as Map<String, dynamic>,
      );
      Checkpoint? current;
      if (r.baselineHead != null) {
        final h = await state.backend.call('history', {'game': widget.gameId}) as List;
        current = h
            .map((e) => HistoryEntry(e as Map<String, dynamic>).checkpoint)
            .where((c) => c.id == r.baselineHead)
            .firstOrNull;
      }
      if (mounted) {
        setState(() {
          _review = r;
          _current = current;
        });
      }
      return true;
    });
    if (ok == null && mounted) setState(() => _failed = true);
  }

  Future<void> _use() async {
    final state = AppScope.read(context);
    final review = _review!;
    final result = await withProgress(
      context,
      'Backing up, copying and verifying…',
      () => state.backend.call('use', {'review': review.json}),
    );
    await state.refresh();
    if (!mounted) return;
    if (result != null) {
      setState(() => _done = result as Map<String, dynamic>);
    } else {
      // A changed save or an interrupted install needs a fresh look.
      setState(() {
        _review = null;
        _current = null;
      });
      await _load();
    }
  }

  Future<void> _decide(String cmd, String message) async {
    final state = AppScope.read(context);
    final ok = await guarded(context, () async {
      await state.backend.call(cmd, {'game': widget.gameId, 'checkpoint': _review!.incoming.id});
      await state.refresh();
      return true;
    });
    if (ok == true && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
      Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    final review = _review;
    final restoring = review != null && !review.isProposal;
    return Scaffold(
      appBar: AppBar(title: Text(restoring ? 'Restore a checkpoint' : 'Incoming progress')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: ContentWidth(
          child: _done != null
              ? _Done(result: _done!, restoring: restoring)
              : review == null
              ? (_failed
                    ? const StatusBanner.error(
                        'Could not open this checkpoint',
                        detail: 'Your save was not changed.',
                      )
                    : const Padding(
                        padding: EdgeInsets.all(48),
                        child: Center(child: CircularProgressIndicator()),
                      ))
              : _body(context, review, restoring),
        ),
      ),
    );
  }

  Widget _body(BuildContext context, Review review, bool restoring) {
    final incoming = review.incoming;
    final from = widget.fromDevice ?? incoming.deviceName;
    final relation = review.relation;
    final canUse = relation != Relation.same && relation != Relation.unknown;
    final cards = [
      _SaveCard(
        heading: 'Your current save',
        checkpoint: _current,
        emptyText: 'No checkpoint yet on this device.',
      ),
      _SaveCard(
        heading: restoring ? 'Checkpoint to restore' : 'Incoming progress from $from',
        checkpoint: incoming,
        highlight: true,
      ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (review.capturedLocalChanges != null) ...[
          StatusBanner.success(
            'Your latest progress was saved first',
            detail:
                'It is checkpoint ${shortId(review.capturedLocalChanges!)}, so nothing is lost whichever you choose.',
          ),
          const SizedBox(height: 12),
        ],
        StatusBanner(restoring ? 'Restore this checkpoint?' : relationHeadline(relation, from)),
        const SizedBox(height: 16),
        LayoutBuilder(
          builder: (context, c) {
            if (c.maxWidth < 640) return Column(children: cards);
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final card in cards)
                  Expanded(
                    child: Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: card),
                  ),
              ],
            );
          },
        ),
        const SizedBox(height: 16),
        Text(
          canUse
              ? 'Doki backs up your current save and verifies it before replacing anything. It stays in your history.'
              : 'There is nothing to install.',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(color: DokiColors.muted),
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            if (canUse)
              FilledButton.icon(
                onPressed: _use,
                icon: const Icon(Icons.shield_outlined),
                label: Text(restoring ? 'Back up and restore' : 'Back up and use this save'),
              ),
            if (!restoring) ...[
              OutlinedButton(
                onPressed: () => _decide('keep', 'Kept alongside. Find it in History any time.'),
                child: const Text('Keep alongside'),
              ),
              TextButton(
                onPressed: () =>
                    _decide('dismiss', 'Kept your save. The other one stays in History.'),
                child: const Text('Keep my save'),
              ),
            ],
          ],
        ),
      ],
    );
  }
}

class _SaveCard extends StatelessWidget {
  const _SaveCard({
    required this.heading,
    required this.checkpoint,
    this.highlight = false,
    this.emptyText,
  });
  final String heading;
  final Checkpoint? checkpoint;
  final bool highlight;
  final String? emptyText;

  @override
  Widget build(BuildContext context) {
    final c = checkpoint;
    final text = Theme.of(context).textTheme;
    return Card(
      color: highlight ? DokiColors.lavenderSoft : DokiColors.surface,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(heading, style: text.titleMedium),
            const SizedBox(height: 8),
            if (c == null)
              Text(emptyText ?? '', style: text.bodyMedium?.copyWith(color: DokiColors.muted))
            else ...[
              Text(checkpointTitle(c), style: text.bodyLarge),
              Text('From ${c.deviceName}', style: text.bodyMedium),
              Text('Saved ${formatWhen(c.capturedAt)}', style: text.bodyMedium),
              Text(sizeSummary(c), style: text.bodyMedium?.copyWith(color: DokiColors.muted)),
            ],
          ],
        ),
      ),
    );
  }
}

class _Done extends StatelessWidget {
  const _Done({required this.result, required this.restoring});
  final Map<String, dynamic> result;
  final bool restoring;

  @override
  Widget build(BuildContext context) {
    final backup = result['backup'] as String?;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        StatusBanner.success(
          restoring ? 'Checkpoint restored' : 'Now using this save',
          detail:
              'Installed checkpoint ${shortId(result['installed'] as String)} and verified every file.',
        ),
        const SizedBox(height: 12),
        if (backup != null)
          Text(
            'Your previous save is checkpoint ${shortId(backup)}. Restore it from History any time.',
            style: Theme.of(context).textTheme.bodyLarge,
          ),
        const SizedBox(height: 24),
        FilledButton(onPressed: () => Navigator.pop(context), child: const Text('Done')),
      ],
    );
  }
}
