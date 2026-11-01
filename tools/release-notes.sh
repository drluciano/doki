#!/usr/bin/env bash
# Checks that a version is ready to release and prints its release notes.
#   tools/release-notes.sh            # version from app/pubspec.yaml
#   tools/release-notes.sh 0.2.0      # must match app/pubspec.yaml
# Fails unless app/pubspec.yaml, the Linux AppStream file and CHANGELOG.md all
# describe the same version. Prints that version's CHANGELOG.md section.
set -euo pipefail
cd "$(dirname "$0")/.."
pubspec=$(sed -n 's/^version: *\([0-9][0-9.]*\).*/\1/p' app/pubspec.yaml)
version="${1:-$pubspec}"
version="${version#v}"
fail() { echo "release-notes: $*" >&2; exit 1; }
[ -n "$pubspec" ] || fail "no version in app/pubspec.yaml"
[ "$version" = "$pubspec" ] || fail "asked for $version but app/pubspec.yaml says $pubspec"
grep -q "<release version=\"$version\"" app/linux/packaging/dev.doki.doki.metainfo.xml ||
  fail "app/linux/packaging/dev.doki.doki.metainfo.xml has no <release version=\"$version\">"
# The section runs from "## <version>" (optionally "## [<version>]") to the
# next "## " heading.
notes=$(awk -v v="$version" '
  /^## / { if (found) exit; h=$2; gsub(/[][]/, "", h); if (h == v) { found=1; next } }
  found { print }
' CHANGELOG.md)
[ -n "$(printf '%s' "$notes" | tr -d '[:space:]')" ] || fail "CHANGELOG.md has no \"## $version\" section"
printf '%s\n' "$notes" | sed -e '/./,$!d'
