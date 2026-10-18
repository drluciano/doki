/// Thin views over the engine's JSON results (see `crates/doki-api`).
library;

String shortId(String id) => id.length > 8 ? id.substring(0, 8) : id;

class Checkpoint {
  Checkpoint(this.id, this.json);
  final String id;
  final Map<String, dynamic> json;

  String get kind => json['kind'] as String;
  String get deviceName => (json['origin_device_name'] as String?) ?? 'another device';
  DateTime get capturedAt => DateTime.parse(json['captured_at'] as String).toLocal();
  String? get note => json['note'] as String?;
  List<String> get parents => [for (final p in json['parents'] as List) p as String];
  List<Map<String, dynamic>> get files => [
    for (final f in json['files'] as List) f as Map<String, dynamic>,
  ];
  int get totalBytes => files.fold(0, (sum, f) => sum + (f['size'] as num).toInt());
}

/// Game status as computed by the `games` command.
enum GameStatus { protected, uncapturedChanges, noCheckpoint, needsRecovery, unavailable }

class Game {
  Game(this.json);
  final Map<String, dynamic> json;

  Map<String, dynamic> get _record => json['game'] as Map<String, dynamic>;
  String get id => _record['bundle_id'] as String;
  String get name => _record['name'] as String;
  String get location => _record['location'] as String;
  String? get head => _record['head'] as String?;
  int get incoming => (json['incoming'] as num).toInt();
  String get rawStatus => json['status'] as String;

  GameStatus get status => switch (rawStatus) {
    'protected' => GameStatus.protected,
    'uncaptured_changes' => GameStatus.uncapturedChanges,
    'no_checkpoint' => GameStatus.noCheckpoint,
    'needs_recovery' => GameStatus.needsRecovery,
    _ => GameStatus.unavailable,
  };

  /// Error code when [status] is unavailable, e.g. `PermissionLost`.
  String? get unavailableCode =>
      rawStatus.startsWith('unavailable:') ? rawStatus.substring(12) : null;
}

class HistoryEntry {
  HistoryEntry(Map<String, dynamic> json)
    : checkpoint = Checkpoint(json['id'] as String, json['checkpoint'] as Map<String, dynamic>),
      isHead = json['is_head'] == true,
      isTip = json['is_tip'] == true,
      setAside = json['set_aside'] == true,
      pinned = json['pinned'] == true;

  final Checkpoint checkpoint;
  final bool isHead, isTip, setAside, pinned;
}

/// How incoming progress relates to this device's current save.
enum Relation { none, same, ahead, behind, divergent, unrelated, unknown }

Relation parseRelation(dynamic json) {
  if (json == null) return Relation.none;
  return switch ((json as Map)['kind']) {
    'same' => Relation.same,
    'ahead' => Relation.ahead,
    'behind' => Relation.behind,
    'divergent' => Relation.divergent,
    'unrelated' => Relation.unrelated,
    _ => Relation.unknown,
  };
}

class IncomingItem {
  IncomingItem(Map<String, dynamic> json)
    : gameId = (json['proposal'] as Map)['bundle_id'] as String,
      fromDevice = (json['proposal'] as Map)['from_device'] as String,
      receivedAt = DateTime.parse((json['proposal'] as Map)['received_at'] as String).toLocal(),
      checkpoint = Checkpoint(
        (json['proposal'] as Map)['checkpoint'] as String,
        json['checkpoint'] as Map<String, dynamic>,
      ),
      relation = parseRelation(json['relation']);

  final String gameId;
  final String fromDevice;
  final DateTime receivedAt;
  final Checkpoint checkpoint;
  final Relation relation;
}

class Unmapped {
  Unmapped(Map<String, dynamic> json)
    : gameId = json['bundle_id'] as String,
      nameHint = json['name_hint'] as String,
      fromDevice = json['from_device'] as String,
      hasProgress = json['has_progress'] as bool? ?? true;
  final String gameId, nameHint, fromDevice;

  /// False when the other device only has the game set up, with no
  /// checkpoint yet.
  final bool hasProgress;
}

/// A review result. [json] must be passed back unchanged to `use`.
class Review {
  Review(this.json);
  final Map<String, dynamic> json;

  Checkpoint get incoming =>
      Checkpoint(json['incoming'] as String, json['incoming_checkpoint'] as Map<String, dynamic>);
  String? get baselineHead => json['baseline_head'] as String?;
  Relation get relation => parseRelation(json['relation']);
  String? get capturedLocalChanges => json['captured_local_changes'] as String?;
  bool get isProposal => json['proposal'] != null;
}

/// Another device in this device's group (`peer_list`).
class Peer {
  Peer(Map<String, dynamic> json)
    : id = json['id'] as String,
      name = json['name'] as String,
      introducedBy = json['introduced_by'] as String?,
      lastSync = DateTime.tryParse(json['last_sync'] as String? ?? '')?.toLocal();
  final String id, name;

  /// Name of the device that introduced this one; null if paired directly.
  final String? introducedBy;
  final DateTime? lastSync;
}

/// Codes from `peer_invite`, open for 15 minutes.
class Invitation {
  Invitation(Map<String, dynamic> json)
    : code = json['code'] as String,
      short = json['short'] as String,
      expires = DateTime.now().add(const Duration(minutes: 15));

  /// Works from any network.
  final String code;

  /// `XXXX-XXXX`; works on the same network only.
  final String short;
  final DateTime expires;
}

/// Result of a `peer_sync`, in counts the UI can put into words.
class PeerSyncReport {
  factory PeerSyncReport(Map<String, dynamic> json) {
    final sessions = [for (final s in json['sessions'] as List) s as Map<String, dynamic>];
    int receivedFrom(Map<String, dynamic> s) {
      final r = s['received'] as Map<String, dynamic>?;
      if (r == null) return 0;
      return (r['new_proposals'] as List).length + (r['unmapped'] as List).length;
    }

    return PeerSyncReport._(
      reached: [for (final s in sessions) s['peer_name'] as String],
      sent: sessions.fold(0, (n, s) => n + (s['sent'] as int)),
      received: sessions.fold(0, (n, s) => n + receivedFrom(s)),
      unreachable: [for (final u in json['unreachable'] as List) (u as List)[0] as String],
    );
  }

  PeerSyncReport._({
    required this.reached,
    required this.sent,
    required this.received,
    required this.unreachable,
  });

  /// Names of devices synced with.
  final List<String> reached;

  /// Games whose current save was sent, and new offers received.
  final int sent, received;

  /// Names of devices that could not be reached (usually offline).
  final List<String> unreachable;
}
