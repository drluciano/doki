# Roadmap and status

Doki is **pre-release**. This page says what works today, what is verified and how, and what is next. It changes as work lands; the [safety contract](https://github.com/drluciano/doki/blob/main/docs/spec/safety-contract.md) (section 13) is the authoritative list of what is tested.

## Built and tested in CI

- The engine: checkpoints, review, install with verified backup, recheck before replacing, recovery after an interruption (with a simulated crash at every step), handoff files.
- The `doki` command line.
- Device-to-device pairing and sync with no host: short and long codes, trusted devices spreading through the group, device removal, both-played detection.
- The optional self-hosted relay and its encryption.
- The Flutter app on Windows and Linux, tested against the real engine; Android builds with the engine for arm64 and x86_64.
- The Android bridge, tested on a desktop JVM with both ways of replacing files.

## Not yet verified

- The Android folder adapter on real hardware and with emulator storage. First target: Eden on an AYN Odin 3 (Android 15).
- Sync between a real Android device and a Windows PC.
- Syncing on a network with no internet connection.

Until these are verified, treat Android installs as experimental and keep copies of your saves.

## Not built yet

- A practice tutorial with sample saves, before connecting a real folder.
- A preview of a folder's files before connecting it.
- QR codes for pairing.
- Background sync on Android.
- Finding devices on a local network with no internet (mDNS).
- Checkpoint cleanup, with a preview and pins always kept.
- Encrypted handoff files.
- Per-device keys for the relay.
- Emulator profiles and a compatibility list.
- Published releases.

## Platforms

| Platform | Status |
|---|---|
| Android | First target. Experimental until verified on hardware. |
| Windows | First target. |
| Linux | Development only. |
| macOS, iOS | Not planned for the first release. |

Open decisions and the defaults used meanwhile are in [open-questions.md](https://github.com/drluciano/doki/blob/main/docs/spec/open-questions.md). Work is tracked in [issues](https://github.com/drluciano/doki/issues) and the [project board](https://github.com/users/drluciano/projects/2).
