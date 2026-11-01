# Roadmap and status

Doki is **pre-release**. This page says what works today, what is verified and how, and what is next. It changes as work lands; the [safety contract](https://github.com/drluciano/doki/blob/main/docs/spec/safety-contract.md) (section 13) is the authoritative list of what is tested.

## Built and tested in CI

- The engine: checkpoints, review, install with verified backup, recheck before replacing, recovery after an interruption (with a simulated crash at every step), handoff files.
- Automatic checkpoints once a save settles, whole-emulator tracking, editing and removing games, deleting checkpoints and **Keep checkpoints for** retention.
- The `doki` command line.
- Device-to-device pairing and sync with no host: short and long codes, trusted devices spreading through the group, device removal, both-played detection.
- The optional self-hosted relay and its encryption.
- The Flutter app on Windows and Linux, tested against the real engine; Android builds with the engine for arm64 and x86_64, and CI checks the release APK keeps the names the engine calls.
- In the app: the Android background watcher, the Windows and Linux tray and start at sign-in, notifications, themes and border colors, game art from SteamGridDB or your own images, and in-app help.
- The Android bridge, tested on a desktop JVM with both ways of replacing files.
- Releases: the Windows installer, the Android APK signed with Doki's own key so updates install in place, and both Linux Flatpaks, with checksums, built and checked by CI ([how releases are made](https://github.com/drluciano/doki/blob/main/docs/release.md)).

## Not yet verified

- The Android folder adapter on real hardware and with emulator storage. First target: Eden on an AYN Odin 3 (Android 15).
- Sync between a real Android device and a Windows PC.
- The Linux Flatpak on a Steam Deck and on an Odin 3 with Armada: saving, restoring and syncing a real save.
- Syncing on a network with no internet connection.

Until these are verified, treat Android installs as experimental and keep copies of your saves.

## Not built yet

- A practice tutorial with sample saves, before connecting a real folder.
- A preview of a folder's files before connecting it.
- QR codes for pairing.
- Finding devices on a local network with no internet (mDNS).
- Encrypted handoff files.
- Per-device keys for the relay.
- Emulator profiles and a compatibility list.
- A signed Windows installer (Windows SmartScreen warns until then).

## Platforms

| Platform | Status |
|---|---|
| Android | First target. Experimental until verified on hardware. |
| Windows | First target. |
| Linux | A Flatpak for x86_64 (Steam Deck) and aarch64 (Odin 3 on Armada), with the tray and start at sign-in. Not yet verified on a real device. |
| macOS, iOS | Not planned for the first release. |

Work is tracked in [issues](https://github.com/drluciano/doki/issues), grouped by milestone.
