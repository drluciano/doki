/// Game art: a cover for the library grid and a wide banner ("hero") for
/// the game's page.
///
/// Art is presentation only. It is kept on this device, in the app's own
/// support folder (never in a save folder or the vault), and is not synced.
/// It comes from one of two places:
///
/// * SteamGridDB (https://www.steamgriddb.com), the community art library
///   emulation front ends use. Its API needs a free personal key, which the
///   user pastes in Game art settings; Doki never ships one.
/// * An image the user chooses.
///
/// Without either, the library shows a styled placeholder with the game's
/// initials.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

import 'models.dart';

enum ArtKind {
  /// Library cover: tall (2:3) or wide, see [ArtSettings.wideCovers].
  cover,

  /// Banner across the top of a game's page (SteamGridDB "hero", 96:31).
  hero,
}

/// Why art could not be found or saved, in words for the user.
class ArtError implements Exception {
  ArtError(this.message);
  final String message;
  @override
  String toString() => message;
}

class ArtSettings {
  const ArtSettings({this.steamGridDbKey, this.autoFetch = true, this.wideCovers = false});

  /// Personal SteamGridDB API key, or null when not set up.
  final String? steamGridDbKey;

  /// Look up a cover for games that have none, once per game.
  final bool autoFetch;

  /// Wide covers (SteamGridDB 920x430) instead of tall ones (600x900).
  final bool wideCovers;

  /// Cover width divided by height.
  double get coverAspect => wideCovers ? 920 / 430 : 2 / 3;

  ArtSettings copyWith({
    String? steamGridDbKey,
    bool clearKey = false,
    bool? autoFetch,
    bool? wideCovers,
  }) => ArtSettings(
    steamGridDbKey: clearKey ? null : steamGridDbKey ?? this.steamGridDbKey,
    autoFetch: autoFetch ?? this.autoFetch,
    wideCovers: wideCovers ?? this.wideCovers,
  );
}

/// Turns a game or folder name into a SteamGridDB search: drops region and
/// dump tags such as `(USA)` or `[!]`, a file extension and underscores.
/// Returns null for names that cannot be searched, such as a Switch title
/// id folder (`0100F2C0115B6000`).
String? searchTerm(String name) {
  var t = name
      .replaceAll(RegExp(r'\([^)]*\)|\[[^\]]*\]'), ' ')
      .replaceAll(
        RegExp(r'\.(sav|srm|state\d*|ss\d|dsv|mcr|gci|ps2|bin)$', caseSensitive: false),
        '',
      )
      .replaceAll('_', ' ')
      .replaceAll(RegExp(r'\s+'), ' ')
      .trim();
  if (t.isEmpty || RegExp(r'^[0-9A-Fa-f]{16}$').hasMatch(t)) return null;
  return t;
}

/// A game in SteamGridDB's catalog.
class SgdbGame {
  SgdbGame(Map<String, dynamic> json)
    : id = (json['id'] as num).toInt(),
      name = json['name'] as String,
      year = json['release_date'] is num
          ? DateTime.fromMillisecondsSinceEpoch((json['release_date'] as num).toInt() * 1000).year
          : null;
  final int id;
  final String name;
  final int? year;
}

/// One image offered by SteamGridDB.
class SgdbImage {
  SgdbImage(Map<String, dynamic> json)
    : url = json['url'] as String,
      thumb = (json['thumb'] as String?) ?? json['url'] as String;
  final String url, thumb;
}

/// Minimal SteamGridDB API v2 client: search, covers, heroes.
class SteamGridDb {
  SteamGridDb(this.apiKey, {HttpClient Function()? client, this.base = defaultBase})
    : _client = client ?? HttpClient.new;

  static const defaultBase = 'https://www.steamgriddb.com/api/v2';

  /// API root; tests point it at a local server.
  final String base;
  final String apiKey;
  final HttpClient Function() _client;

  Future<dynamic> _get(String path) async {
    final client = _client();
    try {
      final req = await client.getUrl(Uri.parse('$base$path')).timeout(const Duration(seconds: 15));
      req.headers.set(HttpHeaders.authorizationHeader, 'Bearer $apiKey');
      final res = await req.close().timeout(const Duration(seconds: 20));
      final body = await res.transform(utf8.decoder).join();
      if (res.statusCode == 401 || res.statusCode == 403) {
        throw ArtError('SteamGridDB did not accept the API key. Check it in Game art settings.');
      }
      if (res.statusCode == 404) return const [];
      if (res.statusCode != 200) {
        throw ArtError('SteamGridDB answered with an error (${res.statusCode}).');
      }
      final json = jsonDecode(body) as Map<String, dynamic>;
      if (json['success'] != true) throw ArtError('SteamGridDB could not answer that search.');
      return json['data'];
    } on ArtError {
      rethrow;
    } on Object {
      throw ArtError('Could not reach SteamGridDB. Check the connection and try again.');
    } finally {
      client.close();
    }
  }

  Future<List<SgdbGame>> search(String term) async => [
    for (final g in await _get('/search/autocomplete/${Uri.encodeComponent(term)}') as List)
      SgdbGame(g as Map<String, dynamic>),
  ];

  Future<List<SgdbImage>> images(int gameId, ArtKind kind, {bool wide = false}) async {
    final path = switch (kind) {
      ArtKind.cover =>
        '/grids/game/$gameId?types=static&nsfw=false&dimensions=${wide ? '920x430,460x215' : '600x900,342x482,660x930'}',
      ArtKind.hero => '/heroes/game/$gameId?types=static&nsfw=false',
    };
    return [for (final i in await _get(path) as List) SgdbImage(i as Map<String, dynamic>)];
  }

  /// Bytes of an image from SteamGridDB's CDN (https only, or the API's
  /// own host).
  Future<Uint8List> download(String url) async {
    final uri = Uri.parse(url);
    if (uri.scheme != 'https' && uri.host != Uri.parse(base).host) {
      throw ArtError('SteamGridDB gave an address Doki will not open.');
    }
    final client = _client();
    try {
      final res = await (await client.getUrl(uri)).close().timeout(const Duration(seconds: 30));
      if (res.statusCode != 200) throw ArtError('The image could not be downloaded.');
      final out = BytesBuilder(copy: false);
      await for (final chunk in res) {
        out.add(chunk);
        if (out.length > maxImageBytes) throw ArtError('That image is too large.');
      }
      return out.takeBytes();
    } on ArtError {
      rethrow;
    } on Object {
      throw ArtError('Could not reach SteamGridDB. Check the connection and try again.');
    } finally {
      client.close();
    }
  }
}

const maxImageBytes = 20 * 1024 * 1024;

/// The file extension for image bytes, or null if they are not an image
/// Flutter can show (PNG, JPEG, WebP, GIF).
String? imageExtension(List<int> b) {
  bool starts(List<int> sig, [int at = 0]) =>
      b.length >= at + sig.length &&
      [for (var i = 0; i < sig.length; i++) b[at + i] == sig[i]].every((x) => x);
  if (starts([0x89, 0x50, 0x4E, 0x47])) return 'png';
  if (starts([0xFF, 0xD8, 0xFF])) return 'jpg';
  if (starts([0x47, 0x49, 0x46, 0x38])) return 'gif';
  if (starts([0x52, 0x49, 0x46, 0x46]) && starts([0x57, 0x45, 0x42, 0x50], 8)) return 'webp';
  return null;
}

/// Covers and banners on disk, plus [ArtSettings]. Layout of [dir]:
/// `<game id>.<cover|hero>.<stamp>.<ext>` per image (the stamp changes on
/// every replacement so no stale picture is shown from the image cache) and
/// `settings.json`.
class ArtStore extends ChangeNotifier {
  ArtStore(this.dir, {HttpClient Function()? client})
    : _client = client; // ignore: prefer_initializing_formals

  /// No art and no settings: used in tests and when storage is unavailable.
  ArtStore.disabled() : dir = null, _client = null;

  static Future<ArtStore> open() async {
    try {
      final support = await getApplicationSupportDirectory();
      final store = ArtStore(Directory('${support.path}${Platform.pathSeparator}art'));
      await store.load();
      return store;
    } on Object {
      return ArtStore.disabled();
    }
  }

  final Directory? dir;
  final HttpClient Function()? _client;
  final _files = <String, File>{};
  final _tried = <String>{};
  ArtSettings _settings = const ArtSettings();
  bool _fetching = false;

  ArtSettings get settings => _settings;
  bool get enabled => dir != null;

  /// A client when an API key is set.
  SteamGridDb? get steamGridDb {
    final key = _settings.steamGridDbKey;
    return key == null ? null : SteamGridDb(key, client: _client);
  }

  File? image(String gameId, ArtKind kind) => _files['$gameId.${kind.name}'];

  File get _settingsFile => File('${dir!.path}${Platform.pathSeparator}settings.json');

  Future<void> load() async {
    final d = dir;
    if (d == null) return;
    await d.create(recursive: true);
    _files.clear();
    await for (final f in d.list()) {
      if (f is! File) continue;
      final parts = f.uri.pathSegments.last.split('.');
      if (parts.length == 4 && (parts[1] == 'cover' || parts[1] == 'hero')) {
        _files['${parts[0]}.${parts[1]}'] = f;
      }
    }
    try {
      final json = jsonDecode(await _settingsFile.readAsString()) as Map<String, dynamic>;
      _settings = ArtSettings(
        steamGridDbKey: json['steamgriddb_key'] as String?,
        autoFetch: json['auto_fetch'] as bool? ?? true,
        wideCovers: json['wide_covers'] as bool? ?? false,
      );
      _tried.addAll([for (final t in json['tried'] as List? ?? const []) t as String]);
    } on Object {
      // No settings yet, or unreadable: start from defaults.
    }
    notifyListeners();
  }

  Future<void> _saveSettings() async {
    if (dir == null) return;
    final tmp = File('${_settingsFile.path}.tmp');
    await tmp.writeAsString(
      jsonEncode({
        'steamgriddb_key': _settings.steamGridDbKey,
        'auto_fetch': _settings.autoFetch,
        'wide_covers': _settings.wideCovers,
        'tried': _tried.toList(),
      }),
      flush: true,
    );
    await tmp.rename(_settingsFile.path);
  }

  Future<void> updateSettings(ArtSettings s) async {
    final shapeChanged = s.wideCovers != _settings.wideCovers;
    final keyChanged = s.steamGridDbKey != _settings.steamGridDbKey;
    _settings = s;
    // A new key or cover shape deserves another look for every game.
    if (shapeChanged || keyChanged) _tried.clear();
    await _saveSettings();
    notifyListeners();
  }

  /// Store image [bytes] as [kind] art for [gameId], replacing any.
  Future<void> setImage(String gameId, ArtKind kind, Uint8List bytes) async {
    final d = dir;
    if (d == null) throw ArtError('Art cannot be saved on this device.');
    final ext = imageExtension(bytes);
    if (ext == null) throw ArtError('That file is not a PNG, JPEG, WebP or GIF image.');
    if (bytes.length > maxImageBytes) throw ArtError('That image is too large.');
    final stamp = DateTime.now().microsecondsSinceEpoch;
    final file = File('${d.path}${Platform.pathSeparator}$gameId.${kind.name}.$stamp.$ext');
    await file.writeAsBytes(bytes, flush: true);
    final old = _files['$gameId.${kind.name}'];
    _files['$gameId.${kind.name}'] = file;
    notifyListeners();
    if (old != null) unawaited(old.delete().then((_) {}, onError: (_) {}));
  }

  Future<void> removeImage(String gameId, ArtKind kind) async {
    final old = _files.remove('$gameId.${kind.name}');
    if (old == null) return;
    // Removed on purpose: do not fetch it straight back.
    _tried.add(gameId);
    await _saveSettings();
    notifyListeners();
    await old.delete().then((_) {}, onError: (_) {});
  }

  /// Best SteamGridDB match for [name]: the first search result's top image.
  Future<bool> fetchBest(String gameId, String name, ArtKind kind) async {
    final api = steamGridDb;
    final term = searchTerm(name);
    if (api == null || term == null) return false;
    final games = await api.search(term);
    if (games.isEmpty) return false;
    final images = await api.images(games.first.id, kind, wide: _settings.wideCovers);
    if (images.isEmpty) return false;
    await setImage(gameId, kind, await api.download(images.first.url));
    return true;
  }

  /// With a key and automatic art on, look up a cover and banner for each
  /// game that has no cover and was not looked up before. Quiet: failures
  /// leave the placeholder.
  Future<void> fetchMissing(List<Game> games, {bool force = false}) async {
    if (_fetching || steamGridDb == null || (!_settings.autoFetch && !force)) return;
    _fetching = true;
    try {
      for (final g in games) {
        if (image(g.id, ArtKind.cover) != null || (_tried.contains(g.id) && !force)) continue;
        _tried.add(g.id);
        try {
          if (await fetchBest(g.id, g.name, ArtKind.cover) && image(g.id, ArtKind.hero) == null) {
            await fetchBest(g.id, g.name, ArtKind.hero);
          }
        } on ArtError {
          // A bad key or no connection: stop here, try again next time.
          _tried.remove(g.id);
          break;
        }
      }
      await _saveSettings();
    } finally {
      _fetching = false;
    }
  }
}
