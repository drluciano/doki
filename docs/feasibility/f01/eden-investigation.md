# Eden: bounded F01 source investigation

Date: 2026-10-17. Research only; no emulator installed, executed or tested. Owner-reported Android target: Eden 0.2.1 on an AYN Odin 3 running Android 15. These details are not independently verified. Windows installed build/OS, disposable test game and profiles remain unrecorded. This report cannot establish compatibility or complete F01.

## Revision qualification

The repository landing page displayed master head `de7e6e01d43e8cf2ec193d74fa053b0d7949955f`. Its commit page failed through the web reader. Cached pages exposed different revision ages; therefore this is not an assertion that all browsed files form that one snapshot. Android DirectoryInitialization's permalink resolved to `c746982201a60958193f0678fff1f39417aaaf1e`, but the immutable page itself could not be fetched. A candidate v0.2.1 tag URL and the Android manifest source/raw URLs were inaccessible through the web reader; neither the exact release tag nor its provider declarations are verified. Before implementation, inspect all relevant files at the actual installed release/tag or one successfully fetched immutable commit. [Repository](https://git.eden-emu.dev/eden-emu/eden)

## Findings directly observed

1. The handbook's storage page directs desktop users to a game's save directory using the context menu. This is discovery guidance, not a stable API or a documented universal absolute path. [Storage handbook](https://git.eden-emu.dev/eden-emu/eden/src/branch/master/docs/user/Storage.md)
2. Desktop import instructions create an initial save, open the game's save-data location, copy the desired files there, then verify inside the game. The page explicitly allows single-file or multi-file saves. Its Android import section is unfinished. It does not describe transactional installation, verified backups or crash recovery. [Importing saves](https://git.eden-emu.dev/eden-emu/eden/src/branch/master/docs/user/ImportingSaves.md)
3. The synchronization handbook illustrates a desktop hierarchy under `nand/user/save`, containing a user-specific directory and title directories. It advises locating it through Eden rather than assuming a fixed system-wide root. It warns about manual backup, conflict and propagated-deletion risks. The inspected guide contains Windows/Linux workflows, not proof of Android integration. [Synchronization handbook](https://git.eden-emu.dev/eden-emu/eden/src/branch/master/docs/user/SyncthingGuide.md)
4. Android's `DirectoryInitialization` source sets its user path from the application's external-files directory and supplies that path to native initialization. This is app-specific storage initialization; it does not itself demonstrate access by another app. [Observed source](https://git.eden-emu.dev/eden-emu/eden/src/branch/master/src/android/app/src/main/java/org/yuzu/yuzu_emu/utils/DirectoryInitialization.kt). [Unfetched immutable candidate](https://git.eden-emu.dev/eden-emu/eden/src/commit/c746982201a60958193f0678fff1f39417aaaf1e/src/android/app/src/main/java/org/yuzu/yuzu_emu/utils/DirectoryInitialization.kt)
5. The Android utilities listing contains document-tree, file utility and game-metadata components. That listing alone does not establish an exported document provider or cross-app write contract. The Android source directory also has a manifest; its provider configuration was not inspected in this bounded pass. [Android utility listing](https://git.eden-emu.dev/eden-emu/eden/src/branch/master/src/android/app/src/main/java/org/yuzu/yuzu_emu/utils)

No Eden code was copied into Doki. No claim of an Android export/import API, configurable save root or external write permission has been validated.

## Integration hypotheses and risks — untested

- Android is the highest-risk gate: app-specific external storage ordinarily prevents direct cross-app access on modern Android. Eden may expose a document provider or export flow, but this requires inspecting manifest/provider implementation and testing its grants on the selected build. Consult the platform research report for Android source evidence.
- A provider, if present, may support user-driven import/export without durable directory grants, atomic rename or unattended writes. Those capabilities must be measured separately.
- Do not treat the profile directory as the game identity. Preserve and explicitly map user/profile and title identities across devices; never hard-code the handbook's example profile identifier.
- Initial save creation may be necessary for a target directory. Doki should explain this state rather than create guessed emulator internals.
- A complete game save may span files. Restrict initial experiments to stopped-emulator disposable saves until a reliable session boundary is established.
- Desktop path relocation/portable installations and profile selection must be researched before implementing automatic discovery.
- Copying a file successfully does not prove format/version/game compatibility. Verify the disposable result by loading it in the matching game/build and restoring the predecessor.
- Whole-user-data export may include unrelated settings or sensitive material. Prefer the smallest supported save bundle and do not sync the entire emulator data tree by default.

## Next source inspection and real-device evidence

- Pin the installed Eden releases and inspect AndroidManifest provider declarations, any provider permission/grant flags, per-game export/import UI handlers and native save-path construction at those revisions.
- Inspect desktop save-location handler and path/profile resolution; do not assume that the current master implementation matches the user's build.
- Record Android picker visibility, read/write grants, reboot persistence, provider supported operations and revoked-access behavior.
- On both platforms, demonstrate fixture capture, independent verified backup, apply, in-game verification and restore; record before/after hashes and exact build/device versions.
- Test multi-file saves, multiple profiles, missing storage and emulator-active refusal. Do not execute destructive tests against personal progress.

The proposed design handoff is available in access-flows.md. Owner-reported Android target details are recorded above; Windows details and runtime evidence remain pending. This document is suitable as draft research evidence, not an accepted platform-support declaration.
