import 'package:flutter/material.dart';

import '../backend.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// Change a game's title, its save folder or automatic checkpoints. Nothing
/// here reads or writes save files.
class EditGameScreen extends StatefulWidget {
  const EditGameScreen({super.key, required this.game});
  final Game game;

  @override
  State<EditGameScreen> createState() => _EditGameScreenState();
}

class _EditGameScreenState extends State<EditGameScreen> {
  late final _name = TextEditingController(text: widget.game.name);
  late bool _auto = widget.game.autoCapture;
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

  Future<void> _save() async {
    final state = AppScope.read(context);
    final name = _name.text.trim();
    if (name.isEmpty) return;
    setState(() => _busy = true);
    final saved = await guarded(context, () async {
      await state.backend.call('edit', {
        'game': widget.game.id,
        if (name != widget.game.name) 'name': name,
        if (_folder != null) 'location': _folder!.location,
        if (_auto != widget.game.autoCapture) 'auto_capture': _auto,
      });
      await state.refresh();
      return true;
    });
    if (!mounted) return;
    setState(() => _busy = false);
    if (saved == true) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    final system = AppScope.of(context).system(widget.game.systemId ?? '');
    return Scaffold(
      appBar: AppBar(title: const Text('Edit game')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ContentWidth(
          maxWidth: 560,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'Game title'),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 24),
              Text('Save folder', style: text.titleMedium),
              const SizedBox(height: 4),
              if (system != null)
                Text(
                  'This game is part of ${system.name}. To use another folder, change the system\'s folder: every game in it moves together.',
                  style: muted,
                )
              else ...[
                Text(
                  'Changing the folder copies and deletes nothing. Doki checkpoints whatever the new folder holds from then on.',
                  style: muted,
                ),
                const SizedBox(height: 12),
                Card(
                  child: ListTile(
                    contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    leading: Icon(Icons.folder_outlined, color: DokiColors.lavender),
                    title: Text(_folder?.displayName ?? 'Current folder'),
                    subtitle: Text(
                      _folder?.location ?? widget.game.location,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    trailing: TextButton(
                      onPressed: _busy ? null : _pick,
                      child: const Text('Change'),
                    ),
                  ),
                ),
              ],
              const SizedBox(height: 16),
              Card(
                child: SwitchListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  value: _auto,
                  onChanged: _busy ? null : (v) => setState(() => _auto = v),
                  title: const Text('Automatic checkpoints'),
                  subtitle: const Text(
                    'Save a checkpoint on its own once the save stops changing for 30 seconds.',
                  ),
                ),
              ),
              const SizedBox(height: 24),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: _busy || _name.text.trim().isEmpty ? null : _save,
                  child: const Text('Save changes'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Ask, then remove [game] from this device only. Returns whether it was
/// removed.
Future<bool> confirmRemoveGame(BuildContext context, Game game) async {
  final state = AppScope.read(context);
  final system = state.system(game.systemId ?? '');
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('Remove ${game.name} from this device?'),
      content: Text(
        system != null
            ? 'Doki stops tracking this game in ${system.name} here, and does not add it back on its own. The save files stay where they are, every checkpoint is kept, and your other devices keep the game. To bring it back, choose Track again on ${system.name}\'s page.'
            : 'Doki stops watching its save folder here. The save files stay where they are, every checkpoint is kept, and your other devices keep the game. You can set it up again later to bring its history back.',
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Remove')),
      ],
    ),
  );
  if (sure != true || !context.mounted) return false;
  final done = await guarded(context, () async {
    await state.backend.call('remove', {'game': game.id});
    await state.refresh();
    return true;
  });
  return done == true;
}
