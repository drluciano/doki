import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../art.dart';
import '../main.dart';
import '../theme.dart';
import '../widgets/common.dart';
import 'art_picker.dart';

/// Where game art comes from: the user's own SteamGridDB API key, whether
/// to look up covers automatically, and the cover shape.
class ArtSettingsScreen extends StatefulWidget {
  const ArtSettingsScreen({super.key});

  static const keyPage = 'https://www.steamgriddb.com/profile/preferences/api';

  @override
  State<ArtSettingsScreen> createState() => _ArtSettingsScreenState();
}

class _ArtSettingsScreenState extends State<ArtSettingsScreen> {
  late final _key = TextEditingController(
    text: AppScope.read(context).art.settings.steamGridDbKey ?? '',
  );
  bool _show = false;
  bool _checking = false;
  String? _keyProblem;

  @override
  void dispose() {
    _key.dispose();
    super.dispose();
  }

  /// Check the key with one search before keeping it.
  Future<void> _saveKey() async {
    final art = AppScope.read(context).art;
    final key = _key.text.trim();
    if (key.isEmpty) {
      await art.updateSettings(art.settings.copyWith(clearKey: true));
      setState(() => _keyProblem = null);
      return;
    }
    setState(() {
      _checking = true;
      _keyProblem = null;
    });
    String? problem;
    try {
      await SteamGridDb(key).search('Tetris');
    } on ArtError catch (e) {
      problem = e.message;
    }
    if (!mounted) return;
    if (problem == null) {
      await art.updateSettings(art.settings.copyWith(steamGridDbKey: key));
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Key saved. Doki can now find art on SteamGridDB.')),
      );
    }
    setState(() {
      _checking = false;
      _keyProblem = problem;
    });
  }

  Future<void> _fetchAll() async {
    final state = AppScope.read(context);
    await withProgress(
      context,
      'Looking for art…',
      () => artGuard(context, () => state.art.fetchMissing(state.games, force: true)),
    );
  }

  @override
  Widget build(BuildContext context) {
    final art = AppScope.of(context).art;
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    return Scaffold(
      appBar: AppBar(title: const Text('Game art')),
      body: ListenableBuilder(
        listenable: art,
        builder: (context, _) {
          final s = art.settings;
          return ListView(
            padding: const EdgeInsets.all(24),
            children: [
              ContentWidth(
                maxWidth: 560,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Covers and banners make the library easier to scan. Art is kept on this device only and never changes your saves. Choose your own image on any game\'s page (Change art), or let Doki find art on SteamGridDB.',
                      style: muted,
                    ),
                    const SizedBox(height: 24),
                    Text('SteamGridDB API key', style: text.titleMedium),
                    const SizedBox(height: 4),
                    Text(
                      'SteamGridDB is a free, community-made art library used by many emulation front ends. Sign in there, open Preferences, then API, and copy your key here. It stays on this device.',
                      style: muted,
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: SelectableText(
                            ArtSettingsScreen.keyPage,
                            style: text.bodyMedium?.copyWith(color: DokiColors.lavender),
                          ),
                        ),
                        IconButton(
                          tooltip: 'Copy address',
                          icon: const Icon(Icons.copy_outlined),
                          onPressed: () => Clipboard.setData(
                            const ClipboardData(text: ArtSettingsScreen.keyPage),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    TextField(
                      controller: _key,
                      obscureText: !_show,
                      autocorrect: false,
                      enableSuggestions: false,
                      decoration: InputDecoration(
                        labelText: 'API key',
                        errorText: _keyProblem,
                        errorMaxLines: 3,
                        suffixIcon: IconButton(
                          tooltip: _show ? 'Hide key' : 'Show key',
                          icon: Icon(_show ? Icons.visibility_off : Icons.visibility),
                          onPressed: () => setState(() => _show = !_show),
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        FilledButton(
                          onPressed: _checking ? null : _saveKey,
                          child: Text(_checking ? 'Checking…' : 'Save key'),
                        ),
                        const SizedBox(width: 12),
                        if (s.steamGridDbKey != null)
                          const Flexible(
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  Icons.check_circle_outline,
                                  color: DokiColors.mintInk,
                                  size: 18,
                                ),
                                SizedBox(width: 6),
                                Flexible(child: Text('Key in use')),
                              ],
                            ),
                          ),
                      ],
                    ),
                    const SizedBox(height: 24),
                    Card(
                      child: SwitchListTile(
                        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                        value: s.autoFetch,
                        onChanged: (v) => art.updateSettings(s.copyWith(autoFetch: v)),
                        title: const Text('Find art automatically'),
                        subtitle: const Text(
                          'Look up a cover and banner for each new game that has none. Needs an API key.',
                        ),
                      ),
                    ),
                    const SizedBox(height: 16),
                    Text('Cover shape', style: text.titleMedium),
                    const SizedBox(height: 8),
                    SegmentedButton<bool>(
                      showSelectedIcon: false,
                      segments: const [
                        ButtonSegment(
                          value: false,
                          icon: Icon(Icons.crop_portrait),
                          label: Text('Tall'),
                        ),
                        ButtonSegment(
                          value: true,
                          icon: Icon(Icons.crop_landscape),
                          label: Text('Wide'),
                        ),
                      ],
                      selected: {s.wideCovers},
                      onSelectionChanged: (v) =>
                          art.updateSettings(s.copyWith(wideCovers: v.first)),
                    ),
                    const SizedBox(height: 24),
                    OutlinedButton.icon(
                      onPressed: s.steamGridDbKey == null ? null : _fetchAll,
                      icon: const Icon(Icons.auto_awesome_outlined),
                      label: const Text('Find art for games without it'),
                    ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}
