# F01 / issue #12: platform research

Research date: 2026-10-17. Status: documentation research only. No application implementation, installation, personal-save discovery or device testing performed. Owner-reported Android target is Eden 0.2.1 on an AYN Odin 3 running Android 15. Windows installed build/OS remain unrecorded. The proposed access-flow handoff is in access-flows.md. No emulator version is tested or supported on this evidence alone.

## Evidence and limits

### Android document access — documented, not tested

Android's Storage Access Framework exposes user-selected files/directories through URIs. Directory grants cover the selected subtree. Providers advertise supported operations; support must be checked rather than assumed. Persistable grants can survive restart, but moved/deleted documents can require selection again. Android 11+ restricts picking `Android/data` and `Android/obb` content and restricts directory-tree selection of storage roots and Download. The create-document action does not overwrite an existing same-name file. These facts support investigating selected shared save folders or explicit import/export; they do not prove that any emulator exposes a usable location, or that any provider offers atomic replacement or coherent multi-file capture. [Android shared-document access](https://developer.android.com/training/data-storage/shared/documents-files)

### Android 11 storage changes — documented, not tested

Android 11 restricts access to other apps' private storage, including app-specific external directories. For apps targeting Android 11+, scoped storage is enforced and legacy opt-out no longer applies. Storage permission is not evidence of unrestricted save access. Test actual OS and app target configuration; older behavior must not become the compatibility promise. [Android 11 storage updates](https://developer.android.com/about/versions/11/privacy/storage)

### Windows file access — documented, not tested

`CreateFileW` requests access and sharing modes. Incompatible existing handles can produce a sharing violation; the earlier handle's sharing rules persist until it closes. Delete-sharing affects rename as well as deletion. Reparse-point flags influence whether operations target a link or its destination. Creation disposition can truncate or replace existing data, so read probes must not use destructive modes. Write-through behavior depends partly on hardware. None of this establishes an emulator's lock policy, atomic multi-file replacement, or recovery after power failure. [Microsoft CreateFileW reference](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)

## Proposed investigation implications — inference

- Use a capability matrix per provider/filesystem, not a platform-wide atomicity claim.
- An emulator that saves only in inaccessible private storage may require its own supported export/import operation or a configurable shared save directory.
- A successful read while an emulator runs does not establish a coherent bundle. Measure its write sequence and use a stopped-emulator/session boundary when needed.
- Refusal to write when sharing/permissions prevent a verified backup is preferable to attempting a forced replacement.
- Inspect all bundle paths and reparse/symlink behavior before allowing writes outside a declared test root.

These are investigation hypotheses, not validated implementation guarantees.

## Environment prerequisites and evidence checklist

Current supplied environment inventory: git is available on PATH; adb, Flutter, Dart and Java are not. This report did not install or independently probe them. Record exact versions before testing; tool absence is not proof that a program is absent outside PATH.

- [ ] Owner selects at least two representative emulators per platform, exact versions, save types (in-game save versus save state), and permissible disposable test data.
- [ ] Record Windows edition/build, filesystem and storage type; Android manufacturer/model, OS/API version, emulator package/version and document provider/version.
- [ ] Obtain a dedicated Windows fixture directory and Android test location. Do not scan personal folders or alter a real playthrough.
- [ ] Establish original fixture hashes and an independently retained copy before any write experiment.
- [ ] Agree the F01 design handoff: permission selection, denied/unsupported access, revoked grant, missing media, locked save, and result presentation.
- [ ] Provision the Android SDK/platform-tools and compatible JDK required by the selected research harness; establish the owner's device connection and record authorization without publishing device identifiers.
- [ ] Select a minimal research harness after design approval; do not bootstrap the full Flutter/Rust application merely to perform feasibility research.
- [ ] If Flutter is used, record Flutter/Dart/SDK/toolchain versions and diagnostic output; select the bridge only after measuring integration needs.
- [ ] Record commands, logs, screenshots, fixture hashes and pass/fail outcomes with sensitive paths and identifiers redacted.

## Required experiment matrix — all untested

| Experiment | Windows evidence required | Android evidence required |
|---|---|---|
| Grant/open/read | Selected fixture path, access mode, return/error | Picker result, URI grant flags, provider capabilities |
| Capture bundle | Files/manifest hashes, stopped/running state, write pattern | Same, including provider stream behavior |
| Preserve backup | Independent verified copy before replacement | Verified copy independent of target document |
| Apply/restore | Disposable-fixture hashes before/after, open-handle conditions | Supported create/update/rename/delete behavior and readback |
| Permission loss | Access-denied handling with originals retained | Grant revocation/restart/reselection and honest status |
| Missing storage | Disconnected fixture media; no deletion inference | Removed SD/provider unavailable; no deletion inference |
| Interrupted write | Fault point and recoverable fixture evidence | Provider-specific partial-write and recovery observations |
| Multi-file consistency | No usable mixed bundle; documented session boundary | Same, without timestamp-only coherence claims |
| Unsupported location | Clear refusal and next action | Private-directory restriction and emulator import/export alternative |

Use test-only copies for apply, restore, interruption and cleanup experiments. Do not weaken the later safety contract based on one successful run. F01 completion requires reproducible results for the selected supported setups, a known-limitations matrix and owner review; official documentation alone does not satisfy its acceptance criteria.

## Pending decisions / blockers

Eden is selected and Android target details are owner-reported above. Windows details, fixtures, Android connection, harness and provider capability results remain pending. The access-flow handoff is proposed; application implementation has not begun. This document is research preparation, not an accepted F01 completion or a platform-support declaration.
