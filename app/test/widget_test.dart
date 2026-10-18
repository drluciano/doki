import 'package:doki/backend.dart';
import 'package:doki/main.dart';
import 'package:doki/screens/review.dart';
import 'package:doki/state.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'fake_backend.dart';

Future<AppState> pump(WidgetTester tester, FakeBackend backend, {Widget? home}) async {
  final state = AppState(backend);
  await state.refresh();
  await tester.pumpWidget(
    home == null
        ? DokiApp(state: state)
        : AppScope(
            state: state,
            child: MaterialApp(home: home),
          ),
  );
  await tester.pumpAndSettle();
  return state;
}

Map<String, dynamic Function(Map<String, dynamic>)> ready(List<Map<String, dynamic>> games) => {
  'device': (_) => {'device_id': 'd1', 'device_name': 'PC'},
  'games': (_) => games,
  'incoming': (_) => [],
  'unmapped': (_) => [],
  'relay_code': (_) => null,
};

void main() {
  testWidgets('first run asks for a device name and opens the library', (tester) async {
    var initialized = false;
    final backend = FakeBackend({
      'device': (_) => initialized
          ? {'device_id': 'd1', 'device_name': 'Desk'}
          : throw DokiError('NotFound', 'no vault'),
      'init': (args) {
        expect(args['name'], 'Desk');
        initialized = true;
        return {'device_id': 'd1', 'device_name': 'Desk'};
      },
      'games': (_) => [],
      'incoming': (_) => [],
      'unmapped': (_) => [],
      'relay_code': (_) => null,
    });
    await pump(tester, backend);
    expect(find.text('Keep your progress, on your terms'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'Desk');
    await tester.ensureVisible(find.text('Go to my library'));
    await tester.tap(find.text('Go to my library'));
    await tester.pumpAndSettle();
    expect(find.text('No games yet'), findsOneWidget);
  });

  testWidgets('library shows status in words and incoming count', (tester) async {
    final backend = FakeBackend(ready([game('uncaptured_changes', incoming: 1)]));
    await pump(tester, backend);
    expect(find.text('Emerald'), findsOneWidget);
    expect(find.text('New progress not in a checkpoint yet'), findsOneWidget);
    expect(find.text('1 incoming'), findsOneWidget);
  });

  testWidgets('a game whose folder access was revoked says how to fix it', (tester) async {
    final backend = FakeBackend(ready([game('unavailable:PermissionLost')]));
    await pump(tester, backend);
    expect(find.text('Folder access needs updating'), findsOneWidget);
  });

  testWidgets('divergent review explains both saves are kept, then installs', (tester) async {
    final review = {
      'bundle_id': 'b1',
      'incoming': 'c3',
      'incoming_checkpoint': checkpoint(device: 'Odin', parents: ['c1']),
      'baseline_head': 'c2',
      'baseline_files': 'sha256:aa',
      'relation': {
        'kind': 'divergent',
        'common': ['c1'],
      },
      'captured_local_changes': null,
      'proposal': 'b1.c3',
    };
    Map<String, dynamic>? used;
    final handlers = ready([game('protected', incoming: 1, head: 'c2')])
      ..['review'] = ((_) => review)
      ..['history'] = ((_) => [
        {
          'id': 'c2',
          'checkpoint': checkpoint(device: 'PC', parents: ['c1']),
          'is_head': true,
          'is_tip': true,
          'set_aside': false,
          'pinned': false,
        },
      ])
      ..['use'] = ((args) {
        used = args;
        return {'installed': 'c3', 'head': 'c4', 'backup': 'c2', 'set_aside': 'c2'};
      });
    final backend = FakeBackend(handlers);
    await pump(
      tester,
      backend,
      home: const ReviewScreen(gameId: 'b1', checkpointId: 'c3', fromDevice: 'Odin'),
    );
    expect(
      find.text(
        'You played on both devices. Choose which save to continue. Both versions will stay in your history.',
      ),
      findsOneWidget,
    );
    expect(find.text('Your current save'), findsOneWidget);
    expect(find.text('Incoming progress from Odin'), findsOneWidget);
    expect(backend.calls, isNot(contains('use')), reason: 'opening a review never installs');

    await tester.ensureVisible(find.text('Back up and use this save'));
    await tester.tap(find.text('Back up and use this save'));
    await tester.pumpAndSettle();
    expect(used!['review'], review, reason: 'the review goes back to the engine unchanged');
    expect(find.text('Now using this save'), findsOneWidget);
    expect(find.textContaining('Your previous save is checkpoint c2'), findsOneWidget);
  });

  testWidgets('a failed install explains whether the save changed', (tester) async {
    final handlers = ready([game('protected', incoming: 1, head: 'c2')])
      ..['review'] = ((_) => {
        'bundle_id': 'b1',
        'incoming': 'c3',
        'incoming_checkpoint': checkpoint(parents: ['c2']),
        'baseline_head': 'c2',
        'baseline_files': 'sha256:aa',
        'relation': {'kind': 'ahead'},
        'captured_local_changes': null,
        'proposal': 'b1.c3',
      })
      ..['history'] = ((_) => [])
      ..['use'] = ((_) => throw DokiError('InstallInterrupted', 'disk', saveMayBeMixed: true));
    await pump(
      tester,
      FakeBackend(handlers),
      home: const ReviewScreen(gameId: 'b1', checkpointId: 'c3'),
    );
    await tester.ensureVisible(find.text('Back up and use this save'));
    await tester.tap(find.text('Back up and use this save'));
    await tester.pumpAndSettle();
    expect(find.text('Finish or undo the last change'), findsOneWidget);
    expect(find.textContaining('part old, part new'), findsOneWidget);
  });
}
