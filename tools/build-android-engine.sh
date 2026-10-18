#!/usr/bin/env bash
# Build the engine's JNI library for the Android app into
# app/android/app/src/main/jniLibs. Needs the Android NDK (ANDROID_NDK_HOME
# or a default SDK location), cargo-ndk and the Rust Android targets:
#   rustup target add aarch64-linux-android x86_64-linux-android
#   cargo install cargo-ndk --locked
# arm64-v8a covers current handhelds and phones; x86_64 covers the emulator.
set -euo pipefail
cd "$(dirname "$0")/.."
profile="${1:-release}"
flags=(--locked)
[ "$profile" = release ] && flags+=(--release)
cargo ndk --platform 24 -t arm64-v8a -t x86_64 \
  -o app/android/app/src/main/jniLibs \
  build -p doki-jni "${flags[@]}"
