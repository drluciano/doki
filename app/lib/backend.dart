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

/// Android: the Java side runs the engine on a background thread.
class ChannelBackend implements DokiBackend {
  static const _channel = MethodChannel('dev.doki/engine');

  @override
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]) async {
    final response = await _channel.invokeMethod<String>('call', {
      'request': encodeRequest(cmd, args),
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

  // Requests run one at a time so the UI never races its own operations.
  Future<void> _queue = Future.value();

  @override
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]) {
    final request = encodeRequest(cmd, args);
    final lib = library, vault = vaultPath;
    final result = _queue.then((_) => Isolate.run(() => _callBlocking(lib, vault, request)));
    _queue = result.then((_) {}, onError: (_) {});
    return result.then(decodeResponse);
  }

  @override
  Future<PickedFolder?> pickFolder() async {
    final path = await getDirectoryPath(confirmButtonText: 'Choose this folder');
    if (path == null) return null;
    final name = path.split(RegExp(r'[\\/]')).where((s) => s.isNotEmpty).lastOrNull ?? path;
    return PickedFolder(path, name);
  }
}
