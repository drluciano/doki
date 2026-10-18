# F01 read-only SAF probe

Experimental native Java harness, not Doki. The original 0.1 probe passed hosted compile/lint and the owner reported initial on-device metadata access. Version 0.2 adds a remembered-folder recheck; its build and on-device checks are pending until separately evidenced. Intended evidence target: owner-reported AYN Odin3 / Android 15 / Eden 0.2.1. No support claim follows from source inspection.

## Hosted validation

The workflow `.github/workflows/f01-saf-probe.yml` validates this project at `tools/f01-saf-probe`. It compiles/lints only this harness on path-filtered pull requests or manual dispatch, using a hosted Ubuntu runner, JDK17, Gradle8.11.1 and SDK35/Build Tools35.0.0. It uploads a debug test APK and lint reports for seven days. No secrets, device installation, deployment or production signing. Local tooling remains absent. [Hosted build 39086483179](https://github.com/drluciano/doki/actions/runs/39086483179) passed compile and lint for code revision `7e9ecd51becab0f6d1cc40d4fece274b28cac3d3`, producing a debug APK and lint reports. Subsequent documentation changes do not establish device behavior. This is F01 probe validation, not the full application's G02 pipeline.

Action syntax was checked against official [setup-java](https://github.com/actions/setup-java), [setup-gradle](https://github.com/gradle/actions/blob/main/docs/setup-gradle.md) and [upload-artifact](https://github.com/actions/upload-artifact) documentation. All workflow actions are pinned to resolved release commit SHAs (checkout v7.0.1, setup-java v6.1.0, setup-gradle v6.4.0, upload-artifact v7.0.2).

The UI now applies system-bar/cutout insets on modern Android and scrolls at large text sizes. Rotation destroys and cancels the old attempt; callbacks cannot update a destroyed Activity. Results may be selected again after recreation; background work is not silently resumed. Timeout visibly requests cooperative cancellation, but provider code can ignore it. These behaviors require runtime validation.

## New actionable source evidence

GitHub connector `fetch` resolved [v0.2.1](https://api.github.com/repos/eden-emulator/mirror/git/ref/tags/v0.2.1) directly to commit `58c1e20ee58efa3900ba616207d460886214480b`, matching the official release link. Connector `fetch_file` succeeded for manifest (blob `b899ca07fa0c6cafd1f534e406626b709dd50770`) and provider (blob `f3be156b5efb0cafaa18b48265d2047c773286c0`). This resolves the previous mirror-tag equivalence unknown.

The [manifest](https://github.com/eden-emulator/mirror/blob/58c1e20ee58efa3900ba616207d460886214480b/src/android/app/src/main/AndroidManifest.xml) exposes a URI-granting documents provider. Its [implementation](https://github.com/eden-emulator/mirror/blob/58c1e20ee58efa3900ba616207d460886214480b/src/android/app/src/main/java/org/yuzu/yuzu_emu/features/DocumentProvider.kt) supplies a public-files root, metadata, child enumeration and descriptor access. Writable files advertise write/delete/move/copy/rename; writable directories advertise creation. Creation avoids existing names; move copies then removes, so do not assume atomicity. Child checks use string prefixes rather than a demonstrated canonical containment policy. This warrants separate containment tests, not a confirmed exploit claim. Child enumeration materializes the directory internally; the probe's 20-row client bound cannot bound provider work. Export/import handlers remain uninspected. No Eden code is copied here.

## Behavior and privacy

The user explicitly opens the system folder picker. The probe requests read and persistable flags only. It queries only MIME type and capability flags for at most 20 immediate entries, with one extra cursor step to detect truncation. It never recursively walks, opens file contents, hashes, writes, renames, deletes, imports or sends data. No broad storage or network permission is declared. Names, paths, IDs, URIs and exception text are not displayed or logged by this code. Android/the provider can independently log diagnostics. Persisted URI grants are stored by Android; release them with the provided button or Android settings.

Five-second cancellation is cooperative: a provider that ignores cancellation may run longer. Use only a small disposable fixture folder. Do not select Eden's whole root or personal-save trees. Capability flags are advertisements, not tested write permissions or safety guarantees.

## Build prerequisites and exact plan

As checked during preparation, java, gradle and adb did not resolve on PATH. No SDK/tool installation was performed. Local compilation requires toolchain provisioning; hosted compilation is verified as linked above. No wrapper binary is bundled; do not claim `gradlew` works.

Use JDK 17, Gradle 8.11.1, Android SDK Platform 35 and Build Tools 35.0.0, with pinned Android Gradle Plugin 8.9.2. This is a deliberately chosen compatible baseline, not a latest-toolchain claim. [Official AGP compatibility table](https://developer.android.com/build/releases/agp-8-9-0-release-notes) documents the series requirements. No Kotlin, Flutter or NDK is needed. Initial Gradle dependency resolution needs approved network access; no permission workaround is included.

From this directory in a provisioned shell:

```powershell
java -version
gradle --version
# Set Android SDK location in an ignored local.properties file using sdk.dir.
gradle --no-daemon :app:assembleDebug :app:lintDebug
```

Expected build artifact: `app/build/outputs/apk/debug/app-debug.apk`. The hosted task passed as linked above; no device tests have run. Generate and commit a verified Gradle wrapper/checksum later if this harness becomes a maintained repository artifact. Do not install an APK or interact with the device without the agreed device setup step.

## Meaningful verification checklist

- Confirm merged manifest has no permissions and only the intended activity.
- Run compile and lint; review warnings rather than masking them. Static inspection cannot substitute for compilation.
- On a synthetic provider/disposable fixture, test cancel, empty folder, more than 20 children, denied read, no persistable grant, revoked grant and provider timeout/failure.
- Confirm reports contain counts/booleans only, no filenames, authorities, URI IDs or paths.
- Confirm selected provider is Eden using the system picker; record package variant separately with owner's approval.
- Restart/reboot and verify grant persistence in a later dedicated check; this version reports only currently observed persisted permission, never claims reboot persistence.
- Retain read-only outcome evidence. Successful metadata access is not contents-read validation, coherent checkpoint capture, backup, apply, restore or F01 completion.

API basis: [DocumentsContract](https://developer.android.com/reference/android/provider/DocumentsContract), [ContentResolver persisted grants](https://developer.android.com/reference/android/content/ContentResolver#takePersistableUriPermission(android.net.Uri,%20int)). Legacy Activity result callbacks are intentional to avoid adding AndroidX dependencies to this small harness.

## Version 0.2: remembered-folder recheck

After selecting a small disposable folder and granting access, choose **Recheck remembered folder (no picker)**. The probe checks the stored read grant and repeats the same bounded metadata query without requesting access again. The last successfully persisted tree URI is kept only in private, non-backed-up app preferences; it is not displayed, exported or logged. A transient replacement selection clears the previous recheck target. Release remembered grants removes the saved target on success. No file contents or writes are introduced.

Owner-reported 0.1 result on AYN Odin 3 / Android 15 / Eden 0.2.1: APK installed/opened; Eden visible/selectable through the picker; post-selection result reported persisted grant true, one sampled entry, one directory, no file-write flags advertised and no additional entries. This is user-observed evidence, not an automated device test. A directory without file-write flags does not establish its create/write capabilities.

For 0.2 device validation:
1. Install the debug APK, select the same small disposable test folder, and capture the result without sharing names/paths.
2. Close/reopen the probe and tap Recheck remembered folder. Record whether the metadata query succeeds without a picker.
3. Reboot the device and repeat Recheck; report the observed result. Only this observation can support a reboot-persistence claim.
4. Tap Release remembered grants, then Recheck; expect an explicit no-remembered-folder result and no query. Choose again to restore the test setup.

CI debug signing keys can differ between builds. If Android rejects updating the old probe, uninstall only **Doki read-only probe** and install the new test APK; this resets the probe's grants/preferences, not Eden data. Select the test folder again before persistence checks. No device debugging or Eden save replacement is required.
