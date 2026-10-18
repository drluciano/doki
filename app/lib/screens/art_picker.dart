import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../art.dart';
import '../main.dart';
import '../models.dart';
import '../theme.dart';
import '../widgets/common.dart';
import '../widgets/cover.dart';
import 'art_settings.dart';

/// Run an art action, explaining a failure in a snackbar. Art never touches
/// saves, so its errors are not save errors.
Future<T?> artGuard<T>(BuildContext context, Future<T> Function() action) async {
  try {
    return await action();
  } catch (e) {
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(e is ArtError ? e.message : 'Something went wrong with the art.')),
      );
    }
    return null;
  }
}

const _images = XTypeGroup(
  label: 'Images',
  extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'],
  mimeTypes: ['image/png', 'image/jpeg', 'image/webp', 'image/gif'],
  uniformTypeIdentifiers: ['public.image'],
);

/// A game's cover and banner: choose your own image, find one on
/// SteamGridDB, or remove it. Art stays on this device.
class ArtPickerScreen extends StatefulWidget {
  const ArtPickerScreen({super.key, required this.game});
  final Game game;

  @override
  State<ArtPickerScreen> createState() => _ArtPickerScreenState();
}

class _ArtPickerScreenState extends State<ArtPickerScreen> {
  late final _query = TextEditingController(text: searchTerm(widget.game.name) ?? widget.game.name);
  List<SgdbGame>? _results;
  SgdbGame? _chosen;
  List<SgdbImage>? _covers, _heroes;
  bool _searching = false;
  String? _saving;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  ArtStore get _art => AppScope.read(context).art;

  Future<void> _upload(ArtKind kind) async {
    await artGuard(context, () async {
      final file = await openFile(acceptedTypeGroups: [_images]);
      if (file == null) return;
      await _art.setImage(widget.game.id, kind, await file.readAsBytes());
    });
  }

  Future<void> _search() async {
    final api = _art.steamGridDb;
    final term = _query.text.trim();
    if (api == null || term.isEmpty) return;
    setState(() {
      _searching = true;
      _chosen = null;
    });
    final found = await artGuard(context, () => api.search(term));
    if (!mounted) return;
    setState(() {
      _searching = false;
      _results = found ?? _results;
    });
  }

  Future<void> _open(SgdbGame g) async {
    final api = _art.steamGridDb!;
    setState(() {
      _chosen = g;
      _covers = _heroes = null;
    });
    final wide = _art.settings.wideCovers;
    final both = await artGuard(
      context,
      () => Future.wait([
        api.images(g.id, ArtKind.cover, wide: wide),
        api.images(g.id, ArtKind.hero),
      ]),
    );
    if (!mounted || _chosen != g) return;
    setState(() {
      _covers = both?[0] ?? [];
      _heroes = both?[1] ?? [];
    });
  }

  Future<void> _use(SgdbImage image, ArtKind kind) async {
    setState(() => _saving = image.url);
    final ok = await artGuard(context, () async {
      await _art.setImage(widget.game.id, kind, await _art.steamGridDb!.download(image.url));
      return true;
    });
    if (!mounted) return;
    setState(() => _saving = null);
    if (ok == true) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(kind == ArtKind.cover ? 'Cover saved' : 'Banner saved')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final art = AppScope.of(context).art;
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    return Scaffold(
      appBar: AppBar(title: Text('Art for ${widget.game.name}')),
      body: ListenableBuilder(
        listenable: art,
        builder: (context, _) {
          final aspect = art.settings.coverAspect;
          return ListView(
            padding: const EdgeInsets.all(24),
            children: [
              ContentWidth(
                maxWidth: 720,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (!art.enabled)
                      const StatusBanner.attention('Art cannot be saved on this device'),
                    _ArtSlot(
                      title: 'Cover',
                      detail: 'Shown in the library.',
                      preview: SizedBox(
                        height: aspect < 1 ? 180 : 100,
                        child: AspectRatio(
                          aspectRatio: aspect,
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(coverRadius),
                            child: GameCover(gameId: widget.game.id, name: widget.game.name),
                          ),
                        ),
                      ),
                      has: art.image(widget.game.id, ArtKind.cover) != null,
                      onUpload: () => _upload(ArtKind.cover),
                      onRemove: () => art.removeImage(widget.game.id, ArtKind.cover),
                    ),
                    const SizedBox(height: 16),
                    _ArtSlot(
                      title: 'Banner',
                      detail: 'Shown across the top of the game\'s page.',
                      preview: SizedBox(
                        height: 100,
                        child: AspectRatio(
                          aspectRatio: 96 / 31,
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(16),
                            child: switch (art.image(widget.game.id, ArtKind.hero)) {
                              null => ColoredBox(
                                color: DokiColors.lavenderSoft,
                                child: Center(child: Text('No banner')),
                              ),
                              final f => Image.file(f, fit: BoxFit.cover, cacheWidth: 800),
                            },
                          ),
                        ),
                      ),
                      has: art.image(widget.game.id, ArtKind.hero) != null,
                      onUpload: () => _upload(ArtKind.hero),
                      onRemove: () => art.removeImage(widget.game.id, ArtKind.hero),
                    ),
                    const SizedBox(height: 32),
                    Text('Find on SteamGridDB', style: text.titleMedium),
                    const SizedBox(height: 4),
                    if (art.steamGridDb == null) ...[
                      Text(
                        'SteamGridDB is a free, community-made library of game art. Add your own API key in Game art settings to search it.',
                        style: muted,
                      ),
                      const SizedBox(height: 12),
                      OutlinedButton.icon(
                        onPressed: () => Navigator.push(
                          context,
                          MaterialPageRoute<void>(builder: (_) => const ArtSettingsScreen()),
                        ),
                        icon: const Icon(Icons.key_outlined),
                        label: const Text('Add an API key'),
                      ),
                    ] else ...[
                      Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: _query,
                              decoration: const InputDecoration(labelText: 'Search'),
                              textInputAction: TextInputAction.search,
                              onSubmitted: (_) => _search(),
                            ),
                          ),
                          const SizedBox(width: 12),
                          FilledButton(
                            onPressed: _searching ? null : _search,
                            child: const Text('Search'),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      if (_searching) const LinearProgressIndicator(),
                      if (_results != null && _results!.isEmpty && !_searching)
                        Text('No games found. Try a shorter name.', style: muted),
                      if (_chosen == null)
                        for (final g in _results ?? <SgdbGame>[])
                          Card(
                            child: ListTile(
                              title: Text(g.name),
                              subtitle: g.year == null ? null : Text('${g.year}'),
                              trailing: const Icon(Icons.chevron_right),
                              onTap: () => _open(g),
                            ),
                          )
                      else ...[
                        Row(
                          children: [
                            IconButton(
                              tooltip: 'Back to results',
                              icon: const Icon(Icons.arrow_back),
                              onPressed: () => setState(() => _chosen = null),
                            ),
                            Expanded(child: Text(_chosen!.name, style: text.titleSmall)),
                          ],
                        ),
                        _Thumbs(
                          title: 'Covers',
                          images: _covers,
                          aspect: aspect,
                          saving: _saving,
                          onPick: (i) => _use(i, ArtKind.cover),
                        ),
                        const SizedBox(height: 16),
                        _Thumbs(
                          title: 'Banners',
                          images: _heroes,
                          aspect: 96 / 31,
                          saving: _saving,
                          onPick: (i) => _use(i, ArtKind.hero),
                        ),
                      ],
                    ],
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

class _ArtSlot extends StatelessWidget {
  const _ArtSlot({
    required this.title,
    required this.detail,
    required this.preview,
    required this.has,
    required this.onUpload,
    required this.onRemove,
  });
  final String title, detail;
  final Widget preview;
  final bool has;
  final VoidCallback onUpload, onRemove;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Wrap(
          spacing: 16,
          runSpacing: 12,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            preview,
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 320),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(title, style: text.titleMedium),
                  Text(detail, style: text.bodyMedium?.copyWith(color: DokiColors.muted)),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    children: [
                      OutlinedButton.icon(
                        onPressed: onUpload,
                        icon: const Icon(Icons.upload_outlined),
                        label: const Text('Choose image'),
                      ),
                      if (has) TextButton(onPressed: onRemove, child: const Text('Remove')),
                    ],
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

/// SteamGridDB thumbnails to pick from.
class _Thumbs extends StatelessWidget {
  const _Thumbs({
    required this.title,
    required this.images,
    required this.aspect,
    required this.saving,
    required this.onPick,
  });
  final String title;
  final List<SgdbImage>? images;
  final double aspect;
  final String? saving;
  final void Function(SgdbImage) onPick;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final list = images;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: text.titleSmall),
        const SizedBox(height: 8),
        if (list == null)
          const LinearProgressIndicator()
        else if (list.isEmpty)
          Text(
            'None on SteamGridDB yet.',
            style: text.bodyMedium?.copyWith(color: DokiColors.muted),
          )
        else
          GridView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
              maxCrossAxisExtent: aspect < 1 ? 130 : 260,
              childAspectRatio: aspect,
              crossAxisSpacing: 12,
              mainAxisSpacing: 12,
            ),
            itemCount: list.length.clamp(0, 30),
            itemBuilder: (context, i) => Material(
              borderRadius: BorderRadius.circular(12),
              clipBehavior: Clip.antiAlias,
              color: DokiColors.lavenderSoft,
              child: InkWell(
                borderRadius: BorderRadius.circular(12),
                onTap: saving == null ? () => onPick(list[i]) : null,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    Image.network(
                      list[i].thumb,
                      fit: BoxFit.cover,
                      errorBuilder: (_, _, _) => const Icon(Icons.broken_image_outlined),
                    ),
                    if (saving == list[i].url)
                      const ColoredBox(
                        color: Color(0x88FFFFFF),
                        child: Center(child: CircularProgressIndicator()),
                      ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}
