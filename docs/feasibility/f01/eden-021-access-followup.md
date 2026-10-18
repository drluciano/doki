# F01: Eden 0.2.1 Android access follow-up

Target: owner-reported AYN Odin3, Android 15, Eden 0.2.1. No physical-device evidence collected. This remains the same unfinished F01 story; no installation, real-save access, application implementation or GitHub write occurred.

## Release pin and evidence boundaries

The [official v0.2.1 release](https://git.eden-emu.dev/eden-emu/eden/releases/tag/v0.2.1) links commit **58c1e20ee58efa3900ba616207d460886214480b**. The release has multiple Android package variants; the owner's installed package/variant has not been independently identified. The commit page returned HTTP 403. Release pin is established by the official release hyperlink, not a locally verified Git object.

Official Forgejo tag HTML, raw-tag source and contents API routes were inaccessible through the web reader. A materially different route succeeded: the Eden GitHub mirror's v0.2.1 manifest. The mirror tag source is evidence at that named tag; its tag-to-commit equivalence has not been independently verified against the official SHA. Do not label the mirror findings cryptographically verified release contents.

## New observed source evidence

The [v0.2.1 mirror manifest](https://github.com/eden-emulator/mirror/blob/v0.2.1/src/android/app/src/main/AndroidManifest.xml) declares a DocumentProvider component with an application-ID-derived user authority, exported access, URI grants and Android's document-management permission. It advertises the documents-provider action. A separate FileProvider is not exported and grants URIs. This supports a **testable system-picker route**, not direct unrestricted private-directory access. Build substitution means the installed authority must be measured rather than guessed. The manifest alone says nothing about per-document write flags, persistent grants, supported save subtrees, coherent snapshots or atomic replacement.

The [same-tag features directory](https://github.com/eden-emulator/mirror/tree/v0.2.1/src/android/app/src/main/java/org/yuzu/yuzu_emu/features) lists DocumentProvider.kt. Fetches of its HTML and raw content failed in this bounded pass. No provider implementation or export/import handler was successfully inspected. Accordingly, root enumeration, create/update/delete/rename semantics, path containment and any import/export overwrite behavior remain **unknown**. Do not infer these capabilities from class names or upstream ancestry.

## Retrieval record

| Route | Result |
|---|---|
| Official release HTML | Success; release-to-commit link |
| Official commit HTML | HTTP 403 |
| Official tag source / raw-tag manifest | Inaccessible through reader |
| Official repository contents / git refs APIs | Inaccessible through reader |
| GitHub mirror tag manifest HTML | Success; provider declarations observed |
| Mirror immutable raw manifest | Cache miss |
| Mirror tag features directory | Success; implementation filename observed |
| Mirror provider HTML / raw-tag implementation | Failed / cache miss |

Further retrieval should use a verified same-commit GitHub connector response or a permitted minimal Git fetch and compare the official commit before expanding source claims. Repeating the identical web URLs is unlikely to add evidence.

## Minimal probe plan — proposed, not executed

1. Record installed version/build/package variant and Android build; do not include device serials in public evidence. Record whether the installed manifest exposes the same provider contract.
2. With the emulator stopped, launch the Android system document picker from a dedicated probe app and inspect whether Eden appears. Select only a disposable fixture subtree, never broad personal user data.
3. Record granted flags, provider root/document IDs (redacted), read/write capability flags, ability to enumerate the chosen fixture and content hashes on read. Stop after read-only evidence first.
4. In a separately approved disposable-write test, create/read/update/rename/delete one sentinel file inside the fixture subtree. Check advertised flags against results; verify no traversal outside the selected root.
5. Test grant persistence after app/device restart and loss after revocation. Document picker/provider refusal honestly. Access success does not establish safe apply or coherent multi-file capture.
6. Only after provider implementation and overwrite behavior are understood, plan backup/apply/restore experiments. If picker access is unavailable, investigate Eden-owned explicit export/import as a manual fallback; those flows remain unverified.

Required tooling to execute an instrumented probe: compatible JDK, Android SDK/platform-tools (adb), build tools/Gradle and an authorized device connection, plus a reviewed minimal probe APK. Existing prior inventory had git but no adb/Flutter/Dart/Java on PATH; this turn did not recheck or install anything. Flutter is unnecessary for the first native access probe. Installing an APK or enabling device debugging requires a concrete owner-approved device setup step. A manual picker observation may provide early evidence but cannot prove another app's persistent write access.

## Conclusion

A DocumentsProvider declaration is new evidence for a plausible sanctioned cross-app route. Its implementation, package identity, mirror revision equivalence and Odin3 behavior still need verification. F01 is neither complete nor ready for acceptance based on this report.
