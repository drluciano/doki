import 'dart:async';

import 'package:flutter/widgets.dart';

import 'backend.dart';
import 'models.dart';

/// What the app knows about the vault, refreshed from the engine after
/// every action. Holds no state the engine does not also hold, apart from
/// the open invitation and the outcome of the last sync.
class AppState extends ChangeNotifier {
  /// With [autoSync], syncs with paired devices when the app opens, every
  /// [syncEvery] while it is open, and when it comes back to the
  /// foreground. Received progress only ever lands in Incoming.
  ///
  /// With [autoCapture], asks the engine every [checkEvery] to checkpoint
  /// save folders that changed and then stopped changing (`auto_check`).
  /// On Android a background service does the same while the app is closed.
  AppState(
    this.backend, {
    this.autoSync = false,
    this.syncEvery = const Duration(minutes: 2),
    this.autoCapture = false,
    this.checkEvery = const Duration(seconds: 15),
  });

  final DokiBackend backend;
  final bool autoSync;
  final Duration syncEvery;
  final bool autoCapture;
  final Duration checkEvery;

  bool loaded = false;
  String? deviceName;
  List<Game> games = [];
  List<IncomingItem> incoming = [];
  List<Unmapped> unmapped = [];
  List<Peer> peers = [];

  /// Pairing code of the optional self-hosted relay, if this device uses one.
  String? pairingCode;
  Invitation? invitation;
  DateTime? lastSync;
  PeerSyncReport? lastSyncReport;
  bool syncing = false;
  Object? loadError;

  /// Whether the background watcher runs when the app is closed; null where
  /// the platform has none (desktop watches while the app is open).
  bool? backgroundWatching;

  Timer? _timer;
  Timer? _checkTimer;
  bool _checking = false;
  AppLifecycleListener? _lifecycle;

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
      // The first peer command also starts this device's peer node, so
      // paired devices can reach it while the app is open.
      peers = [
        for (final p in await backend.call('peer_list') as List) Peer(p as Map<String, dynamic>),
      ];
      final code = await backend.call('relay_code');
      pairingCode = code == null ? null : code['pairing_code'] as String;
      loadError = null;
    } catch (e) {
      loadError = e;
    }
    loaded = true;
    notifyListeners();
    _startAutoSync();
    _startAutoCapture();
  }

  Future<void> init(String name) async {
    await backend.call('init', {'name': name});
    await refresh();
  }

  /// Open pairing on this device and keep the codes to show.
  Future<Invitation> invite() async {
    invitation = Invitation(await backend.call('peer_invite') as Map<String, dynamic>);
    notifyListeners();
    return invitation!;
  }

  /// Join the group of the device showing [code]. Returns its name.
  Future<String> join(String code) async {
    final r = await backend.call('peer_join', {'code': code}) as Map<String, dynamic>;
    lastSync = DateTime.now();
    await refresh();
    return r['name'] as String;
  }

  Future<void> removePeer(String id) async {
    await backend.call('peer_remove', {'id': id});
    await refresh();
  }

  /// Exchange progress with every paired device that is online, and with
  /// the relay if this device uses one. Throws only if nothing could run.
  Future<PeerSyncReport?> sync() async {
    if (syncing) return lastSyncReport;
    syncing = true;
    notifyListeners();
    try {
      final report = PeerSyncReport(await backend.call('peer_sync') as Map<String, dynamic>);
      lastSyncReport = report;
      if (report.reached.isNotEmpty) lastSync = DateTime.now();
      if (pairingCode != null) {
        try {
          await backend.call('sync');
          lastSync = DateTime.now();
        } on DokiError {
          // Report a relay failure only when it was the only way to sync.
          if (peers.isEmpty) rethrow;
        }
      }
      return report;
    } finally {
      syncing = false;
      await refresh();
    }
  }

  Future<void> _quietSync() async {
    if (!initialized || (peers.isEmpty && pairingCode == null)) return;
    try {
      await sync();
    } catch (_) {
      // Background sync failures surface as "last synced" going stale and
      // in the Devices screen; never as an interruption.
    }
  }

  void _startAutoSync() {
    if (!autoSync || !initialized || _timer != null) return;
    _timer = Timer.periodic(syncEvery, (_) => _quietSync());
    _lifecycle = AppLifecycleListener(onResume: _quietSync);
    unawaited(_quietSync());
  }

  /// Look for settled changes now. Refreshes when a checkpoint was taken.
  Future<void> autoCheck() async {
    if (!initialized || games.isEmpty || _checking) return;
    _checking = true;
    try {
      final checks = await backend.call('auto_check') as List;
      if (checks.any((c) => (c as Map)['outcome'] == 'created')) await refresh();
    } catch (_) {
      // Per-game problems already show as the game's status.
    } finally {
      _checking = false;
    }
  }

  Future<void> setBackgroundWatching(bool on) async {
    await backend.setBackgroundWatching(on);
    backgroundWatching = on;
    notifyListeners();
  }

  void _startAutoCapture() {
    if (!autoCapture || !initialized || _checkTimer != null) return;
    _checkTimer = Timer.periodic(checkEvery, (_) => autoCheck());
    unawaited(() async {
      backgroundWatching = await backend.backgroundWatching();
      notifyListeners();
    }());
  }

  @override
  void dispose() {
    _checkTimer?.cancel();
    _timer?.cancel();
    _lifecycle?.dispose();
    super.dispose();
  }
}
