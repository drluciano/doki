#!/usr/bin/env bash
# Analyze and test the Flutter app, including tests against the real engine
# through Dart FFI. Needs Flutter and Rust on PATH.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p doki-ffi --locked
windows=false
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) lib="$PWD/target/debug/doki_ffi.dll"; windows=true ;;
  Darwin) lib="$PWD/target/debug/libdoki_ffi.dylib" ;;
  *) lib="$PWD/target/debug/libdoki_ffi.so" ;;
esac
cd app
flutter pub get
# Checked once, on Unix: Windows checkouts may convert line endings.
$windows || dart format --output=none --set-exit-if-changed lib test
flutter analyze
DOKI_FFI_LIB="$lib" flutter test
