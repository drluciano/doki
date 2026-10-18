import 'package:flutter/material.dart';

import '../main.dart';
import '../motion.dart';
import '../theme.dart';

/// Shows the doki wordmark while the app starts, then fades into [child].
///
/// The letters arrive one after another, a short line follows, and the
/// whole screen fades into the app once the vault has loaded and the intro
/// has played (whichever is later). With reduced motion the intro is shorter
/// and nothing moves; it only fades.
class SplashGate extends StatefulWidget {
  const SplashGate({super.key, required this.child});
  final Widget child;

  @override
  State<SplashGate> createState() => _SplashGateState();
}

class _SplashGateState extends State<SplashGate> with SingleTickerProviderStateMixin {
  static const _intro = Duration(milliseconds: 1300);
  static const _introReduced = Duration(milliseconds: 500);

  // An animation rather than a timer, so the intro never leaves a timer
  // behind and tests can step through it.
  late final _controller = AnimationController(vsync: this);
  bool _introDone = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_controller.duration == null) {
      _controller.duration = DokiMotion.reduced(context) ? _introReduced : _intro;
      _controller.forward().whenComplete(() {
        if (mounted) setState(() => _introDone = true);
      });
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final loaded = AppScope.of(context).loaded;
    final showApp = loaded && _introDone;
    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 420),
      switchInCurve: DokiMotion.enter,
      switchOutCurve: Curves.easeIn,
      child: showApp
          ? KeyedSubtree(key: const ValueKey('app'), child: widget.child)
          : _Wordmark(key: const ValueKey('splash'), progress: _controller),
    );
  }
}

class _Wordmark extends StatelessWidget {
  const _Wordmark({super.key, required this.progress});
  final Animation<double> progress;

  @override
  Widget build(BuildContext context) {
    final reduced = DokiMotion.reduced(context);
    const word = 'doki';
    Widget letter(int i) {
      // Each letter takes 35% of the intro, starting 10% after the last.
      final start = 0.05 + i * 0.1;
      final curve = CurvedAnimation(
        parent: progress,
        curve: Interval(start, start + 0.35, curve: reduced ? Curves.easeOut : Curves.easeOutBack),
      );
      return AnimatedBuilder(
        animation: curve,
        builder: (context, child) {
          final t = curve.value;
          return Opacity(
            opacity: t.clamp(0.0, 1.0),
            child: reduced
                ? child
                : Transform.translate(offset: Offset(0, 18 * (1 - t)), child: child),
          );
        },
        child: Text(
          word[i],
          style: const TextStyle(
            fontSize: 64,
            fontWeight: FontWeight.w800,
            color: DokiColors.lavender,
            letterSpacing: 1,
            height: 1,
          ),
        ),
      );
    }

    final tagline = CurvedAnimation(
      parent: progress,
      curve: const Interval(0.5, 0.85, curve: Curves.easeOut),
    );
    return Scaffold(
      body: Center(
        child: Semantics(
          label: 'doki',
          container: true,
          excludeSemantics: true,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [for (var i = 0; i < word.length; i++) letter(i)],
              ),
              const SizedBox(height: 12),
              FadeTransition(
                opacity: tagline,
                child: const Text(
                  'Your saves, on your terms',
                  style: TextStyle(fontSize: 16, color: DokiColors.muted),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
