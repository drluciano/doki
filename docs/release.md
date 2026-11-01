# Making a release

A release is a GitHub release with four packages, a checksum file and notes taken from [CHANGELOG.md](../CHANGELOG.md):

| File | What it is |
|---|---|
| `doki-<version>-windows-setup.exe` | Windows installer (per-user, no administrator prompt) |
| `doki-<version>-android.apk` | Android app, signed with Doki's release key |
| `doki-<version>-linux-x86_64.flatpak` | Linux Flatpak for the Steam Deck and PCs |
| `doki-<version>-linux-aarch64.flatpak` | Linux Flatpak for ARM handhelds such as the Odin 3 |
| `SHA256SUMS.txt` | SHA-256 of each package |

The [Release workflow](../.github/workflows/release.yml) builds them through the [App workflow](../.github/workflows/app.yml), so a release passes exactly the same tests and package checks as every test build (Windows install, update and uninstall; Flatpak start and update; the APK's engine and bridge names). It then creates a **draft** release. Nothing is public and no tag exists until the draft is published by hand.

## Steps

1. **Choose the version.** Doki uses `MAJOR.MINOR.PATCH`. Versions below 1.0 are published as pre-releases.
2. **Open a pull request that bumps it** in all three places. The workflow refuses to run if they disagree:
   - `version:` in `app/pubspec.yaml` (keep the `+1` build number; CI replaces it).
   - A new `<release version="…" date="YYYY-MM-DD">` at the top of `<releases>` in `app/linux/packaging/dev.doki.doki.metainfo.xml`. Linux app stores show it.
   - A new `## <version>` section at the top of `CHANGELOG.md`, written for players: what is new, what changed, known limitations. `tools/release-notes.sh` prints the notes it will use.
3. **Merge it to `main`.**
4. **Run the workflow:** Actions → **Release** → **Run workflow** on `main`. It takes about 30 to 60 minutes, mostly the Linux and Android builds.
5. **Review the draft** under [Releases](https://github.com/drluciano/doki/releases): read the notes, and install at least the Android and Windows packages over the previous release on a real device to confirm they update in place and keep data.
6. **Publish it.** Publishing creates the tag `v<version>` on the commit the workflow built.

To rebuild a draft (for example after a fix), delete the draft first; the workflow will not make a second release of the same version. To fix a mistake after publishing, release a new patch version instead of replacing files.

## Android signing key

Android only installs an update over an existing app when both are signed with the same key. Every release, and every test build that has the key, is signed with Doki's release key. **If the key is lost, no future version can update an installed Doki**: every user would have to uninstall, losing Doki's data on that device. Keep two offline backups of the keystore and its passwords, for example in a password manager and on a USB stick.

The key lives only in the repository's Actions secrets, never in the repository:

| Secret | Value |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | The keystore file, base64-encoded on one line |
| `ANDROID_KEYSTORE_PASSWORD` | The keystore password |
| `ANDROID_KEY_ALIAS` | The key's alias, `doki` below |
| `ANDROID_KEY_PASSWORD` | The key's password (the same as the keystore password for a PKCS12 keystore) |

### Creating the key (once)

With a JDK installed (`keytool` comes with it):

```sh
keytool -genkeypair -v -keystore doki-release.jks -storetype PKCS12 \
  -keyalg RSA -keysize 4096 -validity 36500 -alias doki \
  -dname "CN=Doki"
base64 -w0 doki-release.jks > doki-release.jks.txt    # macOS: base64 -i doki-release.jks
```

On Windows PowerShell, encode with `[Convert]::ToBase64String([IO.File]::ReadAllBytes("doki-release.jks")) > doki-release.jks.txt`.

Then, in the repository on GitHub, open **Settings → Secrets and variables → Actions → New repository secret** and add the four secrets above. Paste the contents of `doki-release.jks.txt` as `ANDROID_KEYSTORE_BASE64`. Delete `doki-release.jks.txt` afterwards and keep `doki-release.jks` with your backups.

The next App workflow run signs its APK with the key: the **Check the APK's signature** step prints the certificate (`CN=Doki`). Without the secrets, test builds fall back to a throwaway debug key and the workflow warns; a release refuses to build.

### Building a signed APK locally

```sh
export DOKI_ANDROID_KEYSTORE=/path/to/doki-release.jks
export DOKI_ANDROID_KEYSTORE_PASSWORD=… DOKI_ANDROID_KEY_ALIAS=doki DOKI_ANDROID_KEY_PASSWORD=…
cd app && flutter build apk --release --build-number "$(../tools/android-version-code.sh)"
```

### Version codes

Android also refuses an "update" whose `versionCode` is not larger than the installed one. CI sets it with `tools/android-version-code.sh`: the minutes from 2026-01-01 to the built commit's time, so a later commit always gets a larger number and rebuilding a commit gives the same one.

## Windows signing

The installer is not code-signed yet, so Windows SmartScreen warns on first run ("Windows protected your PC"). Signing needs a code-signing certificate; when there is one, it will be added to the App workflow the same way as the Android key.

## What a release does not change

Users' data is never touched by installing a release. Every package updates in place and keeps checkpoints, paired devices and settings, which live outside the program files: `%APPDATA%\dev.doki\doki` on Windows, `~/.var/app/dev.doki.doki` on Linux, and the app's private storage on Android. See [Updating Doki](wiki/Updating-Doki.md).
