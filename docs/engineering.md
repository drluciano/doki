# Engineering overview

How Doki is put together and the rules every change follows. The step-by-step behaviour, error codes and test list are in the [safety contract](spec/safety-contract.md), which wins wherever this page is less precise.

## Modules

| Module | Role |
|---|---|
| `crates/doki-core` | Domain types: checkpoints and their ids, bundle path rules, ancestry relations, error codes. Pure; no I/O. |
| `crates/doki-vault` | The local store: content-addressed objects, immutable checkpoint records, atomically replaced per-game records, OS locks. JSON files are the source of truth. |
| `crates/doki-platform` | `SaveLocation`, the interface for reading, staging and replacing save files, with `ReplaceMode` capability reporting and fault injection for tests. Includes the local-folder adapter and `MemberLocation` (one game inside a whole-system folder). |
| `crates/doki-engine` | Capture, automatic checkpoints, whole-system discovery, incoming progress and review, the journaled install with verified backup, recovery, deletion and retention, handoff packages. No network code. |
| `crates/doki-peer` | Device-to-device sync with no host: per-device Ed25519 keys, a trusted device book shared between devices, pairing codes, LAN discovery, encrypted QUIC sessions through iroh. The default transport. |
| `crates/doki-relay`, `crates/doki-sync` | The optional self-hostable store-and-forward relay and its client (pairing codes, XChaCha20-Poly1305 sealing). |
| `crates/doki-cli` | The `doki` command. |
| `crates/doki-api` | One JSON request per action; the only interface non-Rust front ends use. |
| `crates/doki-ffi` | C ABI over `doki-api` for the desktop app (Dart FFI). |
| `crates/doki-jni`, `android/bridge` | JNI bridge for Android. Java supplies the save folder through the Storage Access Framework (`SafSaveFolder`), which implements the same `SaveLocation` contract. |
| `app/` | The Flutter app. It holds no safety logic: every action is one engine request. See [app.md](app.md). |

Errors cross every boundary as the stable codes in section 11 of the safety contract, so every front end can say whether the save changed.

## Safety rules

These hold for every change. The safety contract gives the precise version of each.

1. Receiving changes Doki's vault only. A save folder changes only when the user chooses **Use this save** or **Restore**, or resolves a recovery.
2. Every replacement first verifies a recoverable copy of the current save. If that backup cannot be confirmed, nothing is replaced.
3. Checkpoints are immutable and record their parents, file hashes, origin device and profile. Times are descriptive; ancestry alone decides how saves relate.
4. Choosing one continuation never deletes another.
5. Immediately before replacing, the save folder is re-read and compared with what was reviewed. Any change is captured and the review starts over.
6. A missing folder, lost permission, removed storage or empty folder is never treated as a deletion.
7. A save is a bundle of files, captured and installed as a unit. Automatic checkpoints wait for a folder to settle, which is a heuristic, so nothing is ever installed without review.
8. A matching hash proves integrity, not that a game can load the save.
9. Deleting and retention never remove the current save, pinned checkpoints, pending incoming progress or anything an install journal refers to, and never run because something is missing.
10. Every error says what happened, what is preserved and what to do, without leaking save contents or secrets.

Any code that writes playable files goes through the journaled install in `crates/doki-engine/src/install.rs`: recheck, verify backup, journal, stage and verify, mark installing, replace, verify the folder, commit. Head never moves before the installed files verify, and recovery finishes or puts back the backup with the same idempotent procedure. Comments on safety-critical code explain why the order matters.

## Testing

- Unit and state-machine tests per crate; ancestry and integrity tests independent of clocks.
- Fault injection: a simulated crash at every install and recovery step, an I/O failure at every folder write, tampered and truncated packages, oversized inputs, concurrent operations.
- End-to-end tests assert that the original save is preserved and restorable, not only that a transfer succeeded (`crates/doki-cli/tests/handoff.rs`, `crates/doki-peer/src/tests.rs`).
- The app is tested with widgets against a fake engine and against the real engine through FFI; the Java bridge runs on a desktop JVM (`tools/jni-desktop-test.sh`); CI checks the release APK keeps the bridge names the engine calls.
- Fixtures are synthetic. No real saves, ROMs, tokens or identifying diagnostics go in the repository.

CI (`.github/workflows/`) runs formatting, Clippy, tests on Linux and Windows, the app tests, the Android build and the wiki link check.

## Platform support

A platform or emulator is called supported only after a full capture, transfer, install and restore has been done on real hardware with that exact emulator, version and device. Until then it is experimental.

| Platform | State |
|---|---|
| Windows | Built and tested in CI; runs from the tray. |
| Android | Built in CI. The folder adapter uses the Storage Access Framework and replaces files by delete-then-rename. Its behaviour on real hardware and emulator storage (permission persistence, removable storage, background limits) still has to be measured. |
| Linux | Used for development. |
| macOS, iOS | Not supported. Would need their own adapter and storage research first. |

## Releases

There are no published releases yet; CI produces test builds. Before a first release: a stable Android signing key so updates install in place ([#82](https://github.com/drluciano/doki/issues/82)), per-device relay keys, release notes with known limits, and a supported-platform list backed by device testing.

## Documentation

Document public interfaces, error codes, schemas, configuration and recovery steps, and update them in the same pull request as the behaviour. Player-facing help lives in the [wiki](wiki/Home.md) and in the app's Help page (`app/lib/screens/help.dart`); keep both in step with button wording.
