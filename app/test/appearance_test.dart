import 'dart:io';
import 'dart:ui';

import 'package:doki/appearance.dart';
import 'package:doki/main.dart';
import 'package:doki/state.dart';
import 'package:doki/theme.dart';
import 'package:doki/widgets/focus_halo.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'fake_backend.dart';

FakeBackend readyBackend() => FakeBackend({
  'device': (_) => {'device_id': 'd1', 'device_name': 'PC'},
  'games': (_) => [game('protected')],
  'incoming': (_) => [],
  'unmapped': (_) => [],
  'relay_code': (_) => null,
  'peers': (_) => [],
});

Future<Appearance> openApp(WidgetTester tester, {Appearance? appearance}) async {
  final state = AppState(readyBackend());
  await state.refresh();
  final a = appearance ?? Appearance();
  await tester.pumpWidget(DokiApp(state: state, appearance: a));
  await tester.pumpAndSettle();
  return a;
}

Color pageColor(WidgetTester tester) =>
    Theme.of(tester.element(find.text('Emerald'))).scaffoldBackgroundColor;

Future<void> openSettings(WidgetTester tester) async {
  await tester.tap(find.text('Settings'));
  await tester.pumpAndSettle();
}

void main() {
  tearDown(() => DokiColors.palette = DokiPalette.light);

  testWidgets('choosing a theme in Settings recolors the app', (tester) async {
    final appearance = await openApp(tester);
    expect(pageColor(tester), DokiPalette.light.cream);

    await openSettings(tester);
    await tester.tap(find.bySemanticsLabel('Dark theme'));
    await tester.pumpAndSettle();
    expect(appearance.theme, ThemeChoice.dark);
    expect(DokiColors.palette, same(DokiPalette.dark));
    expect(
      Theme.of(tester.element(find.text('Theme'))).scaffoldBackgroundColor,
      DokiPalette.dark.cream,
    );

    await tester.tap(find.bySemanticsLabel('OLED theme'));
    await tester.pumpAndSettle();
    expect(DokiColors.palette.cream, const Color(0xFF000000));

    await tester.tap(find.bySemanticsLabel('Light theme'));
    await tester.pumpAndSettle();
    expect(DokiColors.palette, same(DokiPalette.light));
  });

  testWidgets('Follow system uses the device setting', (tester) async {
    tester.platformDispatcher.platformBrightnessTestValue = Brightness.dark;
    addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
    await openApp(tester);
    expect(pageColor(tester), DokiPalette.dark.cream);

    tester.platformDispatcher.platformBrightnessTestValue = Brightness.light;
    await tester.pumpAndSettle();
    expect(pageColor(tester), DokiPalette.light.cream);
  });

  testWidgets('the border color is the one chosen, shaded for the theme', (tester) async {
    final mint = HaloAccent.byId('mint');
    await openApp(tester, appearance: Appearance(accent: mint));
    FocusHalo halo() => tester.widget(find.byType(FocusHalo));
    expect(halo().color, mint.light);

    await openSettings(tester);
    await tester.tap(find.bySemanticsLabel('Rose border'));
    await tester.pumpAndSettle();
    expect(halo().color, HaloAccent.byId('rose').light);

    await tester.tap(find.bySemanticsLabel('Dark theme'));
    await tester.pumpAndSettle();
    expect(halo().color, HaloAccent.byId('rose').dark);
  });

  test('choices are remembered between starts', () async {
    final dir = await Directory.systemTemp.createTemp('doki-appearance');
    addTearDown(() => dir.delete(recursive: true));
    final file = File('${dir.path}/appearance.json');

    final first = await Appearance.load(file);
    expect(first.theme, ThemeChoice.system, reason: 'no file yet means the defaults');
    first
      ..theme = ThemeChoice.oled
      ..accent = HaloAccent.byId('gold');
    await first.saved;

    final second = await Appearance.load(file);
    expect(second.theme, ThemeChoice.oled);
    expect(second.accent.id, 'gold');

    await file.writeAsString('not json');
    expect((await Appearance.load(file)).theme, ThemeChoice.system);
  });

  testWidgets('the border stays whole on the navigation rail at the window edge', (tester) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await openApp(tester);

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: const Offset(600, 400));
    addTearDown(mouse.removePointer);
    await mouse.moveTo(tester.getCenter(find.text('Incoming').first));
    await tester.pumpAndSettle();

    final FocusHaloState state = tester.state(find.byType(FocusHalo));
    final rect = state.haloRect!;
    expect(rect.left, greaterThanOrEqualTo(2), reason: 'not cut off by the window edge');
    expect(rect.right, lessThanOrEqualTo(1200));
    expect(state.targetRadius, lessThanOrEqualTo(rect.shortestSide / 2 + 0.01));
  });
}
