import 'package:flutter/material.dart';

import '../backend.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/remote_games.dart';

/// What a connected folder holds.
enum FolderKind {
  /// One game's save folder.
  game,

  /// An emulator's whole save folder: every game in it is tracked on its own.
  system,
}

/// Register a save folder. Connecting only records where the save lives; it
/// never changes files. [link] sets up a game received from another device;
/// [system] sets up a whole-system folder set up on another device.
class AddGameScreen extends StatefulWidget {
  const AddGameScreen({super.key, this.link, this.system});
  final Unmapped? link;
  final RemoteSystem? system;

  @override
  State<AddGameScreen> createState() => _AddGameScreenState();
}

class _AddGameScreenState extends State<AddGameScreen> {
  late final _name = TextEditingController(
    text: widget.link?.nameHint ?? widget.system?.name ?? '',
  );
  late Unmapped? _link = widget.link;
  late RemoteSystem? _linkSystem = widget.system;
  late FolderKind _kind = widget.system != null ? FolderKind.system : FolderKind.game;
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
      if (_kind == FolderKind.system) {
        await state.backend.call('add_system', {
          'name': name,
          'location': _folder!.location,
          if (_linkSystem != null) 'link': _linkSystem!.id,
        });
      } else {
        await state.backend.call('add', {
          'name': name,
          'location': _folder!.location,
          if (_link != null) 'link': _link!.gameId,
        });
      }
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
    final link = _link;
    final linkSystem = _linkSystem;
    final system = _kind == FolderKind.system;
    final title = link != null
        ? 'Set up ${link.nameHint}'
        : linkSystem != null
        ? 'Set up ${linkSystem.name}'
        : 'Add game';
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ContentWidth(
          maxWidth: 560,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (link != null) ...[
                StatusBanner(
                  link.fromDevice == AppScope.of(context).deviceName
                      ? 'Set up again to bring back its history'
                      : link.hasProgress
                      ? 'Progress arrived from ${link.fromDevice}'
                      : 'Already set up on ${link.fromDevice}',
                  detail: link.hasProgress
                      ? 'Choose where this game saves on this device. The received save waits in Incoming until you review it.'
                      : 'Choose where this game saves on this device. From then on, progress on either device shows up on the other for review.',
                ),
                if (widget.link == null)
                  TextButton(
                    onPressed: () => setState(() {
                      _link = null;
                      _name.clear();
                    }),
                    child: const Text('Add a different game instead'),
                  ),
                const SizedBox(height: 24),
              ] else if (linkSystem != null) ...[
                StatusBanner(
                  linkSystem.fromDevice == AppScope.of(context).deviceName
                      ? 'Set up again to bring back its games'
                      : 'Already set up on ${linkSystem.fromDevice}',
                  detail:
                      'Choose the same emulator\'s save folder on this device. Each game in it keeps in step with the same game on ${linkSystem.fromDevice}, and received progress waits in Incoming until you review it.',
                ),
                if (widget.system == null)
                  TextButton(
                    onPressed: () => setState(() {
                      _linkSystem = null;
                      _name.clear();
                    }),
                    child: const Text('Add a different folder instead'),
                  ),
                const SizedBox(height: 24),
              ] else ...[
                RemoteSystemsSection(
                  onPicked: (r) => setState(() {
                    _linkSystem = r;
                    _kind = FolderKind.system;
                    _name.text = r.name;
                  }),
                ),
                RemoteGamesSection(
                  onPicked: (u) => setState(() {
                    _link = u;
                    _kind = FolderKind.game;
                    _name.text = u.nameHint;
                  }),
                ),
                if (AppScope.of(context).unmappedGames.isNotEmpty ||
                    AppScope.of(context).remoteSystems.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Text('Or add a different game', style: text.titleMedium),
                  const SizedBox(height: 12),
                ],
                Text('What is in this folder?', style: text.titleMedium),
                const SizedBox(height: 12),
                SizedBox(
                  width: double.infinity,
                  child: SegmentedButton<FolderKind>(
                    showSelectedIcon: false,
                    segments: const [
                      ButtonSegment(
                        value: FolderKind.game,
                        icon: Icon(Icons.videogame_asset_outlined),
                        label: Text('One game'),
                      ),
                      ButtonSegment(
                        value: FolderKind.system,
                        icon: Icon(Icons.folder_special_outlined),
                        label: Text('Whole system'),
                      ),
                    ],
                    selected: {_kind},
                    onSelectionChanged: (k) => setState(() => _kind = k.first),
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  system
                      ? 'Every game in the folder is tracked on its own, and games you play later are added automatically.'
                      : 'Only this game\'s save is tracked.',
                  style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                ),
                const SizedBox(height: 24),
              ],
              TextField(
                controller: _name,
                decoration: InputDecoration(
                  labelText: system ? 'System name' : 'Game title',
                  hintText: system ? 'e.g. Eden or RetroArch GBA' : null,
                ),
                onChanged: (_) => setState(() {}),
              ),
              const SizedBox(height: 24),
              Text('Save folder', style: text.titleMedium),
              const SizedBox(height: 4),
              Text(
                system
                    ? 'Choose the folder where the emulator keeps every game\'s save: one subfolder or set of save files per game. Keep Doki\'s own storage out of it.'
                    : 'Choose the folder where the emulator keeps this game\'s save. Keep Doki\'s own storage out of it.',
                style: text.bodyMedium?.copyWith(color: DokiColors.muted),
              ),
              const SizedBox(height: 12),
              Card(
                child: ListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  leading: Icon(Icons.folder_outlined, color: DokiColors.lavender),
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
