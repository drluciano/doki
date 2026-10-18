#!/usr/bin/env bash
# Check that a release APK still has the Java bridge names the native engine
# calls by name (crates/doki-jni). R8 renames classes and methods that only
# native code uses, which breaks every save-folder call at runtime with
# NoSuchMethodError (see app/android/app/proguard-rules.pro).
#
# Usage: tools/check-apk-bridge.sh path/to/app-release.apk
# Needs dexdump from the Android SDK build-tools ($ANDROID_HOME).
set -euo pipefail

apk=${1:?usage: $0 app-release.apk}
dexdump=${DEXDUMP:-$(ls -d "${ANDROID_HOME:?set ANDROID_HOME or DEXDUMP}"/build-tools/*/dexdump | sort -V | tail -1)}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
unzip -q -o "$apk" 'classes*.dex' -d "$work"
for dex in "$work"/classes*.dex; do "$dexdump" "$dex"; done > "$work/dump.txt"

# Prints "Lclass; name type" for every method in the dump.
awk '
  /^    #[0-9]+ *: \(in L/ { cls = $NF; sub(/\)$/, "", cls); next }
  /^      name *: / { name = $3; gsub(/\x27/, "", name); next }
  /^      type *: / { type = $3; gsub(/\x27/, "", type); print cls, name, type }
' "$work/dump.txt" > "$work/methods.txt"

missing=0
need() {
  if ! grep -qxF "$1 $2 $3" "$work/methods.txt"; then
    echo "missing in APK: $1 $2 $3" >&2
    missing=1
  fi
}

opener='Ldev/doki/bridge/SaveFolderOpener;'
folder='Ldev/doki/bridge/SaveFolder;'
need "$opener" open "(Ljava/lang/String;)$folder"
need "$folder" describe '()Ljava/lang/String;'
need "$folder" replaceMode '()I'
need "$folder" checkAvailable '()V'
need "$folder" list '()[Ljava/lang/String;'
need "$folder" read '(Ljava/lang/String;)[B'
need "$folder" stage '(Ljava/lang/String;Ljava/lang/String;[B)V'
need "$folder" readStaged '(Ljava/lang/String;Ljava/lang/String;)[B'
need "$folder" commitStaged '(Ljava/lang/String;Ljava/lang/String;)V'
need "$folder" remove '(Ljava/lang/String;)V'
need "$folder" discardStaging '(Ljava/lang/String;)V'
need 'Ldev/doki/bridge/Doki;' call "(Ljava/lang/String;Ljava/lang/String;$opener)Ljava/lang/String;"
# The app's implementations must keep the interface method names too.
need 'Ldev/doki/app/SafSaveFolder;' describe '()Ljava/lang/String;'
need 'Ldev/doki/app/SafSaveFolder;' commitStaged '(Ljava/lang/String;Ljava/lang/String;)V'

if [ "$missing" -ne 0 ]; then
  echo "The release APK renamed bridge methods the engine calls by name." >&2
  exit 1
fi
echo "APK bridge names intact ($(wc -l < "$work/methods.txt") methods checked against)."
