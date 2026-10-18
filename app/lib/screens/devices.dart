import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../copy.dart';
import '../main.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// This device, relay pairing and sync. The relay only ever holds
/// encrypted copies; syncing delivers progress to Incoming and never
/// installs it.
class DevicesScreen extends StatefulWidget {
  const DevicesScreen({super.key});

  @override
  State<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends State<DevicesScreen> {
  final _url = TextEditingController();
  final _code = TextEditingController();
  String? _lastResult;

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

  Future<void> _sync() async {
    final state = AppScope.read(context);
    final r = await withProgress(context, 'Sending and receiving…', state.sync);
    if (r == null || !mounted) return;
    final rejected = r['rejected'] as num;
    setState(
      () => _lastResult =
          'Sent ${r['uploaded']}, received ${r['received']}'
          '${rejected > 0 ? ', $rejected could not be read and were skipped' : ''}.',
    );
  }

  Future<void> _forget() async {
    final state = AppScope.read(context);
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Forget this relay?'),
        content: const Text(
          'This device stops syncing. Checkpoints already here stay. You can pair again with a code.',
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
                const SizedBox(height: 24),
                if (code == null) ...[
                  Text('Connect your devices', style: text.titleMedium),
                  const SizedBox(height: 4),
                  Text(
                    'Devices exchange progress through a relay you host. It stores only encrypted copies and never decides which save wins.',
                    style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                  ),
                  const SizedBox(height: 16),
                  Text('First device', style: text.titleSmall),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _url,
                    keyboardType: TextInputType.url,
                    decoration: const InputDecoration(
                      labelText: 'Relay address',
                      hintText: 'https://relay.example.net',
                    ),
                  ),
                  const SizedBox(height: 8),
                  FilledButton(onPressed: _create, child: const Text('Create a pairing code')),
                  const SizedBox(height: 24),
                  Text('Another device already has a code', style: text.titleSmall),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _code,
                    decoration: const InputDecoration(
                      labelText: 'Pairing code',
                      hintText: 'DOKI1:…',
                    ),
                  ),
                  const SizedBox(height: 8),
                  OutlinedButton(onPressed: _join, child: const Text('Join')),
                ] else ...[
                  Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Sync', style: text.titleMedium),
                          const SizedBox(height: 4),
                          Text(
                            state.lastSync == null
                                ? 'Not synced since Doki opened.'
                                : 'Last successful exchange ${formatWhen(state.lastSync!)}.',
                            style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                          ),
                          if (_lastResult != null) ...[
                            const SizedBox(height: 8),
                            Text(_lastResult!),
                          ],
                          const SizedBox(height: 12),
                          FilledButton.icon(
                            onPressed: _sync,
                            icon: const Icon(Icons.sync),
                            label: const Text('Sync now'),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Received progress waits in Incoming. Sync is not a backup.',
                            style: text.bodySmall?.copyWith(color: DokiColors.muted, fontSize: 14),
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  Card(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Add another device', style: text.titleMedium),
                          const SizedBox(height: 4),
                          Text(
                            'Enter this code on the other device. Anyone with it can read your synced saves, so keep it private.',
                            style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                          ),
                          const SizedBox(height: 12),
                          Center(
                            child: Container(
                              color: Colors.white,
                              padding: const EdgeInsets.all(8),
                              child: QrImageView(
                                data: code,
                                size: 200,
                                semanticsLabel: 'Pairing code as a QR code',
                              ),
                            ),
                          ),
                          const SizedBox(height: 8),
                          SelectableText(
                            code,
                            style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
                          ),
                          const SizedBox(height: 8),
                          Wrap(
                            spacing: 12,
                            children: [
                              OutlinedButton.icon(
                                onPressed: () {
                                  Clipboard.setData(ClipboardData(text: code));
                                  ScaffoldMessenger.of(context).showSnackBar(
                                    const SnackBar(content: Text('Pairing code copied')),
                                  );
                                },
                                icon: const Icon(Icons.copy),
                                label: const Text('Copy code'),
                              ),
                              TextButton(
                                onPressed: _forget,
                                child: const Text('Forget this relay'),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}
