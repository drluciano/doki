#!/usr/bin/env bash
# Build the JNI bridge for this machine and drive it from a desktop JVM.
# Needs a JDK (javac, java) on PATH or in JAVA_HOME.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p doki-jni --locked
JAVA_BIN="${JAVA_HOME:+$JAVA_HOME/bin/}"
out="target/jni-desktop-test"
rm -rf "$out" && mkdir -p "$out"
"${JAVA_BIN}javac" -d "$out" \
  android/bridge/src/main/java/dev/doki/bridge/*.java \
  android/bridge/src/desktopTest/java/dev/doki/bridge/*.java
"${JAVA_BIN}java" -Djava.library.path=target/debug -cp "$out" dev.doki.bridge.BridgeTest
