import 'package:doki/backend.dart';
import 'package:doki/main.dart';
import 'package:doki/screens/devices.dart';
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
  'peer_list': (_) => [],
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
      'peer_list': (_) => [],
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

  group('devices', () {
    setUp(() {
      final view = TestWidgetsFlutterBinding.instance.platformDispatcher.implicitView!;
      view.physicalSize = const Size(800, 2000);
      view.devicePixelRatio = 1;
    });
    tearDown(() => TestWidgetsFlutterBinding.instance.platformDispatcher.implicitView!.reset());

    Map<String, dynamic> session(String name, {int sent = 0, int received = 0}) => {
      'peer': 'id-$name',
      'peer_name': name,
      'sent': sent,
      'received': received == 0
          ? null
          : {
              'from_device': name,
              'checkpoints_added': received,
              'new_proposals': [
                for (var i = 0; i < received; i++) ['b1', 'c$i'],
              ],
              'unmapped': [],
            },
    };

    testWidgets('pairing with a short code adds the device and says nothing changed', (
      tester,
    ) async {
      var peers = <Map<String, dynamic>>[];
      final handlers = ready([])
        ..['peer_list'] = ((_) => peers)
        ..['peer_join'] = ((args) {
          expect(args['code'], 'ABCD-2345');
          peers = [
            {'id': 'id-odin', 'name': 'Odin', 'introduced_by': null, 'last_sync': null},
          ];
          return {'name': 'Odin'};
        });
      await pump(tester, FakeBackend(handlers), home: const DevicesScreen());
      expect(find.textContaining('No other devices yet'), findsOneWidget);

      await tester.enterText(find.widgetWithText(TextField, 'Code'), 'ABCD-2345');
      await tester.ensureVisible(find.text('Pair'));
      await tester.tap(find.text('Pair'));
      await tester.pumpAndSettle();
      expect(find.text('Paired with Odin'), findsOneWidget);
      expect(find.textContaining('Nothing on this device was changed'), findsOneWidget);
      expect(find.text('Odin'), findsOneWidget);
      expect(find.text('Not synced yet'), findsOneWidget);
    });

    testWidgets('pairing failures are explained in pairing terms', (tester) async {
      final handlers = ready([])
        ..['peer_join'] = ((_) => throw DokiError(
          'SourceUnavailable',
          'no device showing a code was found on this network; use the long code instead',
        ));
      await pump(tester, FakeBackend(handlers), home: const DevicesScreen());
      await tester.enterText(find.widgetWithText(TextField, 'Code'), 'ABCD-2345');
      await tester.ensureVisible(find.text('Pair'));
      await tester.tap(find.text('Pair'));
      await tester.pumpAndSettle();
      expect(find.textContaining('No device showing a code was found'), findsOneWidget);
      expect(find.text('Save location unavailable'), findsNothing);
    });

    testWidgets('showing a code gives the short and the long code', (tester) async {
      final handlers = ready([])
        ..['peer_invite'] = ((_) => {'code': 'DOKIP1:long', 'short': 'WXYZ-6789'});
      await pump(tester, FakeBackend(handlers), home: const DevicesScreen());
      await tester.tap(find.text('Show a code on this device'));
      await tester.pumpAndSettle();
      expect(find.text('WXYZ-6789'), findsOneWidget);
      expect(find.text('DOKIP1:long'), findsOneWidget);
    });

    testWidgets('sync now reports who was reached and what is waiting', (tester) async {
      final handlers = ready([])
        ..['peer_list'] = ((_) => [
          {'id': 'id-odin', 'name': 'Odin', 'introduced_by': null, 'last_sync': null},
          {'id': 'id-deck', 'name': 'Deck', 'introduced_by': 'Odin', 'last_sync': null},
        ])
        ..['peer_sync'] = ((_) => {
          'sessions': [session('Odin', sent: 1, received: 2)],
          'unreachable': [
            ['Deck', 'timed out'],
          ],
        });
      final backend = FakeBackend(handlers);
      await pump(tester, backend, home: const DevicesScreen());
      await tester.ensureVisible(find.text('Sync now'));
      await tester.tap(find.text('Sync now'));
      await tester.pumpAndSettle();
      expect(
        find.text('Synced with Odin: 1 game sent, 2 new saves waiting in Incoming.'),
        findsOneWidget,
      );
      expect(find.text('Not reachable right now · added through Odin'), findsOneWidget);
      expect(backend.calls, isNot(contains('use')), reason: 'syncing never installs');
    });

    testWidgets('the app syncs on its own once devices are paired', (tester) async {
      final handlers = ready([])
        ..['peer_list'] = ((_) => [
          {'id': 'id-odin', 'name': 'Odin', 'introduced_by': null, 'last_sync': null},
        ])
        ..['peer_sync'] = ((_) => {
          'sessions': [session('Odin')],
          'unreachable': [],
        });
      final backend = FakeBackend(handlers);
      final state = AppState(backend, autoSync: true, syncEvery: const Duration(minutes: 2));
      await state.refresh();
      await tester.pump();
      expect(backend.calls.where((c) => c == 'peer_sync').length, 1);
      await tester.pump(const Duration(minutes: 2));
      expect(backend.calls.where((c) => c == 'peer_sync').length, 2);
      state.dispose();
    });
  });
}
