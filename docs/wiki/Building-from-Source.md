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

## Android app

Needs the Android SDK and NDK, the Rust Android targets and `cargo-ndk`:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --locked
tools/build-android-engine.sh      # writes app/android/app/src/main/jniLibs
cd app && flutter build apk
```

The Java bridge can be tested without a device: `tools/jni-desktop-test.sh`.

## Test builds from CI

`.github/workflows/app.yml` runs the app tests on Linux and Windows and uploads two artifacts on every run: `doki-windows` and `doki-android` (debug-signed APK). They are test builds, not releases.

## Contributing

Read [CONTRIBUTING.md](https://github.com/drluciano/doki/blob/main/CONTRIBUTING.md). Key rules: any code that writes save files goes through the journaled install with a crash-point test per step; update the safety contract in the same pull request as behaviour it describes; use synthetic test saves only.
