#!/usr/bin/env bash
# Runs tools/check-linux-desktop.py against a built Linux bundle on a private
# X display and session bus. Needs xvfb, dbus, python3-gi and python3-xlib.
#   tools/check-linux-desktop.sh app/build/linux/x64/release/bundle/doki
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# The python3 that has the distribution's gi bindings.
python="${DOKI_GI_PYTHON:-/usr/bin/python3}"
exec xvfb-run -a -s "-screen 0 1600x900x24" \
  dbus-run-session -- "$python" "$here/check-linux-desktop.py" "$1"
