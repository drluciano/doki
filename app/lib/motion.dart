import 'package:flutter/material.dart';

/// Motion tokens from docs/design.md ("Interaction and motion"): 120–180 ms
/// for control feedback, 180–260 ms for panels and pages. Every animation in
/// the app reads [DokiMotion.reduced] and drops translation, scale and bounce
/// when the OS asks for reduced motion; fades stay so state changes remain
/// visible.
class DokiMotion {
  static const quick = Duration(milliseconds: 150);
  static const panel = Duration(milliseconds: 240);
  static const page = Duration(milliseconds: 260);
  static const enter = Curves.easeOutCubic;
  static const exit = Curves.easeInCubic;

  /// True when the platform's reduce-motion / remove-animations setting is on.
  static bool reduced(BuildContext context) =>
      MediaQuery.maybeDisableAnimationsOf(context) ?? false;
}

/// Page transitions for every pushed screen: the new screen fades in while
/// rising a few pixels, and the screen underneath settles back slightly. The
/// old screen stays opaque so nothing behind the app shows through.
class DokiPageTransitionsBuilder extends PageTransitionsBuilder {
  const DokiPageTransitionsBuilder();

  @override
  Duration get transitionDuration => DokiMotion.page;

  @override
  Duration get reverseTransitionDuration => const Duration(milliseconds: 220);

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) => _DokiPageTransition(animation: animation, secondary: secondaryAnimation, child: child);
}

class _DokiPageTransition extends StatelessWidget {
  const _DokiPageTransition({
    required this.animation,
    required this.secondary,
    required this.child,
  });
  final Animation<double> animation;
  final Animation<double> secondary;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final fade = CurvedAnimation(
      parent: animation,
      curve: const Interval(0, 0.75, curve: Curves.easeOut),
      reverseCurve: const Interval(0.25, 1, curve: Curves.easeIn),
    );
    if (DokiMotion.reduced(context)) return FadeTransition(opacity: fade, child: child);
    final rise = Tween(begin: const Offset(0, 0.035), end: Offset.zero).animate(
      CurvedAnimation(parent: animation, curve: DokiMotion.enter, reverseCurve: DokiMotion.exit),
    );
    final settle = Tween(begin: 1.0, end: 0.97).animate(
      CurvedAnimation(parent: secondary, curve: DokiMotion.enter, reverseCurve: DokiMotion.exit),
    );
    return ScaleTransition(
      scale: settle,
      child: SlideTransition(
        position: rise,
        child: FadeTransition(opacity: fade, child: child),
      ),
    );
  }
}

/// An [IndexedStack] that fades through when [index] changes: the old page
/// fades out, then the new one fades in (and grows from 98% unless motion is
/// reduced). Every page keeps its state. Pages that are not shown cannot be
/// focused, tapped or hovered, and their animations are paused.
class FadeIndexedStack extends StatefulWidget {
  const FadeIndexedStack({super.key, required this.index, required this.children});
  final int index;
  final List<Widget> children;

  @override
  State<FadeIndexedStack> createState() => _FadeIndexedStackState();
}

class _FadeIndexedStackState extends State<FadeIndexedStack> with SingleTickerProviderStateMixin {
  late final _controller = AnimationController(vsync: this, duration: DokiMotion.page, value: 1);
  int? _previous;

  @override
  void didUpdateWidget(FadeIndexedStack old) {
    super.didUpdateWidget(old);
    if (old.index != widget.index) {
      _previous = old.index;
      _controller.forward(from: 0).whenCompleteOrCancel(() {
        if (mounted) setState(() => _previous = null);
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
    final reduced = DokiMotion.reduced(context);
    final fadeOut = ReverseAnimation(
      CurvedAnimation(
        parent: _controller,
        curve: const Interval(0, 0.35, curve: Curves.easeIn),
      ),
    );
    final fadeIn = CurvedAnimation(
      parent: _controller,
      curve: const Interval(0.35, 1, curve: Curves.easeOut),
    );
    return Stack(
      fit: StackFit.expand,
      children: [
        for (var i = 0; i < widget.children.length; i++)
          KeyedSubtree(
            key: ValueKey(i),
            child: _page(i, fadeIn: fadeIn, fadeOut: fadeOut, reduced: reduced),
          ),
      ],
    );
  }

  Widget _page(
    int i, {
    required Animation<double> fadeIn,
    required Animation<double> fadeOut,
    required bool reduced,
  }) {
    final active = i == widget.index;
    final leaving = i == _previous;
    // The wrapper structure never changes between states, so pages keep
    // their state when they are shown, hidden or animated.
    final Animation<double> opacity = active
        ? fadeIn
        : leaving
        ? fadeOut
        : kAlwaysDismissedAnimation;
    final Animation<double> scale = active && !reduced
        ? Tween(begin: 0.98, end: 1.0).animate(fadeIn)
        : kAlwaysCompleteAnimation;
    return Offstage(
      offstage: !active && !leaving,
      child: TickerMode(
        enabled: active || leaving,
        child: ExcludeFocus(
          excluding: !active,
          child: IgnorePointer(
            ignoring: !active,
            child: FadeTransition(
              opacity: opacity,
              child: ScaleTransition(scale: scale, child: widget.children[i]),
            ),
          ),
        ),
      ),
    );
  }
}

/// Fades its child in once when it first appears, rising [rise] pixels unless
/// motion is reduced. [delay] lets a group of items arrive one after another.
class Entrance extends StatefulWidget {
  const Entrance({super.key, required this.child, this.delay = Duration.zero, this.rise = 8});
  final Widget child;
  final Duration delay;
  final double rise;

  @override
  State<Entrance> createState() => _EntranceState();
}

class _EntranceState extends State<Entrance> with SingleTickerProviderStateMixin {
  // The delay is part of the animation rather than a timer, so nothing is
  // left pending when the widget goes away.
  late final _controller = AnimationController(
    vsync: this,
    duration: widget.delay + DokiMotion.panel,
  )..forward();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final total = _controller.duration!.inMicroseconds;
    final start = widget.delay.inMicroseconds / total;
    final curve = CurvedAnimation(
      parent: _controller,
      curve: Interval(start, 1, curve: DokiMotion.enter),
    );
    final reduced = DokiMotion.reduced(context);
    return FadeTransition(
      opacity: curve,
      child: reduced
          ? widget.child
          : AnimatedBuilder(
              animation: curve,
              builder: (context, child) => Transform.translate(
                offset: Offset(0, widget.rise * (1 - curve.value)),
                child: child,
              ),
              child: widget.child,
            ),
    );
  }
}
