import 'package:flutter/material.dart';

import '../backend.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/cover.dart';
import 'library.dart';

/// One whole-system folder: its games, the games hidden from it, and its
/// settings. Each game in it is a full game of its own (open it for
/// checkpoints and history).
class SystemScreen extends StatelessWidget {
  const SystemScreen({super.key, required this.systemId});
  final String systemId;

  Future<void> _trackAgain(BuildContext context, String key) async {
    final state = AppScope.read(context);
    await guarded(context, () async {
      await state.backend.call('track_again', {'system': systemId, 'key': key});
      await state.refresh();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final system = state.system(systemId);
    if (system == null) {
      return Scaffold(
        appBar: AppBar(),
        body: const EmptyState(
          icon: Icons.folder_off_outlined,
          title: 'System not found',
          body: 'It may have been removed from this device.',
        ),
      );
    }
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    final games = state.gamesIn(systemId);
    return Scaffold(
      appBar: AppBar(
        title: Text(system.name),
        actions: [
          PopupMenuButton<String>(
            tooltip: 'System actions',
            onSelected: (v) async {
              if (v == 'edit') {
                await Navigator.push(
                  context,
                  MaterialPageRoute<void>(builder: (_) => EditSystemScreen(system: system)),
                );
              } else if (v == 'remove' && await confirmRemoveSystem(context, system)) {
                if (context.mounted) Navigator.pop(context);
              }
            },
            itemBuilder: (_) => const [
              PopupMenuItem(value: 'edit', child: Text('Edit system')),
              PopupMenuItem(value: 'remove', child: Text('Remove from this device')),
            ],
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: state.refresh,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
          children: [
            ContentWidth(
              maxWidth: 1100,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Card(
                    child: ListTile(
                      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                      leading: const Icon(
                        Icons.folder_special_outlined,
                        color: DokiColors.lavender,
                      ),
                      title: Text(
                        games.length == 1 ? '1 game tracked' : '${games.length} games tracked',
                      ),
                      subtitle: Text(system.location, maxLines: 2, overflow: TextOverflow.ellipsis),
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(4, 8, 4, 0),
                    child: Text(
                      'Every subfolder and set of save files in this folder is its own game. New games are added automatically. ${system.autoCapture ? 'New games get automatic checkpoints.' : 'Automatic checkpoints are off for new games.'}',
                      style: muted,
                    ),
                  ),
                  if (games.isEmpty)
                    Padding(
                      padding: const EdgeInsets.all(16),
                      child: Text(
                        'No games found yet. Each game appears here once the emulator saves it.',
                        style: muted,
                      ),
                    )
                  else
                    CoverGrid(
                      games: games,
                      tile: (g) => GameTile(game: g),
                    ),
                  if (system.hidden.isNotEmpty) ...[
                    const SizedBox(height: 16),
                    Text('Not tracked', style: text.titleMedium),
                    const SizedBox(height: 4),
                    Text(
                      'You removed these. Their save files and checkpoints are kept; track one again to bring it back with its history.',
                      style: muted,
                    ),
                    const SizedBox(height: 8),
                    for (final key in system.hidden)
                      Card(
                        child: ListTile(
                          leading: GameBadge(hiddenGameName(key), size: 40),
                          title: Text(hiddenGameName(key)),
                          trailing: TextButton(
                            onPressed: () => _trackAgain(context, key),
                            child: const Text('Track again'),
                          ),
                        ),
                      ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Rename a system, move it to another folder, or set automatic
/// checkpoints for all its games. Nothing here reads or writes save files.
class EditSystemScreen extends StatefulWidget {
  const EditSystemScreen({super.key, required this.system});
  final SystemEntry system;

  @override
  State<EditSystemScreen> createState() => _EditSystemScreenState();
}

class _EditSystemScreenState extends State<EditSystemScreen> {
  late final _name = TextEditingController(text: widget.system.name);
  late bool _auto = widget.system.autoCapture;
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
      await state.backend.call('edit_system', {
        'system': widget.system.id,
        if (name != widget.system.name) 'name': name,
        if (_folder != null) 'location': _folder!.location,
        if (_auto != widget.system.autoCapture) 'auto_capture': _auto,
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
    return Scaffold(
      appBar: AppBar(title: const Text('Edit system')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ContentWidth(
          maxWidth: 560,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              TextField(
                controller: _name,
                decoration: const InputDecoration(labelText: 'System name'),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 24),
              Text('Save folder', style: text.titleMedium),
              const SizedBox(height: 4),
              Text(
                'Every game in this system moves with it. Changing the folder copies and deletes nothing.',
                style: muted,
              ),
              const SizedBox(height: 12),
              Card(
                child: ListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  leading: const Icon(Icons.folder_outlined, color: DokiColors.lavender),
                  title: Text(_folder?.displayName ?? 'Current folder'),
                  subtitle: Text(
                    _folder?.location ?? widget.system.location,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  trailing: TextButton(
                    onPressed: _busy ? null : _pick,
                    child: const Text('Change'),
                  ),
                ),
              ),
              const SizedBox(height: 16),
              Card(
                child: SwitchListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  value: _auto,
                  onChanged: _busy ? null : (v) => setState(() => _auto = v),
                  title: const Text('Automatic checkpoints'),
                  subtitle: const Text(
                    'For every game in this system, now and later. Each game can still be changed on its own page.',
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

/// Ask, then remove [system] and its games from this device only.
Future<bool> confirmRemoveSystem(BuildContext context, SystemEntry system) async {
  final state = AppScope.read(context);
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('Remove ${system.name} from this device?'),
      content: const Text(
        'Doki stops watching this folder and every game in it here. The save files stay where they are, every checkpoint is kept, and your other devices keep the system. You can set it up again later to bring its games back.',
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Remove')),
      ],
    ),
  );
  if (sure != true || !context.mounted) return false;
  final done = await guarded(context, () async {
    await state.backend.call('remove_system', {'system': system.id});
    await state.refresh();
    return true;
  });
  return done == true;
}
