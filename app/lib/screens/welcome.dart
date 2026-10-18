import 'dart:io';

import 'package:flutter/material.dart';

import '../main.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// First run: the safety promise and a local device name. No account or
/// network is needed for local use.
class WelcomeScreen extends StatefulWidget {
  const WelcomeScreen({super.key});

  @override
  State<WelcomeScreen> createState() => _WelcomeScreenState();
}

class _WelcomeScreenState extends State<WelcomeScreen> {
  late final _name = TextEditingController(
    text: Platform.isAndroid ? 'My handheld' : (Platform.isWindows ? 'My PC' : 'My computer'),
  );
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _start() async {
    final name = _name.text.trim();
    if (name.isEmpty) return;
    setState(() => _busy = true);
    await guarded(context, () => AppScope.read(context).init(name));
    if (mounted) setState(() => _busy = false);
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    Widget point(IconData icon, String title, String body) => Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, color: DokiColors.lavender),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: text.titleMedium),
                Text(body, style: text.bodyMedium?.copyWith(color: DokiColors.muted)),
              ],
            ),
          ),
        ],
      ),
    );
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 480),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Center(child: GameBadge('Doki', size: 72)),
                  const SizedBox(height: 24),
                  Text('Keep your progress, on your terms', style: text.headlineSmall),
                  const SizedBox(height: 16),
                  point(
                    Icons.bookmark_added_outlined,
                    'Keep checkpoints',
                    'Doki keeps a verified copy each time you save a checkpoint.',
                  ),
                  point(
                    Icons.move_to_inbox_outlined,
                    'Review incoming saves',
                    'A save from another device never replaces yours until you choose it.',
                  ),
                  point(
                    Icons.history,
                    'Restore earlier progress',
                    'Every version stays in your history, including the one you set aside.',
                  ),
                  const SizedBox(height: 24),
                  TextField(
                    controller: _name,
                    decoration: const InputDecoration(
                      labelText: 'Name for this device',
                      helperText: 'Shown on your other devices. You can use any name.',
                    ),
                    onSubmitted: (_) => _start(),
                  ),
                  const SizedBox(height: 24),
                  SizedBox(
                    width: double.infinity,
                    child: FilledButton(
                      onPressed: _busy ? null : _start,
                      child: const Text('Go to my library'),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
