import 'dart:async';

import 'package:flutter/widgets.dart';

import 'art.dart';
import 'backend.dart';
import 'models.dart';

/// What the app knows about the vault, refreshed from the engine after
/// every action. Holds no state the engine does not also hold, apart from
/// the open invitation, the outcome of the last sync and game art ([art],
/// which lives only on this device).
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
    ArtStore? art,
  }) : art = art ?? ArtStore.disabled();

  final DokiBackend backend;
  final bool autoSync;
  final Duration syncEvery;
  final bool autoCapture;
  final Duration checkEvery;

  /// Covers and banners for games, and the SteamGridDB settings.
  final ArtStore art;

  bool loaded = false;
  String? deviceName;
  List<Game> games = [];
  List<IncomingItem> incoming = [];
  List<Unmapped> unmapped = [];
  List<SystemEntry> systems = [];
  List<RemoteSystem> remoteSystems = [];
  List<Peer> peers = [];

  /// Pairing code of the optional self-hosted relay, if this device uses one.
  String? pairingCode;
  Invitation? invitation;
  DateTime? lastSync;
  PeerSyncReport? lastSyncReport;
  bool syncing = false;
  Object? loadError;

  /// Whether Doki keeps working when its window is closed (Android's
  /// background service, Windows' tray); null where the platform has neither.
  bool? backgroundWatching;

  /// Whether Doki starts when the user signs in; null where not managed.
  bool? startWithSystem;

  /// Retention and notification choices for this device.
  Settings settings = const Settings();

  /// Incoming progress already seen, so only new arrivals notify. Null
  /// until the first refresh, which seeds it without notifying.
  Set<String>? _seenIncoming;

  Timer? _timer;
  Timer? _checkTimer;
  bool _checking = false;
  AppLifecycleListener? _lifecycle;

  bool get initialized => deviceName != null;

  Game? game(String id) => games.where((g) => g.id == id).firstOrNull;

  SystemEntry? system(String id) => systems.where((s) => s.id == id).firstOrNull;

  /// Games found in the system [id], by name.
  List<Game> gamesIn(String id) =>
      games.where((g) => g.systemId == id).toList()
        ..sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));

  /// Games whose folder was chosen on their own.
  List<Game> get standaloneGames => games.where((g) => g.systemId == null).toList();

  /// Games from other devices to offer for setup one by one: those that
  /// belong to a system on the other device are set up with that system.
  List<Unmapped> get unmappedGames {
    final offered = {for (final r in remoteSystems) r.id};
    return unmapped.where((u) => !offered.contains(u.systemId)).toList();
  }

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
      systems = [
        for (final s in await backend.call('systems') as List)
          SystemEntry(s as Map<String, dynamic>),
      ]..sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
      remoteSystems = [
        for (final r in await backend.call('remote_systems') as List)
          RemoteSystem(r as Map<String, dynamic>),
      ];
      // The first peer command also starts this device's peer node, so
      // paired devices can reach it while the app is open.
      peers = [
        for (final p in await backend.call('peer_list') as List) Peer(p as Map<String, dynamic>),
      ];
      final code = await backend.call('relay_code');
      pairingCode = code == null ? null : code['pairing_code'] as String;
      try {
        settings = Settings.fromJson(await backend.call('settings') as Map<String, dynamic>);
      } on DokiError {
        settings = const Settings(); // An engine without settings yet.
      }
      _notifyNewIncoming();
      loadError = null;
    } catch (e) {
      loadError = e;
    }
    loaded = true;
    notifyListeners();
    _startAutoSync();
    _startAutoCapture();
    unawaited(art.fetchMissing(games));
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

  /// Look for settled changes, and new games in system folders, now.
  /// Refreshes when a checkpoint was taken or a game was found, and notifies
  /// about changes found and checkpoints saved as the user chose in
  /// [settings].
  Future<void> autoCheck() async {
    if (!initialized || (games.isEmpty && systems.isEmpty) || _checking) return;
    _checking = true;
    try {
      final checks = (await backend.call('auto_check') as List).cast<Map<String, dynamic>>();
      for (final c in checks) {
        final key = 'game:${c['bundle_id']}';
        final name = c['name'] as String;
        if (c['outcome'] == 'detected' && settings.notifyChanges) {
          await backend.notify(
            key,
            'New progress in $name',
            'Doki will save a checkpoint once the game stops writing.',
          );
        } else if (c['outcome'] == 'created' && settings.notifyCheckpoints) {
          await backend.notify(key, 'Checkpoint saved', '$name is protected up to now.');
        }
      }
      final known = {for (final g in games) g.id};
      if (checks.any((c) => c['outcome'] == 'created' || !known.contains(c['bundle_id']))) {
        await refresh();
      }
    } catch (_) {
      // Per-game problems already show as the game's status.
    } finally {
      _checking = false;
    }
  }

  void _notifyNewIncoming() {
    final now = {for (final i in incoming) '${i.gameId}.${i.checkpoint.id}'};
    final seen = _seenIncoming;
    _seenIncoming = now;
    if (seen == null || !settings.notifyIncoming) return;
    for (final i in incoming) {
      if (seen.contains('${i.gameId}.${i.checkpoint.id}')) continue;
      final name = game(i.gameId)?.name ?? 'a game';
      unawaited(
        backend.notify(
          'incoming:${i.gameId}',
          'Progress from ${i.fromDevice}',
          'New progress for $name is waiting in Incoming. Nothing changes until you choose.',
        ),
      );
    }
  }

  /// Change notification choices or the retention time. A shorter
  /// retention time takes effect at once (the caller shows a preview first).
  Future<void> updateSettings(Map<String, dynamic> changes) async {
    settings = Settings.fromJson(
      await backend.call('set_settings', changes) as Map<String, dynamic>,
    );
    if (changes.containsKey('keep_days')) await backend.call('apply_retention');
    notifyListeners();
  }

  /// Delete a removed game and its checkpoints from this device.
  Future<void> forgetGame(String id) async {
    await backend.call('forget', {'game': id});
    await refresh();
  }

  Future<void> setBackgroundWatching(bool on) async {
    await backend.setBackgroundWatching(on);
    backgroundWatching = on;
    notifyListeners();
  }

  Future<void> setStartWithSystem(bool on) async {
    await backend.setStartWithSystem(on);
    startWithSystem = on;
    notifyListeners();
  }

  void _startAutoCapture() {
    if (!autoCapture || !initialized || _checkTimer != null) return;
    _checkTimer = Timer.periodic(checkEvery, (_) => autoCheck());
    unawaited(() async {
      backgroundWatching = await backend.backgroundWatching();
      startWithSystem = await backend.startWithSystem();
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
