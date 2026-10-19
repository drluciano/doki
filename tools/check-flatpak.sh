#!/usr/bin/env bash
# Checks a Flatpak built by tools/build-flatpak.sh: it installs, every library
# it needs is in the runtime, it starts, and installing a newer build over it
# updates in place and keeps Doki's data (vault and settings).
#   tools/check-flatpak.sh app/build/doki-linux-x86_64.flatpak
# Installs dev.doki.doki for the current user; run it on a CI runner or a
# throwaway machine, not next to a Doki you use.
set -euo pipefail
cd "$(dirname "$0")/.."
bundle="$1"
app=dev.doki.doki
data="$HOME/.var/app/$app"
fail() { echo "FAIL: $*" >&2; exit 1; }

flatpak install --user -y --noninteractive "$bundle"
first="$(flatpak info --user --show-commit "$app")"
echo "ok - installs (commit $first)"

# Every library Doki loads resolves inside the GNOME runtime, including the
# engine, which Dart opens only when the app starts.
missing="$(flatpak run --command=sh "$app" -c '
  loader=$(ls /lib*/ld-linux*.so* /usr/lib/*/ld-linux*.so* 2>/dev/null | head -n1)
  for f in /app/doki/doki /app/doki/lib/*.so; do "$loader" --list "$f"; done' | grep 'not found' || true)"
[ -z "$missing" ] || fail "libraries missing from the runtime: $missing"
echo "ok - every library resolves in the runtime"

# It starts and keeps running (nothing to show it on but a virtual display).
log="$(mktemp)"
set +e
timeout 30 xvfb-run -a dbus-run-session -- flatpak run "$app" >"$log" 2>&1
code=$?
set -e
[ "$code" = 124 ] || { cat "$log"; fail "Doki exited with $code instead of running"; }
grep -q 'Impeller\|flutter' "$log" || { cat "$log"; fail "no sign that Flutter started"; }
echo "ok - starts and keeps running"

# The run above kept its data in the sandbox's own folder, where the vault
# lives (~/.var/app/dev.doki.doki/data/dev.doki.doki). Add a marker standing
# in for a user's vault.
[ -d "$data/data/$app" ] || fail "Doki did not use $data/data/$app"
echo "keep me" > "$data/data/update-check"

# A newer build: same app, new commit, as the next CI build would be.
build=app/build/flatpak-build
date +%s > "$build/files/doki/data/build-stamp"
flatpak build-export app/build/flatpak-repo "$build"
next="$(mktemp -d)/doki-next.flatpak"
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  app/build/flatpak-repo "$next" "$app"
flatpak install --user -y --noninteractive "$next"
second="$(flatpak info --user --show-commit "$app")"
[ "$first" != "$second" ] || fail "the newer build did not replace the installed one"
[ "$(cat "$data/data/update-check")" = "keep me" ] || fail "updating lost Doki's data"
echo "ok - installing a newer build updates in place and keeps data"

# Uninstalling without --delete-data keeps it too.
flatpak uninstall --user -y --noninteractive "$app"
[ -f "$data/data/update-check" ] || fail "uninstalling deleted Doki's data"
echo "ok - uninstalling keeps data"
