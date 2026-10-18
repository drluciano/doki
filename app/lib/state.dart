import 'package:flutter/foundation.dart';

import 'backend.dart';
import 'models.dart';

/// What the app knows about the vault, refreshed from the engine after
/// every action. Holds no state the engine does not also hold.
class AppState extends ChangeNotifier {
  AppState(this.backend);

  final DokiBackend backend;

  bool loaded = false;
  String? deviceName;
  List<Game> games = [];
  List<IncomingItem> incoming = [];
  List<Unmapped> unmapped = [];
  String? pairingCode;
  DateTime? lastSync;
  Object? loadError;

  bool get initialized => deviceName != null;

  Game? game(String id) => games.where((g) => g.id == id).firstOrNull;

  Future<void> refresh() async {
    try {
      final device = await backend.call('device');
      deviceName = device['device_name'] as String;
    } on DokiError catch (e) {
      // A vault that does not exist yet means onboarding.
      if (e.code != 'NotFound') {
        loadError = e;
      }
      deviceName = null;
      loaded = true;
      notifyListeners();
      return;
    }
    try {
      games = [
        for (final g in await backend.call('games') as List) Game(g as Map<String, dynamic>),
      ];
      incoming = [
        for (final i in await backend.call('incoming') as List)
          IncomingItem(i as Map<String, dynamic>),
      ];
      unmapped = [
        for (final u in await backend.call('unmapped') as List) Unmapped(u as Map<String, dynamic>),
      ];
      final code = await backend.call('relay_code');
      pairingCode = code == null ? null : code['pairing_code'] as String;
      loadError = null;
    } catch (e) {
      loadError = e;
    }
    loaded = true;
    notifyListeners();
  }

  Future<void> init(String name) async {
    await backend.call('init', {'name': name});
    await refresh();
  }

  Future<Map<String, dynamic>> sync() async {
    final r = await backend.call('sync') as Map<String, dynamic>;
    lastSync = DateTime.now();
    await refresh();
    return r;
  }
}
