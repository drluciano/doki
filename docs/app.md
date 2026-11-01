# The Doki app

The app (`app/`, Flutter) is the main way to use Doki on Android, Windows and Linux (a Flatpak for x86_64 and aarch64, aimed at SteamOS and ARM handhelds running Armada). Every action is one request to the Rust engine; the app holds no safety logic of its own.

| Platform | Engine connection | Save folders |
|---|---|---|
| Windows, Linux | `crates/doki-ffi` through Dart FFI, bundled next to the executable | Local folders, replaced by rename-over |
| Android | Platform channel → `dev.doki.bridge` (Java) → `crates/doki-jni` | Storage Access Framework folders the user picks, replaced by delete-then-rename (safety contract, sections 6 and 9) |

## Using it

1. **Welcome**: name this device. No account or network is needed for local use.
2. **Library → Add game**: give a title and choose the emulator's save folder. Connecting only records where the save lives. Choose **Whole system** to connect an emulator's whole save folder: each subfolder or set of save files sharing a name becomes its own game, and new ones are added automatically (`add_system`, engine `systems.rs`). Library shows games as covers grouped by system; covers and banners come from SteamGridDB with the user's own API key, or from images the user picks (**Game art**, `app/lib/art.dart`). Art is kept in the app support folder on this device only.
3. **Checkpoints happen on their own.** Doki looks at each save folder about every 15 seconds and saves a checkpoint once a change has stayed the same for 30 seconds, so a save is never caught halfway through being written. **Checkpoint now** on a game does it immediately (close the game first). On Android a background watcher keeps doing this, and syncing, while you play in an emulator; it shows a notification and can be turned off under **Devices** or from the notification. On Windows, closing or minimizing Doki hides it in the tray (notification area), where it keeps checking and syncing; click the icon to open it, or right-click for **Sync now** and **Quit**. **Devices** has switches to turn the tray off (closing then quits) and to start Doki with Windows, in the tray. Opening Doki again while it is in the tray brings back the running copy. On Linux, closing Doki hides it in the tray (a StatusNotifierItem, shown by KDE Plasma and most other desktops; plain GNOME needs the AppIndicator extension), with the same **Sync now** and **Quit** menu; **Devices** has **Keep Doki in the tray** and **Start Doki when you sign in** (the Background portal in the Flatpak, else an entry in `~/.config/autostart`). Without a tray, closing quits. SteamOS Game Mode has no tray or sign-in start, so there Doki works only while open.
   - **Edit game** (the menu on a game) changes its title, its save folder, or turns automatic checkpoints off. No save files are touched.
   - **Remove from this device** stops watching it here. Its save files stay, every checkpoint is kept, other devices keep the game, and it is listed under **On your other devices** so you can set it up again.
   - A removed game's menu (⋮ under **On your other devices**) has **Delete from this device**, which deletes its checkpoints here and stops listing it. For a game set up only on another device the same menu says **Don't set up on this device**. The save folder is never touched and other devices keep their copies.
4. **Devices**: choose **Show a code on this device**, then type the short code on your other device (same Wi-Fi) or paste the long code (any network). Devices connect directly; any device can add the next one, and there is no host. Doki syncs while it is open on both devices, and **Sync now** syncs at once. Received progress waits in **Incoming**; sync never changes a save. Games added on another device appear under **On your other devices** in Library; tap one and choose where it saves here. See [peer.md](peer.md). A self-hosted [relay](relay.md) is under **Advanced**.
5. **Incoming → review**: Doki compares the incoming save with yours. If you played on both devices it says so, and both versions stay in history whichever you choose:
   - **Back up and use this save** verifies a backup of your current save, then installs and verifies the incoming one.
   - **Keep alongside** files it in history and removes it from Incoming; because no decision is recorded, the other device may offer it again.
   - **Keep my save** records that you kept yours, so other devices do not ask again.
6. **History** (open a game): every checkpoint, which one is the current save, and progress set aside by a choice. **Restore this checkpoint** goes through the same backup and verification as Use. **Delete from this device** deletes a checkpoint's files here (not the current save or pinned ones); other devices keep theirs.
   - The **History** tab's **Keep checkpoints for** sets how long this device keeps checkpoints: forever (the default), 1 year, 90, 30 or 7 days. Picking a shorter time first says how many checkpoints would go. The 10 newest of each game, current saves and pinned checkpoints are always kept.
7. **Notifications** (switches under **Devices**): when Doki saves a checkpoint on its own, when a save folder starts changing, and when progress arrives from another device. Android and Linux show them as normal notifications (allow them when asked on Android); Windows shows them from the tray icon, so they need **Keep Doki in the tray** on. A newer notification about the same game replaces the older one.
8. If an install is interrupted (power loss, storage removed), the game shows **A change to this save was interrupted** until you choose **Finish the change** or **Put my save back**. Doki never decides for you.

The app opens on a doki splash. It waits until the vault has loaded, then the letters arrive, the wordmark holds for a moment and the screen fades into the library (about two seconds; shorter and without movement when the system asks for reduced motion). On Android the system launch screen before it uses the same page color, light or dark to match the device.

**Settings** (the last tab):

- **Theme**: Follow system (the default), Light, Dark or OLED (dark with true black behind everything, for OLED screens). The change applies at once.
- **Border color**: the color of the soft border that shows where you are with a mouse, keyboard or controller. Each color has a light and a dark shade so it stays visible on every theme. High contrast mode always uses the text color.
- Both choices are stored in `appearance.json` in the app's support folder, next to the vault. Deleting that file returns to the defaults; nothing in it touches saves.
- **Help and how-tos**: short how-tos (adding a game or a whole system, game art, automatic checkpoints, pairing, sync, Incoming, using and restoring saves, deleting checkpoints and games, how long checkpoints are kept, notifications, recovery, keyboard and controller). Their text lives in `app/lib/screens/help.dart`, so update it when the wording of a button it names changes.

Every error says what happened, whether your playable save changed, and what to do next, with technical details under **Details**.

## Building

Needs Rust (see `rust-version` in `Cargo.toml`) and Flutter 3.47.

```sh
tools/app-test.sh                      # format, analyze, widget tests and real-engine tests
cd app && flutter run -d windows       # or -d linux; the desktop build compiles the engine itself
```

Android additionally needs the NDK, the Rust Android targets and `cargo-ndk`:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --locked
tools/build-android-engine.sh          # writes app/android/app/src/main/jniLibs
cd app && flutter build apk
```

CI (`.github/workflows/app.yml`) runs the tests on Linux and Windows and publishes test builds: `doki-windows-setup` (installer), `doki-windows` (folder with `doki.exe`), `doki-android` (APK, signed with Doki's release key when the repository secrets have it, otherwise a throwaway debug key) and `doki-linux-x86_64` / `doki-linux-aarch64` (Flatpak bundles, built by `tools/build-flatpak.sh`; see the wiki page *Installing on Linux*). They are test builds, not releases; releases are built from the same workflow (see [release.md](release.md)).

## What is verified, and what is not

- Verified in CI: widget tests against a fake engine; the real engine through FFI (capture, restore, a stale review refused); the Java bridge on a desktop JVM with both replace modes; Android sources compile and the APK carries the engine for arm64 and x86_64; on Linux, the tray icon and its menu, closing to the tray, single instance and hidden starts (`tools/check-linux-desktop.sh`, with a stand-in tray), and for each Flatpak that every library resolves in the runtime, it starts, and installing a newer build keeps the vault (`tools/check-flatpak.sh`).
- Not yet verified: the Storage Access Framework adapter on real hardware and with emulator storage providers, and device-to-device sync between an Android device and a PC (automated tests cover devices on one machine and on a local network). Until then, treat Android installs as experimental and keep copies of saves. The Linux Flatpak has not been run on a Steam Deck or an Odin 3, or against a real tray and Background portal; treat it as experimental too.
- Verified in CI as well: the release APK keeps the bridge class and method names the engine calls (`tools/check-apk-bridge.sh`).
- Not built yet: the practice-saves tutorial, a file preview before connecting a folder, QR codes and per-device relay keys.
- Android updates: APKs signed with Doki's release key install over each other and keep the vault, with a `versionCode` that grows with each commit (`tools/android-version-code.sh`). Builds made before the key existed used a new debug key each time (#82), so moving from one of them needs one last uninstall; the wiki page *Updating Doki* has the steps.
