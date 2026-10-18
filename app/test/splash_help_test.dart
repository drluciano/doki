import 'package:doki/main.dart';
import 'package:doki/screens/help.dart';
import 'package:doki/state.dart';
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
  testWidgets('the splash shows doki, then fades into the library', (tester) async {
    final state = AppState(readyBackend());
    await tester.pumpWidget(DokiApp(state: state..refresh()));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1000));
    expect(find.text('Your saves, on your terms'), findsOneWidget);
    expect(
      find.text('Emerald'),
      findsNothing,
      reason: 'the intro plays even when loading is quick',
    );

    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump(const Duration(milliseconds: 200));
    expect(find.text('Emerald'), findsOneWidget, reason: 'mid-fade, the library is arriving');
    await tester.pumpAndSettle();
    expect(find.text('Your saves, on your terms'), findsNothing);
  });

  testWidgets('the splash waits for the vault to load', (tester) async {
    final state = AppState(readyBackend());
    await tester.pumpWidget(DokiApp(state: state));
    await tester.pump(const Duration(seconds: 3));
    expect(find.text('Your saves, on your terms'), findsOneWidget, reason: 'still loading');
    await state.refresh();
    await tester.pumpAndSettle();
    expect(find.text('Emerald'), findsOneWidget);
  });

  testWidgets('Help is in the navigation and its topics open in place', (tester) async {
    final state = AppState(readyBackend());
    await state.refresh();
    await tester.pumpWidget(DokiApp(state: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Help'));
    await tester.pumpAndSettle();
    expect(find.text('How Doki keeps your saves safe'), findsOneWidget);
    expect(
      find.textContaining('Receiving it never changes your save'),
      findsOneWidget,
      reason: 'the first topic starts open',
    );

    expect(find.textContaining('Show a code on this device'), findsNothing);
    await tester.ensureVisible(find.text('Pair your devices'));
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
