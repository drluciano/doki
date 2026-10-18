import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../backend.dart';
import '../copy.dart';
import '../main.dart';
import '../models.dart';
import '../state.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// This device, the devices it is paired with, pairing and sync.
///
/// Devices connect to each other directly (see docs/peer.md); there is no
/// host. Syncing delivers progress to Incoming and never installs it. The
/// self-hosted relay stays available under "Advanced".
class DevicesScreen extends StatefulWidget {
  const DevicesScreen({super.key});

  @override
  State<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends State<DevicesScreen> {
  final _code = TextEditingController();
  String? _joinProblem;
  String? _joined;

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  Future<void> _showCode() async {
    final state = AppScope.read(context);
    await withProgress(context, 'Opening pairing…', state.invite);
  }

  Future<void> _join() async {
    final state = AppScope.read(context);
    final code = _code.text.trim();
    if (code.isEmpty) return;
    setState(() {
      _joinProblem = null;
      _joined = null;
    });
    String? name;
    String? problem;
    await withProgress(context, 'Looking for the other device…', () async {
      try {
        name = await state.join(code);
      } on DokiError catch (e) {
        problem = pairingProblem(e);
      }
    });
    if (!mounted) return;
    setState(() {
      _joinProblem = problem;
      _joined = name;
      if (name != null) _code.clear();
    });
  }

  Future<void> _sync() async {
    final state = AppScope.read(context);
    await guarded(context, state.sync);
  }

  Future<void> _remove(Peer peer) async {
    final state = AppScope.read(context);
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Remove ${peer.name}?'),
        content: Text(
          '${peer.name} can no longer sync with any of your devices. Checkpoints it already sent stay here. To add it back, pair it again with a code.',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Remove')),
        ],
      ),
    );
    if (sure != true || !mounted) return;
    await guarded(context, () => state.removePeer(peer.id));
  }

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('Devices'), backgroundColor: Colors.transparent),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          ContentWidth(
            maxWidth: 640,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Card(
                  child: ListTile(
                    contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    leading: Icon(
                      Platform.isAndroid ? Icons.smartphone : Icons.computer,
                      color: DokiColors.lavender,
                    ),
                    title: Text(state.deviceName ?? ''),
                    subtitle: const Text('This device'),
                  ),
                ),
                if (state.backgroundWatching != null)
                  Card(
                    child: SwitchListTile(
                      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                      value: state.backgroundWatching!,
                      onChanged: (on) => guarded(context, () => state.setBackgroundWatching(on)),
                      title: const Text('Keep working in the background'),
                      subtitle: const Text(
                        'Saves checkpoints and syncs while you play, with a notification showing. Turn off to only work while Doki is open.',
                      ),
                    ),
                  ),
                const SizedBox(height: 16),
                _YourDevices(state: state, onSync: _sync, onRemove: _remove),
                const SizedBox(height: 16),
                _addDevice(context, state),
                const SizedBox(height: 16),
                const _RelaySection(),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _addDevice(BuildContext context, AppState state) {
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    final invitation = state.invitation;
    final open = invitation != null && invitation.expires.isAfter(DateTime.now());
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Add a device', style: text.titleMedium),
            const SizedBox(height: 4),
            Text(
              'Show a code here and enter it on the other device, or the other way around. Once paired, a device also meets the rest of your devices.',
              style: muted,
            ),
            const SizedBox(height: 16),
            if (!open)
              OutlinedButton.icon(
                onPressed: _showCode,
                icon: const Icon(Icons.key),
                label: const Text('Show a code on this device'),
              )
            else ...[
              Text('On the same Wi-Fi, enter this code', style: text.titleSmall),
              const SizedBox(height: 8),
              SelectableText(
                invitation.short,
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 32,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 4,
                  color: DokiColors.lavender,
                ),
                semanticsLabel: 'Short code ${invitation.short.split('').join(' ')}',
              ),
              const SizedBox(height: 12),
              Text('From another network, use the long code', style: text.titleSmall),
              const SizedBox(height: 4),
              Text(
                invitation.code,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: invitation.code));
                  ScaffoldMessenger.of(context)
                      .showSnackBar(const SnackBar(content: Text('Long code copied')));
                },
                icon: const Icon(Icons.copy),
                label: const Text('Copy long code'),
              ),
              const SizedBox(height: 8),
              Text(
                'Codes work until ${formatWhen(invitation.expires)} for any number of devices. Anyone with a code can join your devices, so share them only with your own.',
                style: text.bodySmall?.copyWith(color: DokiColors.muted, fontSize: 14),
              ),
            ],
            const Divider(height: 32),
            Text('Have a code from another device?', style: text.titleSmall),
            const SizedBox(height: 8),
            TextField(
              controller: _code,
              textCapitalization: TextCapitalization.characters,
              decoration: const InputDecoration(
                labelText: 'Code',
                hintText: 'ABCD-2345 or the long code',
              ),
              onSubmitted: (_) => _join(),
            ),
            const SizedBox(height: 8),
            FilledButton(onPressed: _join, child: const Text('Pair')),
            if (_joinProblem != null) ...[
              const SizedBox(height: 12),
              StatusBanner.error(_joinProblem!),
            ],
            if (_joined != null) ...[
              const SizedBox(height: 12),
              StatusBanner.success(
                'Paired with $_joined',
                detail: 'Its games appear in Incoming. Nothing on this device was changed.',
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Pairing failures in words. The engine reuses general error codes, so the
/// pairing screen gives them pairing-specific meaning.
String pairingProblem(DokiError e) => switch (e.code) {
  'InvalidInput' when e.message.contains('this device') =>
    'That code is from this device. Enter it on your other device.',
  'InvalidInput' => 'That is not a Doki code. Check it and try again.',
  'PermissionLost' => 'That code has expired or is wrong. Show a new code on the other device.',
  'SourceUnavailable' when e.message.contains('no device showing a code') => 'No device showing a code was found on this Wi-Fi. Check both are on the same network, or use the long code.',
  'SourceUnavailable' =>
    'Could not reach the other device. Check both devices are online and Doki is open on it.',
  _ => errorCopy(e.code).title,
};

class _YourDevices extends StatelessWidget {
  const _YourDevices({required this.state, required this.onSync, required this.onRemove});

  final AppState state;
  final VoidCallback onSync;
  final void Function(Peer) onRemove;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final muted = text.bodyMedium?.copyWith(color: DokiColors.muted);
    final report = state.lastSyncReport;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Your devices', style: text.titleMedium),
            const SizedBox(height: 4),
            if (state.peers.isEmpty)
              Text(
                'No other devices yet. Devices connect to each other directly, so no server or account is needed.',
                style: muted,
              )
            else
              Text(
                'Doki syncs while it is open on both devices. Received progress waits in Incoming. Sync is not a backup.',
                style: muted,
              ),
            const SizedBox(height: 8),
            for (final p in state.peers)
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.devices_other, color: DokiColors.lavender),
                title: Text(p.name),
                subtitle: Text(peerSubtitle(p, report)),
                trailing: IconButton(
                  tooltip: 'Remove ${p.name}',
                  icon: const Icon(Icons.link_off),
                  onPressed: () => onRemove(p),
                ),
              ),
            if (state.peers.isNotEmpty || state.pairingCode != null) ...[
              const SizedBox(height: 8),
              if (report != null && state.peers.isNotEmpty) ...[
                Text(syncSummary(report)),
                const SizedBox(height: 12),
              ],
              FilledButton.icon(
                onPressed: state.syncing ? null : onSync,
                icon: state.syncing
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.sync),
                label: Text(state.syncing ? 'Syncing…' : 'Sync now'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

String peerSubtitle(Peer p, PeerSyncReport? report) {
  final parts = <String>[
    if (report != null && report.unreachable.contains(p.name))
      'Not reachable right now'
    else if (p.lastSync != null)
      'Last synced ${formatWhen(p.lastSync!)}'
    else
      'Not synced yet',
    if (p.introducedBy != null) 'added through ${p.introducedBy}',
  ];
  return parts.join(' · ');
}

String syncSummary(PeerSyncReport r) {
  if (r.reached.isEmpty) {
    return 'None of your devices could be reached. Open Doki on another device and try again.';
  }
  String plural(int n, String one) => '$n $one${n == 1 ? '' : 's'}';
  final received = r.received == 0
      ? 'nothing new received'
      : '${plural(r.received, 'new save')} waiting in Incoming';
  return 'Synced with ${r.reached.join(', ')}: ${plural(r.sent, 'game')} sent, $received.';
}

/// The optional self-hosted relay from before peer-to-peer sync, for
/// devices that are rarely online at the same time.
class _RelaySection extends StatefulWidget {
  const _RelaySection();

  @override
  State<_RelaySection> createState() => _RelaySectionState();
}

class _RelaySectionState extends State<_RelaySection> {
  final _url = TextEditingController();
  final _code = TextEditingController();

  @override
  void dispose() {
    _url.dispose();
    _code.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    final state = AppScope.read(context);
    final url = _url.text.trim();
    if (url.isEmpty) return;
    await withProgress(context, 'Connecting to the relay…', () async {
      await state.backend.call('relay_create', {'url': url});
      await state.refresh();
    });
  }

  Future<void> _join() async {
    final state = AppScope.read(context);
    final code = _code.text.trim();
    if (code.isEmpty) return;
    await withProgress(context, 'Joining…', () async {
      await state.backend.call('relay_join', {'code': code});
      await state.refresh();
    });
  }

  Future<void> _forget() async {
    final state = AppScope.read(context);
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Forget this relay?'),
        content: const Text(
          'This device stops using the relay. Devices paired directly keep syncing, and checkpoints already here stay.',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(context, true), child: const Text('Forget')),
        ],
      ),
    );
    if (sure != true || !mounted) return;
    await guarded(context, () async {
      await state.backend.call('relay_forget');
      await state.refresh();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final text = Theme.of(context).textTheme;
    final code = state.pairingCode;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: ExpansionTile(
        title: const Text('Advanced: your own relay server'),
        subtitle: Text(code == null ? 'Not used' : 'In use', style: text.bodySmall),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        expandedCrossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Optional. A relay you host holds encrypted copies, so devices that are rarely online at the same time can still pass progress along. It never decides which save wins.',
            style: text.bodyMedium?.copyWith(color: DokiColors.muted),
          ),
          const SizedBox(height: 12),
          if (code == null) ...[
            TextField(
              controller: _url,
              keyboardType: TextInputType.url,
              decoration: const InputDecoration(
                labelText: 'Relay address',
                hintText: 'https://relay.example.net',
              ),
            ),
            const SizedBox(height: 8),
            OutlinedButton(onPressed: _create, child: const Text('Create a relay code')),
            const SizedBox(height: 16),
            TextField(
              controller: _code,
              decoration: const InputDecoration(labelText: 'Relay code', hintText: 'DOKI1:…'),
            ),
            const SizedBox(height: 8),
            OutlinedButton(onPressed: _join, child: const Text('Join a relay')),
          ] else ...[
            Text('Relay code for your other devices', style: text.titleSmall),
            const SizedBox(height: 4),
            SelectableText(code, style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
            const SizedBox(height: 8),
            Wrap(
              spacing: 12,
              children: [
                OutlinedButton.icon(
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: code));
                    ScaffoldMessenger.of(context)
                        .showSnackBar(const SnackBar(content: Text('Relay code copied')));
                  },
                  icon: const Icon(Icons.copy),
                  label: const Text('Copy code'),
                ),
                TextButton(onPressed: _forget, child: const Text('Forget this relay')),
              ],
            ),
          ],
        ],
      ),
    );
  }
}
