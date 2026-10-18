import 'package:flutter/material.dart';

import '../backend.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// Register a save folder. Connecting only records where the save lives; it
/// never changes files. [link] sets up a game received from another device.
class AddGameScreen extends StatefulWidget {
  const AddGameScreen({super.key, this.link});
  final Unmapped? link;

  @override
  State<AddGameScreen> createState() => _AddGameScreenState();
}

class _AddGameScreenState extends State<AddGameScreen> {
  late final _name = TextEditingController(text: widget.link?.nameHint ?? '');
  PickedFolder? _folder;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _pick() async {
    final state = AppScope.read(context);
    final folder = await guarded(context, state.backend.pickFolder);
    if (folder != null && mounted) setState(() => _folder = folder);
  }

  Future<void> _connect() async {
    final state = AppScope.read(context);
    final name = _name.text.trim();
    if (name.isEmpty || _folder == null) return;
    setState(() => _busy = true);
    final added = await guarded(context, () async {
      await state.backend.call('add', {
        'name': name,
        'location': _folder!.location,
        if (widget.link != null) 'link': widget.link!.gameId,
      });
      await state.refresh();
      return true;
    });
    if (!mounted) return;
    setState(() => _busy = false);
    if (added == true) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final link = widget.link;
    return Scaffold(
      appBar: AppBar(title: Text(link == null ? 'Add game' : 'Set up ${link.nameHint}')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ContentWidth(
          maxWidth: 560,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (link != null) ...[
                StatusBanner(
                  'Progress arrived from ${link.fromDevice}',
                  detail: 'Choose where this game saves on this device. The received save waits in Incoming until you review it.',
                ),
                const SizedBox(height: 24),
              ],
              TextField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'Game title'),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 24),
              Text('Save folder', style: text.titleMedium),
              const SizedBox(height: 4),
              Text(
                'Choose the folder where the emulator keeps this game\'s save. Keep Doki\'s own storage out of it.',
                style: text.bodyMedium?.copyWith(color: DokiColors.muted),
              ),
              const SizedBox(height: 12),
              Card(
                child: ListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  leading: const Icon(Icons.folder_outlined, color: DokiColors.lavender),
                  title: Text(_folder?.displayName ?? 'No folder chosen'),
                  subtitle: _folder == null
                      ? null
                      : Text(_folder!.location, maxLines: 2, overflow: TextOverflow.ellipsis),
                  trailing: TextButton(
                    onPressed: _busy ? null : _pick,
                    child: Text(_folder == null ? 'Choose' : 'Change'),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              const StatusBanner.success(
                'Connecting does not change your save',
                detail: 'Doki reads the folder to make checkpoints. It replaces files only when you choose Use this save or Restore.',
              ),
              const SizedBox(height: 24),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: _busy || _folder == null || _name.text.trim().isEmpty
                      ? null
                      : _connect,
                  child: const Text('Connect folder'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
