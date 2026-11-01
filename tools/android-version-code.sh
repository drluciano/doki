#!/usr/bin/env bash
# Prints the Android versionCode for the commit that is checked out: minutes
# from 2026-01-01 to the commit's time. Later commits get larger numbers, so a
# newer APK always installs over an older one, and rebuilding the same commit
# gives the same number. Stays below Android's limit (2100000000) for
# thousands of years.
set -euo pipefail
commit_time=$(git -C "$(dirname "$0")/.." log -1 --format=%ct)
echo $(( (commit_time - 1767225600) / 60 ))
