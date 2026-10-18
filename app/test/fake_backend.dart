import 'package:doki/backend.dart';

/// In-memory stand-in for the engine, for widget tests. Each command maps to
/// a handler; unhandled commands fail like an unknown request would.
class FakeBackend implements DokiBackend {
  FakeBackend(this.handlers);

  final Map<String, dynamic Function(Map<String, dynamic> args)> handlers;
  final List<String> calls = [];
  PickedFolder? folder;

  @override
  Future<dynamic> call(String cmd, [Map<String, dynamic> args = const {}]) async {
    calls.add(cmd);
    final h = handlers[cmd];
    if (h == null) throw DokiError('InvalidInput', 'unhandled $cmd');
    return h(args);
  }

  @override
  Future<PickedFolder?> pickFolder() async => folder;

  bool? watching;

  @override
  Future<bool?> backgroundWatching() async => watching;

  @override
  Future<void> setBackgroundWatching(bool on) async => watching = on;

  bool? startsWithSystem;

  @override
  Future<bool?> startWithSystem() async => startsWithSystem;

  @override
  Future<void> setStartWithSystem(bool on) async => startsWithSystem = on;
}

Map<String, dynamic> checkpoint({
  String device = 'Odin',
  String kind = 'capture',
  List<String> parents = const [],
  String? note,
}) => {
  'schema_version': 1,
  'bundle_id': 'b1',
  'parents': parents,
  'kind': kind,
  'files': [
    {'path': 'game.sav', 'hash': 'sha256:00', 'size': 2048},
  ],
  'profile': {'id': 'generic-folder', 'version': 1},
  'origin_device': 'd1',
  'origin_device_name': device,
  'captured_at': '2026-10-18T10:00:00Z',
  'capture_mode': 'emulator-stopped-confirmed',
  'note': ?note,
};

Map<String, dynamic> game(
  String status, {
  int incoming = 0,
  String? head = 'c1',
  String id = 'b1',
  String name = 'Emerald',
  String? system,
}) => {
  'game': {
    'bundle_id': id,
    'name': name,
    if (system != null) 'member': {'system_id': system, 'kind': 'folder', 'name': name},
    'location': '/saves/emerald',
    'profile': {'id': 'generic-folder', 'version': 1},
    'head': head,
    'pins': <String>[],
    'created_at': '2026-10-18T09:00:00Z',
  },
  'status': status,
  'incoming': incoming,
};
