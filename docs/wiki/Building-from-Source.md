# Building from source

## Engine and command line

Needs Rust 1.91 or newer.

```sh
git clone https://github.com/drluciano/doki
cd doki
cargo build --release          # target/release/doki and target/release/doki-relay
```

Checks to run before a pull request, the same as CI (`.github/workflows/rust.yml`, Linux and Windows):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Desktop app (Windows, Linux)

Needs Rust and Flutter 3.47.

```sh
tools/app-test.sh                  # format, analyze, widget tests and real-engine tests
cd app && flutter run -d windows   # or -d linux; this also compiles the engine
```

The Windows installer is built with [Inno Setup](https://jrsoftware.org/isinfo.php) 6.3 or newer from `app/windows/installer/doki.iss`, after `flutter build windows --release`: `iscc /DAppVersion=0.1.0 app\windows\installer\doki.iss` writes `app/build/windows/installer/doki-setup-0.1.0.exe`. CI also copies the Visual C++ runtime DLLs next to `doki.exe` first, so the app runs on PCs without it; `tools/check-windows-installer.ps1` installs, updates and uninstalls the result to check it. The app icon and wizard images are drawn by `app/windows/installer/make_art.py`.

The Linux Flatpak packages the release bundle (Flutter cannot build inside flatpak-builder's offline sandbox). With `flatpak` and `flatpak-builder` installed, on the architecture you want (x86_64 or aarch64):

```sh
cd app && flutter build linux --release && cd ..
tools/build-flatpak.sh             # writes app/build/doki-linux-<arch>.flatpak
tools/check-linux-desktop.sh app/build/linux/x64/release/bundle/doki   # tray and single-instance checks (Xvfb, python3-gi, python3-xlib)
tools/check-flatpak.sh app/build/doki-linux-x86_64.flatpak            # installs it for your user: use a throwaway machine
```

The manifest, desktop entry, AppStream file and icons are in `app/linux/packaging` (icons drawn by `make_icons.py` from the Windows art). The tray, sign-in start and notifications live in `app/linux/runner`.

## Android app

Needs the Android SDK and NDK, the Rust Android targets and `cargo-ndk`:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --locked
tools/build-android-engine.sh      # writes app/android/app/src/main/jniLibs
cd app && flutter build apk      # signed with a debug key unless DOKI_ANDROID_KEYSTORE is set (see docs/release.md)
```

The Java bridge can be tested without a device: `tools/jni-desktop-test.sh`.

## Test builds from CI

`.github/workflows/app.yml` runs the app tests on Linux and Windows and uploads these artifacts on every run: `doki-windows-setup` (the installer, unsigned), `doki-windows` (the same app as a plain folder), `doki-android` (the APK, signed with Doki's key when the repository has it), and `doki-linux-x86_64` and `doki-linux-aarch64` (Flatpak bundles). They are test builds, not releases.

Releases are built by `.github/workflows/release.yml` through the same workflow. [docs/release.md](https://github.com/drluciano/doki/blob/main/docs/release.md) is the runbook: bumping the version, the changelog, the Android signing key and publishing.

## Contributing

Read [CONTRIBUTING.md](https://github.com/drluciano/doki/blob/main/CONTRIBUTING.md). Key rules: any code that writes save files goes through the journaled install with a crash-point test per step; update the safety contract in the same pull request as behaviour it describes; use synthetic test saves only.
