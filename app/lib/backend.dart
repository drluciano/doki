/// Connection to the Doki engine.
///
/// The app never makes safety decisions itself: every action is one JSON
/// request to the Rust engine (see `crates/doki-api`). On Android the request
/// goes through a platform channel to the Java bridge, which supplies save
/// folders through the Storage Access Framework. On Windows and Linux it goes
/// straight to the `doki_ffi` library through Dart FFI.
library;

import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

/// An error reported by the engine. [code] is a stable identifier from the
/// safety contract (section 11).
class DokiError implements Exception {
  DokiError(this.code, this.message, {this.saveMayBeMixed = false});

  final String code;
  final String message;

  /// The playable save may be part old, part new until recovery runs.
  final bool saveMayBeMixed;

  @override
  String toString() => '$code: $message';
}

/// A save folder the user picked. [location] is what the engine stores
/// (a path on desktop, a document tree URI on Android).
class PickedFolder {
  const PickedFolder(this.location, this.displayName);
  final String location;
  final String displayName;
}

abstract class DokiBackend {
  /// Run one engine command and return its `result`, or throw [DokiError].
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]);

  /// Ask the user for a save folder. Null when they cancel.
  Future<PickedFolder?> pickFolder();

  /// Whether save folders are watched while the app is closed, or null
  /// where that is not available (Linux). On Android this is the background
  /// service; on Windows it is "keep in tray": closing or minimizing hides
  /// Doki to the notification area instead of quitting.
  Future<bool?> backgroundWatching();

  Future<void> setBackgroundWatching(bool on);

  /// Whether Doki starts (in the tray) when the user signs in, or null where
  /// the app does not manage that (everywhere but Windows).
  Future<bool?> startWithSystem();

  Future<void> setStartWithSystem(bool on);

  /// Show a system notification. [key] identifies what it is about (a game
  /// id), so a newer notification about the same thing replaces the older
  /// one. Android posts a notification, Windows shows it from the tray
  /// icon; elsewhere, or with notifications blocked, nothing is shown.
  Future<void> notify(String key, String title, String body);

  static Future<DokiBackend> forPlatform() async {
    if (Platform.isAndroid) return ChannelBackend();
    final support = await getApplicationSupportDirectory();
    return FfiBackend('${support.path}${Platform.pathSeparator}vault');
  }
}

dynamic decodeResponse(String response) {
  final json = jsonDecode(response) as Map<String, dynamic>;
  if (json['ok'] == true) return json['result'];
  final e = json['error'] as Map<String, dynamic>;
  throw DokiError(
    e['code'] as String,
    e['message'] as String,
    saveMayBeMixed: e['save_may_be_mixed'] == true,
  );
}

String encodeRequest(String cmd, Map<String, dynamic> args) => jsonEncode({'cmd': cmd, ...args});

/// Commands that wait on other devices, for up to about 20 seconds. They run
/// in their own lane so a slow network never holds up the rest of the app;
/// the engine's vault locks keep the two lanes safe to overlap.
const networkCommands = {'peer_invite', 'peer_join', 'peer_sync'};

/// Android: the Java side runs the engine on a background thread.
class ChannelBackend implements DokiBackend {
  static const _channel = MethodChannel('dev.doki/engine');

  @override
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]) async {
    final response = await _channel.invokeMethod<String>('call', {
      'request': encodeRequest(cmd, args),
      'lane': networkCommands.contains(cmd) ? 'network' : 'vault',
    });
    return decodeResponse(response!);
  }

  @override
  Future<PickedFolder?> pickFolder() async {
    try {
      final picked = await _channel.invokeMapMethod<String, String>('pickFolder');
      if (picked == null) return null;
      return PickedFolder(picked['location']!, picked['name'] ?? picked['location']!);
    } on PlatformException catch (e) {
      throw DokiError(e.code, e.message ?? 'Could not open the folder picker');
    }
  }

  @override
  Future<bool?> backgroundWatching() => _channel.invokeMethod<bool>('isWatching');

  @override
  Future<void> setBackgroundWatching(bool on) =>
      _channel.invokeMethod<void>('setWatching', {'enabled': on});

  @override
  Future<bool?> startWithSystem() async => null;

  @override
  Future<void> setStartWithSystem(bool on) async {}

  @override
  Future<void> notify(String key, String title, String body) =>
      _channel.invokeMethod<void>('notify', {'key': key, 'title': title, 'body': body});
}

typedef _CallNative = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

/// Path of the engine library bundled with the desktop app: next to the
/// executable on Windows, in `lib/` beside it on Linux. `DOKI_FFI_LIB`
/// overrides it (used by tests against a local build).
String _libraryPath() {
  final override = Platform.environment['DOKI_FFI_LIB'];
  if (override != null && override.isNotEmpty) return override;
  final dir = File(Platform.resolvedExecutable).parent.path;
  final sep = Platform.pathSeparator;
  return Platform.isWindows ? '$dir${sep}doki_ffi.dll' : '$dir${sep}lib${sep}libdoki_ffi.so';
}

String _callBlocking(String library, String vault, String request) {
  final lib = DynamicLibrary.open(library);
  final call = lib.lookupFunction<_CallNative, _CallNative>('doki_call');
  final free = lib.lookupFunction<_FreeNative, _FreeDart>('doki_free');
  final v = vault.toNativeUtf8();
  final r = request.toNativeUtf8();
  try {
    final out = call(v, r);
    if (out == nullptr) {
      return '{"ok":false,"error":{"code":"Internal","message":"no response","save_may_be_mixed":false}}';
    }
    final s = out.toDartString();
    free(out);
    return s;
  } finally {
    malloc.free(v);
    malloc.free(r);
  }
}

/// Windows and Linux: calls `doki_ffi` on a background isolate.
class FfiBackend implements DokiBackend {
  FfiBackend(this.vaultPath, {String? library}) : library = library ?? _libraryPath();

  final String vaultPath;
  final String library;

  // Requests in each lane run one at a time so the UI never races its own
  // operations; see [networkCommands].
  Future<void> _vaultQueue = Future.value();
  Future<void> _networkQueue = Future.value();

  @override
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]) {
    final request = encodeRequest(cmd, args);
    final lib = library, vault = vaultPath;
    final network = networkCommands.contains(cmd);
    final queue = network ? _networkQueue : _vaultQueue;
    final result = queue.then((_) => Isolate.run(() => _callBlocking(lib, vault, request)));
    final done = result.then((_) {}, onError: (_) {});
    if (network) {
      _networkQueue = done;
    } else {
      _vaultQueue = done;
    }
    return result.then(decodeResponse);
  }

  @override
  Future<PickedFolder?> pickFolder() async {
    final path = await getDirectoryPath(confirmButtonText: 'Choose this folder');
    if (path == null) return null;
    final name = path.split(RegExp(r'[\\/]')).where((s) => s.isNotEmpty).lastOrNull ?? path;
    return PickedFolder(path, name);
  }

  // The Windows runner keeps Doki in the tray (windows/runner/flutter_window.h).
  // Linux has no tray yet, so Doki watches only while its window is open.
  static const _window = MethodChannel('dev.doki/window');

  Future<bool?> _windowSetting(String name) async {
    if (!Platform.isWindows) return null;
    try {
      return await _window.invokeMethod<bool>('get$name');
    } on MissingPluginException {
      return null; // Tests and tools that run without the runner.
    }
  }

  Future<void> _setWindowSetting(String name, bool on) async {
    if (!Platform.isWindows) return;
    try {
      await _window.invokeMethod<void>('set$name', {'enabled': on});
    } on PlatformException catch (e) {
      throw DokiError(e.code, e.message ?? 'Windows did not save the setting');
    }
  }

  @override
  Future<bool?> backgroundWatching() => _windowSetting('KeepInTray');

  @override
  Future<void> setBackgroundWatching(bool on) => _setWindowSetting('KeepInTray', on);

  @override
  Future<bool?> startWithSystem() => _windowSetting('StartWithWindows');

  @override
  Future<void> setStartWithSystem(bool on) => _setWindowSetting('StartWithWindows', on);

  @override
  Future<void> notify(String key, String title, String body) async {
    if (!Platform.isWindows) return;
    try {
      await _window.invokeMethod<void>('notify', {'key': key, 'title': title, 'body': body});
    } on MissingPluginException {
      // Running without the Windows runner (tests).
    }
  }
}
