import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

import '../motion.dart';
import '../theme.dart';

/// One soft, rounded border that marks the control you are about to use and
/// glides from control to control as the mouse hovers, as keyboard or
/// controller focus moves, and as a finger presses on a touch screen.
///
/// It sits once above the whole app (see `DokiApp`), so screens need no
/// changes: any enabled tappable control (buttons, cards, list rows,
/// navigation items, fields) and any focused control is tracked.
///
/// * Hover follows the mouse. Pressing squeezes the border a little.
/// * Keyboard and controller focus is shown only while focus is being driven
///   by keys, matching how Material shows focus highlights.
/// * On touch, the border pops in under the finger and fades on release; it
///   cancels if the finger starts scrolling.
/// * With reduced motion the border jumps straight to its target instead of
///   gliding or popping. High contrast uses the ink color and a thicker line.
///
/// The border never takes input or semantics; screen readers are unaffected.
class FocusHalo extends StatefulWidget {
  const FocusHalo({super.key, required this.child});
  final Widget child;

  /// Lets the border let go of controls on a screen that is being covered or
  /// removed. Add it to the app's `navigatorObservers`.
  static final NavigatorObserver observer = _HaloRouteObserver();

  @override
  State<FocusHalo> createState() => FocusHaloState();
}

class _HaloRouteObserver extends NavigatorObserver {
  final _halos = <FocusHaloState>{};

  void _changed() {
    for (final h in _halos) {
      h._routeChanged();
    }
  }

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) => _changed();
  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) => _changed();
  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) => _changed();
  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) => _changed();
}

/// A slightly under-damped spring, so a move overshoots by a hair and
/// settles: the "swoop".
class _Spring {
  _Spring(this.value) : target = value;
  double value;
  double target;
  double velocity = 0;

  static const _stiffness = 600.0;
  static const _damping = 2 * 0.74 * 24.49; // 2·ζ·√k with ζ = 0.74

  void snap(double v) {
    value = target = v;
    velocity = 0;
  }

  void step(double dt) {
    final force = -_stiffness * (value - target) - _damping * velocity;
    velocity += force * dt;
    value += velocity * dt;
  }

  bool get atRest => (value - target).abs() < 0.25 && velocity.abs() < 2;
}

class _RectSpring {
  _RectSpring() : _parts = List.generate(4, (_) => _Spring(0));
  final List<_Spring> _parts;

  Rect get value =>
      Rect.fromLTRB(_parts[0].value, _parts[1].value, _parts[2].value, _parts[3].value);
  set target(Rect r) {
    _parts[0].target = r.left;
    _parts[1].target = r.top;
    _parts[2].target = r.right;
    _parts[3].target = r.bottom;
  }

  void snap(Rect r) {
    _parts[0].snap(r.left);
    _parts[1].snap(r.top);
    _parts[2].snap(r.right);
    _parts[3].snap(r.bottom);
  }

  void step(double dt) {
    for (final p in _parts) {
      p.step(dt);
    }
  }

  bool get atRest => _parts.every((p) => p.atRest);
}

/// Where the border should be, in the halo's own coordinates.
class _Placement {
  const _Placement(this.rect, this.clip, this.radius);
  final Rect rect;
  final Rect clip;
  final double radius;
}

/// What the painter draws on the current frame.
class _HaloFrame extends ChangeNotifier {
  Rect rect = Rect.zero;
  Rect clip = Rect.zero;
  double radius = 0;
  double opacity = 0;
  void changed() => notifyListeners();
}

class FocusHaloState extends State<FocusHalo>
    with SingleTickerProviderStateMixin, WidgetsBindingObserver {
  static const _gap = 4.0; // how far the border sits outside the control
  static const _popFrom = 10.0; // extra size the border shrinks in from
  static const _squeeze = 2.0; // how far a press pulls the border in

  late final Ticker _ticker = createTicker(_tick);
  final _frame = _HaloFrame();
  final _rect = _RectSpring();
  final _clip = _RectSpring();
  final _radius = _Spring(0);
  Duration _lastTick = Duration.zero;
  int _quietFrames = 0;
  int _holdFrames = 0;

  Offset? _mouse; // last mouse position, while the mouse is in the window
  RenderBox? _hovered;
  bool _mousePressed = false;
  RenderBox? _touched;
  Offset? _touchStart;
  bool _keysLead = false; // focus moved by keys more recently than the mouse
  bool _visible = false;

  /// The rectangle the border is drawn around (including its gap), or null
  /// when it is hidden. Exposed for tests.
  @visibleForTesting
  Rect? get haloRect => _frame.opacity > 0.01 ? _frame.rect : null;

  /// Where the border is heading, or null when it is fading out.
  @visibleForTesting
  Rect? get targetRect => _visible ? _targetRect : null;
  Rect? _targetRect;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    FocusManager.instance.addListener(_focusChanged);
    FocusManager.instance.addHighlightModeListener(_highlightModeChanged);
    (FocusHalo.observer as _HaloRouteObserver)._halos.add(this);
  }

  @override
  void dispose() {
    (FocusHalo.observer as _HaloRouteObserver)._halos.remove(this);
    FocusManager.instance.removeHighlightModeListener(_highlightModeChanged);
    FocusManager.instance.removeListener(_focusChanged);
    WidgetsBinding.instance.removeObserver(this);
    _ticker.dispose();
    _frame.dispose();
    super.dispose();
  }

  @override
  void didChangeMetrics() => _wake();

  void _focusChanged() {
    if (FocusManager.instance.highlightMode == FocusHighlightMode.traditional) _keysLead = true;
    _wake();
  }

  void _highlightModeChanged(FocusHighlightMode _) => _wake();

  void _routeChanged() {
    // The screen under the pointer is changing; follow it through the whole
    // page transition, then look again at what the mouse is over.
    _hovered = null;
    _touched = null;
    _wake(hold: 24);
  }

  void _wake({int hold = 0}) {
    if (!mounted) return;
    _quietFrames = 0;
    _holdFrames = math.max(_holdFrames, hold);
    if (!_ticker.isActive) {
      _lastTick = Duration.zero;
      _ticker.start();
    }
  }

  // ---- Pointer input --------------------------------------------------------

  void _onHover(PointerHoverEvent e) {
    if (e.kind != PointerDeviceKind.mouse) return;
    _mouse = e.position;
    final hit = _controlAt(e.position);
    if (hit != _hovered) {
      _hovered = hit;
      if (hit != null) _keysLead = false;
    }
    _wake();
  }

  void _onDown(PointerDownEvent e) {
    if (e.kind == PointerDeviceKind.mouse) {
      _mouse = e.position;
      _hovered = _controlAt(e.position) ?? _hovered;
      _mousePressed = true;
      _keysLead = false;
    } else {
      _touched = _controlAt(e.position);
      _touchStart = e.position;
    }
    _wake();
  }

  void _onMove(PointerMoveEvent e) {
    if (e.kind == PointerDeviceKind.mouse) {
      _mouse = e.position;
      return;
    }
    // A finger that travels is scrolling, not pressing.
    if (_touched != null && (e.position - _touchStart!).distance > kTouchSlop) {
      _touched = null;
      _wake();
    }
  }

  void _onUp(PointerEvent e) {
    if (e.kind == PointerDeviceKind.mouse) {
      _mousePressed = false;
    } else {
      _touched = null;
    }
    _wake();
  }

  void _onExitWindow(PointerExitEvent e) {
    _mouse = null;
    _hovered = null;
    _mousePressed = false;
    _wake();
  }

  bool _onScroll(Notification n) {
    // Content moved under a still mouse: what it points at may have changed.
    if (_mouse != null) _hovered = _controlAt(_mouse!);
    _wake();
    return false;
  }

  /// The innermost enabled control at [position]: the nearest area that
  /// tells assistive technology it can be tapped. Disabled controls and
  /// scrolling areas do not, so they are skipped.
  RenderBox? _controlAt(Offset position) {
    final result = HitTestResult();
    WidgetsBinding.instance.hitTestInView(result, position, View.of(context).viewId);
    for (final entry in result.path) {
      final target = entry.target;
      if (target is RenderSemanticsAnnotations &&
          (target.properties.onTap != null || target.properties.onLongPress != null) &&
          target.hasSize) {
        return target;
      }
    }
    return null;
  }

  // ---- Choosing and measuring the target ------------------------------------

  /// The box of the control that has keyboard or controller focus, if focus
  /// is being shown at all.
  RenderBox? _focusedBox() {
    if (FocusManager.instance.highlightMode != FocusHighlightMode.traditional) return null;
    final node = FocusManager.instance.primaryFocus;
    final context = node?.context;
    if (node == null || node is FocusScopeNode || context is! Element || !context.mounted) {
      return null;
    }
    // A text field is focused inside its decoration; outline the whole field.
    RenderObject? box;
    var depth = 0;
    context.visitAncestorElements((e) {
      if (e.widget is InputDecorator) {
        box = e.renderObject;
        return false;
      }
      return ++depth < 24;
    });
    box ??= context.findRenderObject();
    return box is RenderBox ? box as RenderBox : null;
  }

  _Placement? _measure(RenderBox? box) {
    final me = context.findRenderObject();
    if (box == null || me is! RenderBox || !box.attached || !box.hasSize || !me.hasSize) {
      return null;
    }
    // Not painted: an inactive tab or a covered screen kept for later.
    for (RenderObject? o = box; o != null; o = o.parent) {
      if (o is RenderOffstage && o.offstage) return null;
      if (o == me) break;
    }
    final rect = MatrixUtils.transformRect(box.getTransformTo(me), Offset.zero & box.size);
    final area = me.size.width * me.size.height;
    // Whole-screen focus (a page or a list) is not a control.
    if (rect.width * rect.height > area * 0.5) return null;
    var clip = Offset.zero & me.size;
    final RenderObject? viewport = RenderAbstractViewport.maybeOf(box);
    if (viewport is RenderBox && viewport.hasSize) {
      clip = MatrixUtils.transformRect(
        viewport.getTransformTo(me),
        Offset.zero & viewport.size,
      ).inflate(_gap + 2);
    }
    if (!clip.overlaps(rect)) return null;
    // Pills stay pills; cards and rows get the card corner.
    final shortest = rect.shortestSide;
    final radius = (shortest <= 56 ? shortest / 2 : 20.0) + _gap;
    return _Placement(rect.inflate(_gap), clip, radius);
  }

  _Placement? _choose() {
    if (_mouse != null && _holdFrames > 0) _hovered = _controlAt(_mouse!);
    if (_touched != null) {
      final p = _measure(_touched);
      return p == null ? null : _Placement(p.rect.deflate(_squeeze), p.clip, p.radius - _squeeze);
    }
    final focus = _measure(_focusedBox());
    final hover = _measure(_hovered);
    if (_keysLead && focus != null) return focus;
    if (hover != null) {
      return _mousePressed
          ? _Placement(hover.rect.deflate(_squeeze), hover.clip, hover.radius - _squeeze)
          : hover;
    }
    return focus;
  }

  // ---- Animation ------------------------------------------------------------

  void _tick(Duration elapsed) {
    final dt = math.min((elapsed - _lastTick).inMicroseconds / 1e6, 0.05);
    _lastTick = elapsed;
    if (_holdFrames > 0) _holdFrames--;

    final reduced = DokiMotion.reduced(context);
    final place = _choose();
    _targetRect = place?.rect;
    if (place != null) {
      if (!_visible && _frame.opacity < 0.05) {
        // Appearing from nothing: start a little larger and shrink in.
        final from = reduced ? place.rect : place.rect.inflate(_popFrom);
        _rect.snap(from);
        _clip.snap(place.clip);
        _radius.snap(place.radius + (reduced ? 0 : _popFrom));
      }
      _visible = true;
      _rect.target = place.rect;
      _clip.target = place.clip;
      _radius.target = place.radius;
    } else {
      _visible = false;
    }

    if (reduced) {
      if (place != null) {
        _rect.snap(place.rect);
        _clip.snap(place.clip);
        _radius.snap(place.radius);
      }
    } else {
      // Small fixed steps keep the spring stable at any frame rate.
      final steps = math.max(1, (dt / (1 / 240)).ceil());
      for (var i = 0; i < steps; i++) {
        _rect.step(dt / steps);
        _clip.step(dt / steps);
        _radius.step(dt / steps);
      }
    }
    final fadeStep = dt / DokiMotion.quick.inMilliseconds * 1000;
    final opacityTarget = _visible ? 1.0 : 0.0;
    final opacity = _frame.opacity < opacityTarget
        ? math.min(opacityTarget, _frame.opacity + fadeStep)
        : math.max(opacityTarget, _frame.opacity - fadeStep);

    _frame
      ..rect = _rect.value
      ..clip = _clip.value
      ..radius = math.max(0, _radius.value)
      ..opacity = opacity
      ..changed();

    final settled =
        (reduced || (_rect.atRest && _clip.atRest && _radius.atRest)) && opacity == opacityTarget;
    _quietFrames = settled ? _quietFrames + 1 : 0;
    if (_quietFrames >= 2 && _holdFrames == 0) _ticker.stop();
  }

  @override
  Widget build(BuildContext context) {
    final highContrast = MediaQuery.maybeHighContrastOf(context) ?? false;
    return MouseRegion(
      opaque: false,
      onExit: _onExitWindow,
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerHover: _onHover,
        onPointerDown: _onDown,
        onPointerMove: _onMove,
        onPointerUp: _onUp,
        onPointerCancel: _onUp,
        child: NotificationListener<ScrollNotification>(
          onNotification: _onScroll,
          child: NotificationListener<ScrollMetricsNotification>(
            onNotification: _onScroll,
            child: Stack(
              fit: StackFit.passthrough,
              children: [
                widget.child,
                Positioned.fill(
                  child: IgnorePointer(
                    child: CustomPaint(
                      painter: _HaloPainter(
                        _frame,
                        color: highContrast ? DokiColors.ink : DokiColors.lavender,
                        width: highContrast ? 3.5 : 2.5,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _HaloPainter extends CustomPainter {
  _HaloPainter(this.frame, {required this.color, required this.width}) : super(repaint: frame);
  final _HaloFrame frame;
  final Color color;
  final double width;

  @override
  void paint(Canvas canvas, Size size) {
    if (frame.opacity <= 0.01 || frame.rect.isEmpty) return;
    final rrect = RRect.fromRectAndRadius(frame.rect, Radius.circular(frame.radius));
    canvas
      ..save()
      ..clipRect(frame.clip);
    canvas.drawRRect(
      rrect,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = width + 5
        ..color = color.withValues(alpha: 0.22 * frame.opacity)
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 5),
    );
    canvas.drawRRect(
      rrect,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = width
        ..color = color.withValues(alpha: frame.opacity),
    );
    canvas.restore();
  }

  @override
  bool shouldRepaint(_HaloPainter old) =>
      old.frame != frame || old.color != color || old.width != width;
}
