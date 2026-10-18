import 'package:flutter/material.dart';

import '../main.dart';
import '../motion.dart';
import '../theme.dart';

/// Shows the doki wordmark while the app starts, then fades into [child].
///
/// The intro waits until the vault has loaded and the splash has been on
/// screen for a frame, so it is never spent while the app is still busy
/// starting. Until then the screen is the plain page color, matching the
/// system launch screen. Then the letters arrive one after another, a short
/// line follows, the wordmark holds for a moment, and the screen fades into
/// the app. With reduced motion nothing moves; the wordmark fades in, holds
/// and fades out.
class SplashGate extends StatefulWidget {
  const SplashGate({super.key, required this.child});
  final Widget child;

  /// How long the intro plays once started, including the hold at the end.
  static const intro = Duration(milliseconds: 2000);
  static const introReduced = Duration(milliseconds: 900);

  @override
  State<SplashGate> createState() => _SplashGateState();
}

class _SplashGateState extends State<SplashGate> with SingleTickerProviderStateMixin {
  // An animation rather than a timer, so the intro never leaves a timer
  // behind and tests can step through it.
  late final _controller = AnimationController(vsync: this);
  bool _starting = false;
  bool _introDone = false;

  void _startWhenLoaded(bool loaded) {
    if (!loaded || _starting) return;
    _starting = true;
    _controller.duration = DokiMotion.reduced(context) ? SplashGate.introReduced : SplashGate.intro;
    // Start after this frame is drawn: loading can keep the first frames
    // from reaching the screen, and an intro that runs then is never seen.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _controller.forward().whenComplete(() {
        if (mounted) setState(() => _introDone = true);
      });
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final loaded = AppScope.of(context).loaded;
    _startWhenLoaded(loaded);
    final showApp = loaded && _introDone;
    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 480),
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
      // Each letter takes 26% of the intro, starting 9% after the last; with
      // reduced motion they all fade in together.
      final start = reduced ? 0.0 : 0.04 + i * 0.09;
      final curve = CurvedAnimation(
        parent: progress,
        curve: Interval(
          start,
          start + (reduced ? 0.35 : 0.26),
          curve: reduced ? Curves.easeOut : Curves.easeOutBack,
        ),
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
          style: TextStyle(
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
      // The last stretch of the intro is a hold with everything shown.
      curve: Interval(reduced ? 0.15 : 0.42, reduced ? 0.5 : 0.68, curve: Curves.easeOut),
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
                child: Text(
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
