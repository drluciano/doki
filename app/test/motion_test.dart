import 'package:doki/motion.dart';
import 'package:doki/theme.dart';
import 'package:doki/widgets/focus_halo.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

Widget harness({bool reduceMotion = false, Widget? home}) => MediaQuery(
  data: MediaQueryData(disableAnimations: reduceMotion),
  child: MaterialApp(
    theme: dokiTheme(),
    navigatorObservers: [FocusHalo.observer],
    builder: (context, child) => FocusHalo(child: child!),
    home: home ?? const _Cards(),
  ),
);

class _Cards extends StatelessWidget {
  const _Cards();

  @override
  Widget build(BuildContext context) => Scaffold(
    body: ListView(
      padding: const EdgeInsets.all(16),
      children: [
        for (final name in ['Emerald', 'Ruby'])
          Card(
            child: InkWell(
              borderRadius: BorderRadius.circular(20),
              onTap: () => Navigator.push(
                context,
                MaterialPageRoute<void>(
                  builder: (_) => Scaffold(appBar: AppBar(title: Text('$name page'))),
                ),
              ),
              child: Padding(padding: const EdgeInsets.all(24), child: Text(name)),
            ),
          ),
        FilledButton(onPressed: () {}, child: const Text('Create checkpoint')),
        const FilledButton(onPressed: null, child: Text('Disabled')),
        const TextField(
          decoration: InputDecoration(labelText: 'Device name', helperText: 'Any name'),
        ),
      ],
    ),
  );
}

FocusHaloState halo(WidgetTester tester) => tester.state(find.byType(FocusHalo));

Rect control(WidgetTester tester, String text) =>
    tester.getRect(find.ancestor(of: find.text(text), matching: find.byType(InkWell)).first);

Future<TestGesture> mouseAt(WidgetTester tester, Offset at) async {
  final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
  await mouse.addPointer(location: Offset.zero);
  addTearDown(mouse.removePointer);
  await mouse.moveTo(at);
  return mouse;
}

void main() {
  testWidgets('hovering a card draws the border around it, then it glides to the next', (
    tester,
  ) async {
    await tester.pumpWidget(harness());
    expect(halo(tester).haloRect, isNull);

    final mouse = await mouseAt(tester, tester.getCenter(find.text('Emerald')));
    await tester.pumpAndSettle();
    final emerald = control(tester, 'Emerald').inflate(4);
    expect(halo(tester).haloRect, rectMoreOrLessEquals(emerald, epsilon: 0.5));

    await mouse.moveTo(tester.getCenter(find.text('Ruby')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 40));
    final ruby = control(tester, 'Ruby').inflate(4);
    final moving = halo(tester).haloRect!;
    expect(moving.top, greaterThan(emerald.top), reason: 'on its way');
    expect(moving.top, lessThan(ruby.top), reason: 'not there yet: it glides');

    await tester.pumpAndSettle();
    expect(halo(tester).haloRect, rectMoreOrLessEquals(ruby, epsilon: 0.5));
  });

  testWidgets('disabled controls and empty space do not get the border', (tester) async {
    await tester.pumpWidget(harness());
    final mouse = await mouseAt(tester, tester.getCenter(find.text('Disabled')));
    await tester.pumpAndSettle();
    expect(halo(tester).haloRect, isNull);
    await mouse.moveTo(const Offset(790, 590));
    await tester.pumpAndSettle();
    expect(halo(tester).haloRect, isNull);
  });

  testWidgets('keyboard and controller focus moves the border', (tester) async {
    await tester.pumpWidget(harness());
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();
    expect(
      halo(tester).haloRect,
      rectMoreOrLessEquals(control(tester, 'Emerald').inflate(4), epsilon: 0.5),
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pumpAndSettle();
    expect(
      halo(tester).haloRect,
      rectMoreOrLessEquals(control(tester, 'Ruby').inflate(4), epsilon: 0.5),
    );
  });

  testWidgets('with reduced motion the border jumps instead of gliding', (tester) async {
    await tester.pumpWidget(harness(reduceMotion: true));
    final mouse = await mouseAt(tester, tester.getCenter(find.text('Emerald')));
    await tester.pumpAndSettle();
    await mouse.moveTo(tester.getCenter(find.text('Ruby')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));
    expect(
      halo(tester).haloRect,
      rectMoreOrLessEquals(control(tester, 'Ruby').inflate(4), epsilon: 0.5),
    );
  });

  testWidgets('a finger press shows the border and releasing fades it', (tester) async {
    await tester.pumpWidget(harness());
    final finger = await tester.startGesture(tester.getCenter(find.text('Create checkpoint')));
    await tester.pumpAndSettle();
    expect(halo(tester).targetRect, isNotNull);
    await finger.up();
    await tester.pumpAndSettle();
    expect(halo(tester).haloRect, isNull);
  });

  testWidgets('opening a screen lets go of the card that opened it', (tester) async {
    await tester.pumpWidget(harness());
    final mouse = await mouseAt(tester, tester.getCenter(find.text('Emerald')));
    await tester.pumpAndSettle();
    expect(halo(tester).targetRect, isNotNull);
    await mouse.down(tester.getCenter(find.text('Emerald')));
    await mouse.up();
    await tester.pumpAndSettle();
    expect(find.text('Emerald page'), findsOneWidget);
    expect(halo(tester).targetRect, isNull, reason: 'nothing on the new screen is under the mouse');
    expect(halo(tester).haloRect, isNull);
  });

  testWidgets('the border follows each control\'s own corners', (tester) async {
    await tester.pumpWidget(harness());
    final mouse = await mouseAt(tester, tester.getCenter(find.text('Emerald')));
    await tester.pumpAndSettle();
    expect(halo(tester).targetRadius, moreOrLessEquals(20 + 4), reason: 'card corner');

    await mouse.moveTo(tester.getCenter(find.text('Create checkpoint')));
    await tester.pumpAndSettle();
    final button = tester.getRect(find.byType(FilledButton).first);
    expect(halo(tester).targetRadius, moreOrLessEquals(button.height / 2 + 4), reason: 'pill');

    await mouse.moveTo(tester.getCenter(find.text('Device name')));
    await tester.pumpAndSettle();
    expect(halo(tester).targetRadius, moreOrLessEquals(16 + 4), reason: 'field corner');
    final field = tester.getRect(find.byType(InputDecorator));
    final outline = halo(tester).targetRect!.deflate(4);
    expect(outline.top, moreOrLessEquals(field.top, epsilon: 0.5));
    expect(outline.bottom, lessThan(field.bottom), reason: 'helper text is outside the border');
  });

  testWidgets('a focused text field gets its own corners too', (tester) async {
    await tester.pumpWidget(harness());
    for (var i = 0; i < 4; i++) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    }
    await tester.pumpAndSettle();
    expect(
      tester.widget<EditableText>(find.byType(EditableText)).focusNode.hasPrimaryFocus,
      isTrue,
    );
    expect(halo(tester).targetRadius, moreOrLessEquals(16 + 4));
  });

  testWidgets('switching tabs fades through and keeps each page', (tester) async {
    var index = 0;
    late StateSetter setIndex;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            setIndex = setState;
            return FadeIndexedStack(
              index: index,
              children: const [
                Scaffold(body: TextField(key: Key('kept'))),
                Scaffold(body: Text('Second')),
              ],
            );
          },
        ),
      ),
    );
    await tester.enterText(find.byKey(const Key('kept')), 'typed');
    setIndex(() => index = 1);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 60));
    expect(find.text('Second'), findsOneWidget, reason: 'both pages exist mid-fade');
    await tester.pumpAndSettle();
    expect(find.text('typed'), findsNothing, reason: 'the old page is offstage');
    setIndex(() => index = 0);
    await tester.pumpAndSettle();
    expect(
      tester.widget<EditableText>(find.byType(EditableText)).controller.text,
      'typed',
      reason: 'the first page kept its state',
    );
  });
}
