// Drives the real Rust engine through Dart FFI. Runs when DOKI_FFI_LIB points
// at a built doki_ffi library (tools/app-test.sh sets it); skipped otherwise.
import 'dart:io';

import 'package:doki/backend.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  final lib = Platform.environment['DOKI_FFI_LIB'];
  final skip = lib == null || lib.isEmpty ? 'DOKI_FFI_LIB not set' : false;

  test('capture, restore and errors through the real engine', () async {
    final tmp = await Directory.systemTemp.createTemp('doki-ffi');
    addTearDown(() => tmp.delete(recursive: true));
    final save = await Directory('${tmp.path}/save').create();
    final doki = FfiBackend('${tmp.path}/vault', library: lib);
    final file = File('${save.path}/game.sav');

    await expectLater(
      doki.call('device'),
      throwsA(isA<DokiError>().having((e) => e.code, 'code', 'NotFound')),
    );
    await doki.call('init', {'name': 'Desk'});
    await doki.call('add', {'name': 'Emerald', 'location': save.path});
    await expectLater(
      doki.call('capture', {'game': 'Emerald'}),
      throwsA(isA<DokiError>().having((e) => e.code, 'code', 'BundleIncomplete')),
    );

    await file.writeAsString('one');
    final first = (await doki.call('capture', {'game': 'Emerald'}))['checkpoint'] as String;
    await file.writeAsString('two');
    await doki.call('capture', {'game': 'Emerald'});

    final games = await doki.call('games') as List;
    expect(games.single['status'], 'protected');

    final review = await doki.call('review', {'game': 'Emerald', 'checkpoint': first});
    expect(review['relation']['kind'], 'behind');
    expect(await file.readAsString(), 'two', reason: 'review never changes the save');

    final used = await doki.call('use', {'review': review});
    expect(used['installed'], first);
    expect(await file.readAsString(), 'one');

    final history = await doki.call('history', {'game': 'Emerald'}) as List;
    expect(history.length, 3);
    expect(history.where((e) => e['set_aside'] == true).length, 1);

    // The same review is stale now: the engine refuses it.
    await expectLater(
      doki.call('use', {'review': review}),
      throwsA(isA<DokiError>().having((e) => e.code, 'code', 'LocalChangedSinceReview')),
    );
  }, skip: skip);
}
