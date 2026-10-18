# The Doki app

The app (`app/`, Flutter) is the main way to use Doki on Android and Windows. It also runs on Linux for development. Every action is one request to the Rust engine; the app holds no safety logic of its own.

| Platform | Engine connection | Save folders |
|---|---|---|
| Windows, Linux | `crates/doki-ffi` through Dart FFI, bundled next to the executable | Local folders, replaced by rename-over |
| Android | Platform channel → `dev.doki.bridge` (Java) → `crates/doki-jni` | Storage Access Framework folders the user picks, replaced by delete-then-rename (safety contract, sections 6 and 9) |

## Using it

1. **Welcome**: name this device. No account or network is needed for local use.
2. **Library → Add game**: give a title and choose the emulator's save folder. Connecting only records where the save lives.
3. **Create checkpoint**: close the game first, then confirm. Doki copies and verifies the files. If nothing changed, it says so and links the existing checkpoint.
4. **Devices**: choose **Show a code on this device**, then type the short code on your other device (same Wi-Fi) or paste the long code (any network). Devices connect directly; any device can add the next one, and there is no host. Doki syncs while it is open on both devices, and **Sync now** syncs at once. Received progress waits in **Incoming**; sync never changes a save. See [peer.md](peer.md). A self-hosted [relay](relay.md) is under **Advanced**.
5. **Incoming → review**: Doki compares the incoming save with yours. If you played on both devices it says so, and both versions stay in history whichever you choose:
   - **Back up and use this save** verifies a backup of your current save, then installs and verifies the incoming one.
   - **Keep alongside** files it in history without prompting again.
   - **Keep my save** records that you kept yours, so other devices do not ask again.
6. **History** (open a game): every checkpoint, which one is the current save, and progress set aside by a choice. **Restore this checkpoint** goes through the same backup and verification as Use.
7. If an install is interrupted (power loss, storage removed), the game shows **A change to this save was interrupted** until you choose **Finish the change** or **Put my save back**. Doki never decides for you.

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

CI (`.github/workflows/app.yml`) runs the tests on Linux and Windows and publishes test builds: `doki-windows` (folder with `doki.exe`) and `doki-android` (debug-signed APK). They are test builds, not releases.

## What is verified, and what is not

- Verified in CI: widget tests against a fake engine; the real engine through FFI (capture, restore, a stale review refused); the Java bridge on a desktop JVM with both replace modes; Android sources compile and the APK carries the engine for arm64 and x86_64.
- Not yet verified: the Storage Access Framework adapter on real hardware and emulator storage providers (F01). Until then, treat Android installs as experimental and keep copies of saves.
- Not built yet: the practice-saves tutorial, a file preview before connecting a folder, QR codes, background sync on Android, checkpoint cleanup and per-device relay keys. Peer-to-peer sync is tested between devices on one machine and over a local network, not yet between an Android device and a PC.
