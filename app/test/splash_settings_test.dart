import 'package:doki/main.dart';
import 'package:doki/screens/help.dart';
import 'package:doki/state.dart';
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

void main() {
  double taglineOpacity(WidgetTester tester) => tester
      .widget<FadeTransition>(
        find
            .ancestor(
              of: find.text('Your saves, on your terms'),
              matching: find.byType(FadeTransition),
            )
            .first,
      )
      .opacity
      .value;

  testWidgets('the splash plays once loaded, holds, then fades into the library', (tester) async {
    final state = AppState(readyBackend());
    await tester.pumpWidget(DokiApp(state: state..refresh()));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1400));
    expect(taglineOpacity(tester), greaterThan(0.9));
    expect(
      find.text('Emerald'),
      findsNothing,
      reason: 'the intro plays even when loading is quick',
    );

    await tester.pump(const Duration(milliseconds: 500));
    expect(find.text('Emerald'), findsNothing, reason: 'the wordmark holds before fading');

    await tester.pump(const Duration(milliseconds: 200));
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text('Emerald'), findsOneWidget, reason: 'mid-fade, the library is arriving');
    await tester.pumpAndSettle();
    expect(find.text('Your saves, on your terms'), findsNothing);
  });

  testWidgets('the intro does not start until the vault has loaded', (tester) async {
    final state = AppState(readyBackend());
    await tester.pumpWidget(DokiApp(state: state));
    await tester.pump(const Duration(seconds: 3));
    expect(taglineOpacity(tester), 0, reason: 'still loading, so nothing has played yet');
    await state.refresh();
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1400));
    expect(taglineOpacity(tester), greaterThan(0.9), reason: 'the intro plays after loading');
    expect(find.text('Emerald'), findsNothing);
    await tester.pumpAndSettle();
    expect(find.text('Emerald'), findsOneWidget);
  });

  testWidgets('Help is in Settings and its topics open in place', (tester) async {
    final state = AppState(readyBackend());
    await state.refresh();
    await tester.pumpWidget(DokiApp(state: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    expect(find.text('Help and how-tos'), findsOneWidget);

    expect(find.textContaining('Show a code on this device'), findsNothing);
    await tester.scrollUntilVisible(find.text('Pair your devices'), 200);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Pair your devices'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Show a code on this device'), findsOneWidget);
  });

  test('every help topic has steps', () {
    for (final t in helpTopics) {
      expect(t.points, isNotEmpty, reason: t.title);
    }
  });
}
