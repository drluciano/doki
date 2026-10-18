import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:doki/art.dart';
import 'package:doki/backend.dart';
import 'package:doki/screens/system.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'fake_backend.dart';
import 'widget_test.dart' show pump, ready;

/// Smallest valid PNG (1x1, transparent).
final png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
);

Map<String, dynamic> system(String id, String name, {List<String> hidden = const []}) => {
  'system_id': id,
  'name': name,
  'location': '/saves/$name',
  'created_at': '2026-10-18T09:00:00Z',
  'auto_capture': true,
  'ignored': hidden,
};

void main() {
  group('systems', () {
    setUp(() {
      final view = TestWidgetsFlutterBinding.instance.platformDispatcher.implicitView!;
      view.physicalSize = const Size(900, 2200);
      view.devicePixelRatio = 1;
    });
    tearDown(() => TestWidgetsFlutterBinding.instance.platformDispatcher.implicitView!.reset());

    testWidgets('the library groups games found in a system under its name', (tester) async {
      final handlers = ready([
        game('protected', id: 'b1', name: 'Zelda', system: 's1'),
        game('no_checkpoint', id: 'b2', name: 'Metroid', system: 's1'),
        game('protected', id: 'b3', name: 'Emerald'),
      ])..['systems'] = ((_) => [system('s1', 'Eden')]);
      await pump(tester, FakeBackend(handlers));
      expect(find.text('Eden'), findsOneWidget);
      expect(find.text('2 games'), findsOneWidget);
      expect(find.text('Games'), findsOneWidget, reason: 'games on their own get a heading');
      expect(find.text('Zelda'), findsOneWidget);
      expect(find.text('No checkpoint yet'), findsOneWidget, reason: 'status stays in words');
      expect(
        tester.getTopLeft(find.text('Metroid')).dy,
        lessThan(tester.getTopLeft(find.text('Emerald')).dy),
        reason: 'system games come first',
      );
    });

    testWidgets('a whole emulator folder is added as a system', (tester) async {
      Map<String, dynamic>? added;
      final handlers = ready([])
        ..['add_system'] = ((args) {
          added = args;
          return system('s1', 'Eden');
        });
      final backend = FakeBackend(handlers)..folder = const PickedFolder('/saves/eden', 'eden');
      await pump(tester, backend);
      await tester.tap(find.text('Add game'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Whole system'));
      await tester.pumpAndSettle();
      expect(find.textContaining('games you play later are added automatically'), findsOneWidget);
      await tester.enterText(find.widgetWithText(TextField, 'System name'), 'Eden');
      await tester.tap(find.text('Choose'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Connect folder'));
      await tester.pumpAndSettle();
      expect(added, {'name': 'Eden', 'location': '/saves/eden'});
      expect(backend.calls, isNot(contains('add')));
    });

    testWidgets('a system on another device is set up here in one tap', (tester) async {
      Map<String, dynamic>? added;
      final handlers = ready([])
        ..['remote_systems'] = ((_) => added != null
            ? []
            : [
                {'system_id': 's1', 'name': 'Eden', 'from_device': 'Deck'},
              ])
        ..['unmapped'] = ((_) => [
          {
            'bundle_id': 'b9',
            'name_hint': 'Zelda',
            'from_device': 'Deck',
            'has_progress': true,
            'member': {'system_id': 's1', 'kind': 'folder', 'name': 'Zelda'},
          },
        ])
        ..['add_system'] = ((args) {
          added = args;
          return system('s1', 'Eden');
        });
      final backend = FakeBackend(handlers)..folder = const PickedFolder('/saves/eden', 'eden');
      await pump(tester, backend);
      expect(find.text('Systems on your other devices'), findsOneWidget);
      expect(find.textContaining('1 game is waiting for it'), findsOneWidget);
      expect(find.text('On your other devices'), findsNothing, reason: 'Zelda comes with Eden');

      await tester.tap(find.text('Eden'));
      await tester.pumpAndSettle();
      expect(find.text('Already set up on Deck'), findsOneWidget);
      await tester.tap(find.text('Choose'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Connect folder'));
      await tester.pumpAndSettle();
      expect(added, {'name': 'Eden', 'location': '/saves/eden', 'link': 's1'});
    });

    testWidgets('a removed game can be tracked again from its system', (tester) async {
      String? key;
      final handlers = ready([game('protected', id: 'b1', name: 'Metroid', system: 's1')])
        ..['systems'] = ((_) => [
          system('s1', 'Eden', hidden: key == null ? ['folder:Zelda'] : []),
        ])
        ..['track_again'] = ((args) {
          key = args['key'] as String;
          return [];
        });
      await pump(tester, FakeBackend(handlers), home: const SystemScreen(systemId: 's1'));
      expect(find.text('1 game tracked'), findsOneWidget);
      expect(find.text('Not tracked'), findsOneWidget);
      await tester.tap(find.text('Track again'));
      await tester.pumpAndSettle();
      expect(key, 'folder:Zelda');
      expect(find.text('Not tracked'), findsNothing);
    });
  });

  group('art', () {
    test('search terms drop tags, extensions and title ids', () {
      expect(
        searchTerm('Pokemon - Emerald Version (USA, Europe) [!]'),
        'Pokemon - Emerald Version',
      );
      expect(searchTerm('Super_Mario_World.srm'), 'Super Mario World');
      expect(searchTerm('0100F2C0115B6000'), isNull);
      expect(searchTerm('(USA)'), isNull);
    });

    test('only images are stored, and replacing one swaps the file', () async {
      final dir = await Directory.systemTemp.createTemp('doki-art');
      addTearDown(() => dir.delete(recursive: true));
      final store = ArtStore(dir);
      await store.load();
      expect(
        () => store.setImage('b1', ArtKind.cover, Uint8List.fromList(utf8.encode('not an image'))),
        throwsA(isA<ArtError>()),
      );
      await store.setImage('b1', ArtKind.cover, png);
      final first = store.image('b1', ArtKind.cover)!;
      expect(first.path, endsWith('.png'));
      await store.setImage('b1', ArtKind.cover, png);
      final second = store.image('b1', ArtKind.cover)!;
      expect(second.path, isNot(first.path), reason: 'a new name so no stale image is shown');

      await store.updateSettings(store.settings.copyWith(steamGridDbKey: 'k', wideCovers: true));
      final reopened = ArtStore(dir);
      await reopened.load();
      expect(reopened.image('b1', ArtKind.cover)!.path, second.path);
      expect(reopened.settings.steamGridDbKey, 'k');
      expect(reopened.settings.wideCovers, isTrue);
      await reopened.removeImage('b1', ArtKind.cover);
      expect(reopened.image('b1', ArtKind.cover), isNull);
    });

    test('SteamGridDB: search, pick the top cover, and a bad key explained', () {
      // The test binding answers every request with 400; talk to a real
      // local server instead.
      return HttpOverrides.runWithHttpOverrides(_steamGridDbScenario, _RealHttp());
    });
  });
}

class _RealHttp extends HttpOverrides {}

Future<void> _steamGridDbScenario() async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  addTearDown(server.close);
  final base = 'http://127.0.0.1:${server.port}';
  final seen = <String>[];
  server.listen((req) async {
    seen.add('${req.uri.path}?${req.uri.query}');
    final res = req.response;
    if (req.uri.path == '/img.png') {
      // The CDN: the API key is never sent there.
      expect(req.headers.value('authorization'), isNull);
      res.add(png);
    } else if (req.headers.value('authorization') != 'Bearer good') {
      res.statusCode = 401;
    } else if (req.uri.path.startsWith('/search/autocomplete/')) {
      res.write(
        jsonEncode({
          'success': true,
          'data': [
            {'id': 42, 'name': 'Pokémon Emerald', 'release_date': 1095724800},
          ],
        }),
      );
    } else if (req.uri.path == '/grids/game/42' || req.uri.path == '/heroes/game/42') {
      res.write(
        jsonEncode({
          'success': true,
          'data': [
            {'url': '$base/img.png', 'thumb': '$base/thumb.png'},
          ],
        }),
      );
    } else {
      res.statusCode = 404;
    }
    await res.close();
  });

  final api = SteamGridDb('good', base: base);
  final games = await api.search('Pokémon Emerald');
  expect(games.single.id, 42);
  expect(games.single.year, 2004);
  expect(seen.last, startsWith('/search/autocomplete/Pok%C3%A9mon%20Emerald'));
  final covers = await api.images(42, ArtKind.cover);
  expect(seen.last, contains('dimensions=600x900'));
  expect(await api.download(covers.first.url), png);
  await api.images(42, ArtKind.cover, wide: true);
  expect(seen.last, contains('dimensions=920x430'));
  expect(
    () => SteamGridDb('bad', base: base).search('x'),
    throwsA(isA<ArtError>().having((e) => e.message, 'message', contains('API key'))),
  );
}
