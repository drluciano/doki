import 'package:flutter/material.dart';

import '../backend.dart';
import '../copy.dart';
import '../motion.dart';
import '../theme.dart';

/// Run an engine action, showing a persistent explanation if it fails.
/// Returns null on failure.
Future<T?> guarded<T>(BuildContext context, Future<T> Function() action) async {
  try {
    return await action();
  } on DokiError catch (e) {
    if (context.mounted) await showErrorSheet(context, e);
  } catch (e) {
    if (context.mounted) await showErrorSheet(context, DokiError('Internal', e.toString()));
  }
  return null;
}

Future<void> showErrorSheet(BuildContext context, DokiError e) {
  final copy = errorCopy(e.code);
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (context) => SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            StatusBanner.error(copy.title),
            const SizedBox(height: 16),
            Text(filesChangedLine(e), style: Theme.of(context).textTheme.bodyLarge),
            const SizedBox(height: 8),
            Text(copy.next, style: Theme.of(context).textTheme.bodyLarge),
            const SizedBox(height: 8),
            ExpansionTile(
              tilePadding: EdgeInsets.zero,
              title: const Text('Details'),
              children: [
                Align(
                  alignment: Alignment.centerLeft,
                  child: SelectableText('${e.code}\n${e.message}'),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerRight,
              child: FilledButton(onPressed: () => Navigator.pop(context), child: const Text('OK')),
            ),
          ],
        ),
      ),
    ),
  );
}

enum BannerKind { success, attention, error, info }

/// A state message that always carries a word and an icon, never color alone.
class StatusBanner extends StatelessWidget {
  const StatusBanner(this.text, {super.key, this.kind = BannerKind.info, this.detail});
  const StatusBanner.success(this.text, {super.key, this.detail}) : kind = BannerKind.success;
  const StatusBanner.attention(this.text, {super.key, this.detail}) : kind = BannerKind.attention;
  const StatusBanner.error(this.text, {super.key, this.detail}) : kind = BannerKind.error;

  final String text;
  final String? detail;
  final BannerKind kind;

  @override
  Widget build(BuildContext context) {
    final (bg, fg, icon) = switch (kind) {
      BannerKind.success => (DokiColors.mint, DokiColors.mintInk, Icons.verified_user_outlined),
      BannerKind.attention => (DokiColors.attention, DokiColors.attentionInk, Icons.info_outline),
      BannerKind.error => (DokiColors.errorSoft, DokiColors.error, Icons.error_outline),
      BannerKind.info => (DokiColors.lavenderSoft, DokiColors.lavender, Icons.auto_awesome),
    };
    return Semantics(
      container: true,
      liveRegion: kind == BannerKind.error,
      child: AnimatedContainer(
        duration: DokiMotion.panel,
        curve: DokiMotion.enter,
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(16)),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: fg),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    text,
                    style: TextStyle(color: fg, fontWeight: FontWeight.w600, fontSize: 16),
                  ),
                  if (detail != null) ...[
                    const SizedBox(height: 4),
                    Text(detail!, style: const TextStyle(color: DokiColors.ink, fontSize: 15)),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Rounded initials badge used instead of game art.
class GameBadge extends StatelessWidget {
  const GameBadge(this.name, {super.key, this.size = 48});
  final String name;
  final double size;

  @override
  Widget build(BuildContext context) {
    final words = name.trim().split(RegExp(r'\s+')).where((w) => w.isNotEmpty).toList();
    final initials = words.isEmpty
        ? '?'
        : words.take(2).map((w) => w.characters.first.toUpperCase()).join();
    const palette = [DokiColors.lavenderSoft, DokiColors.mint, DokiColors.attention];
    final bg = palette[name.hashCode.abs() % palette.length];
    return ExcludeSemantics(
      child: Container(
        width: size,
        height: size,
        alignment: Alignment.center,
        decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(size / 3)),
        child: Text(
          initials,
          style: TextStyle(
            fontWeight: FontWeight.w700,
            fontSize: size / 2.6,
            color: DokiColors.ink,
          ),
        ),
      ),
    );
  }
}

class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    required this.body,
    this.action,
  });
  final IconData icon;
  final String title;
  final String body;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(32),
        child: Entrance(
          rise: 16,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: 56, color: DokiColors.lavender),
                const SizedBox(height: 16),
                Text(
                  title,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text(
                  body,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyLarge?.copyWith(color: DokiColors.muted),
                ),
                if (action != null) ...[const SizedBox(height: 24), action!],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Caps content width on wide windows.
class ContentWidth extends StatelessWidget {
  const ContentWidth({super.key, required this.child, this.maxWidth = 900});
  final Widget child;
  final double maxWidth;

  @override
  Widget build(BuildContext context) => Align(
    alignment: Alignment.topCenter,
    child: ConstrainedBox(
      constraints: BoxConstraints(maxWidth: maxWidth),
      child: child,
    ),
  );
}

/// Shows a labeled, indeterminate progress dialog while [action] runs.
Future<T?> withProgress<T>(BuildContext context, String label, Future<T> Function() action) async {
  showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (_) => PopScope(
      canPop: false,
      child: AlertDialog(
        content: Row(
          children: [
            const CircularProgressIndicator(),
            const SizedBox(width: 20),
            Expanded(child: Text(label)),
          ],
        ),
      ),
    ),
  );
  return guarded(context, () async {
    try {
      return await action();
    } finally {
      if (context.mounted) Navigator.of(context, rootNavigator: true).pop();
    }
  });
}
