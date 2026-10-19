# Doki app

The Flutter app for Android and Windows (and Linux, for development). What it does, how to build it and what is tested are in [docs/app.md](../docs/app.md); how it looks and moves is in [docs/design.md](../docs/design.md).

| Path | What it is |
|---|---|
| `lib/backend.dart` | The only connection to the engine: Dart FFI on desktop, a platform channel on Android. |
| `lib/state.dart` | App state, the automatic checkpoint and sync timers, and background settings. |
| `lib/screens/` | Library, Incoming, History, Devices, Settings, game and system pages, review, add and edit game, game art, Help, splash. |
| `lib/widgets/` | Shared pieces: covers, the focus border (`focus_halo.dart`), retention picker, games on other devices. |
| `lib/copy.dart` | User-facing wording for errors and relations. |
| `lib/theme.dart`, `lib/appearance.dart`, `lib/motion.dart` | Light, Dark and OLED palettes, saved appearance choices, transitions. |
| `lib/art.dart` | SteamGridDB lookups and locally stored game art. |
| `android/app/src/main/java/dev/doki/app/` | `MainActivity` (channel, folder picker), `SafSaveFolder` (Storage Access Framework adapter), `WatchService` (background watcher), `Notifier` (notifications). |
| `windows/runner/` | Windows host: the tray icon, single-instance handling and tray notifications. |

Run `tools/app-test.sh` from the repository root before opening a pull request.
