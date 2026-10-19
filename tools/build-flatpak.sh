#!/usr/bin/env bash
# Packages the Linux app as a Flatpak bundle for this machine's architecture.
# Run after `flutter build linux --release` (in app/). Needs flatpak and
# flatpak-builder; the GNOME runtime comes from Flathub.
#   tools/build-flatpak.sh            # writes app/build/doki-linux-<arch>.flatpak
set -euo pipefail
cd "$(dirname "$0")/.."
arch="$(flatpak --default-arch)"
case "$arch" in
  x86_64) flutter_arch=x64 ;;
  aarch64) flutter_arch=arm64 ;;
  *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
esac
bundle="app/build/linux/$flutter_arch/release/bundle"
if [ ! -x "$bundle/doki" ]; then
  echo "no release build at $bundle; run 'flutter build linux --release' in app/" >&2
  exit 1
fi
rm -rf app/build/flatpak-bundle
cp -a "$bundle" app/build/flatpak-bundle

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak-builder --user --force-clean --install-deps-from=flathub \
  --state-dir=app/build/flatpak-state --repo=app/build/flatpak-repo \
  app/build/flatpak-build app/linux/packaging/dev.doki.doki.yml
# --runtime-repo lets `flatpak install` fetch the GNOME runtime from Flathub
# on a device that does not have it yet.
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  app/build/flatpak-repo "app/build/doki-linux-$arch.flatpak" dev.doki.doki
ls -l "app/build/doki-linux-$arch.flatpak"
