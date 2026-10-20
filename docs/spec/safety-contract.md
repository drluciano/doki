# Save-safety and recovery contract

This is the authoritative description of how Doki protects saves. It covers capture, incoming progress, install and rollback, retention and deletion; every state change and what happens if it is interrupted; the difference between integrity and validity; and what each platform does and does not guarantee. Where another document differs, this one wins, and it is updated in the same pull request as the behaviour it describes.

Status: implemented in `crates/` and tested as listed in section 13. The Android folder guarantees in section 9 are implemented but have not yet been measured on real hardware.

Normative words: MUST / MUST NOT / SHOULD as in RFC 2119.

## 1. Terms

| Internal term | User-facing wording | Meaning |
|---|---|---|
| Bundle | (the game's save) | The complete set of files that form one save, e.g. a `.sav` plus a sidecar. Always captured and installed as a unit. |
| Playable files | "the save your emulator uses" | The bundle files in the emulator's save folder. Only the install procedure writes them. |
| Checkpoint | Checkpoint | Immutable record of one bundle's exact bytes, metadata and parent links. |
| Vault | (not shown) | Local content-addressed store plus records. Receiving writes here only. |
| Head | "Current save" | Per device and bundle: the checkpoint this device's playable files match. |
| Tip / set-aside checkpoint | Separate progress | A continuation no later checkpoint builds on, or one a choice set aside. |
| Proposal | Incoming progress | Another device's checkpoint offered for review on this device. |
| Choice checkpoint | "You chose…" | Checkpoint recording a decision between continuations (3.3). |
| Install | Use this save / Restore a checkpoint | Making the playable files equal a checkpoint. |
| Journal | (not shown) | Durable record of an install in progress. |

## 2. Global invariants

- **I1 Immutability.** A checkpoint's id is the SHA-256 of its canonical JSON. Once stored, its record and referenced objects are never modified. Corrections create new checkpoints.
- **I2 Delivery ≠ activation.** No receive, sync, retry or background task writes playable files. Only an install started by an explicit user action bound to a reviewed checkpoint does.
- **I3 Backup before replacement.** No playable file is overwritten, created or removed until the current playable files equal a stored checkpoint whose bytes re-verify from the vault.
- **I4 Ancestry decides.** Relations between checkpoints (same, ahead, behind, divergent, unrelated, unknown) come only from parent links. Timestamps, sizes and modification times are descriptive and never select a winner.
- **I5 Nothing disappears by implication.** Missing folders, revoked permission, removed storage, empty folders and absent remote data are unavailability, never deletion and never an "empty save" checkpoint.
- **I6 Preserve both.** Choosing one continuation never deletes another; the other stays reachable in history.
- **I7 Approval is bound.** Using a checkpoint is bound to the head and the exact playable bytes at review time. Any change voids the approval.
- **I8 Visible only when complete.** A partial capture, download or install is never shown or selectable as a checkpoint.
- **I9 Integrity ≠ validity.** A hash match proves bytes are unchanged, not that a game can load them (section 8).
- **I10 One writer per bundle.** At most one capture, review or install per bundle at a time, enforced by an OS lock that is released if the process dies.
- **I11 Honest errors.** Every failure says what happened, whether playable files changed, what is preserved and what to do next.

## 3. Data model

### 3.1 Checkpoint (schema 1)

```
schema_version      1; other versions are refused, not guessed at
bundle_id           stable game identity, shared between devices by explicit linking
parents             checkpoint ids; [] only for a bundle's first capture on a device
kind                capture | backup | choice
files[]             { path, size, hash }, strictly sorted by path
profile             { id, version } of the emulator profile that defined the bundle
origin_device       device id
origin_device_name  device name at capture time, for display
captured_at         device clock, RFC 3339, descriptive only (I4)
capture_mode        emulator-stopped-confirmed | integration-attested | quiescence-heuristic | automatic
note?               user or system text
```

Field order is part of the identity; new fields go at the end with a schema bump. Paths are normalized to Unicode NFC with `/` separators and rejected if they are absolute, contain `.`/`..`/empty segments, use characters or names Windows cannot store, collide case-insensitively, or are both a file and a folder. `.doki-staging` is reserved. Limits: 4096 files per bundle, 512 bytes per path.

### 3.2 Device state per bundle

`BundleRecord { bundle_id, name, location, profile, head, pins }` and at most one open `Journal`. Head changes only on capture, on a committed install, or when adopting a checkpoint with identical bytes and more history (5.2).

### 3.3 Recording a choice

When the user uses a checkpoint that does not descend from the current head (divergent, unrelated, unknown, or an older checkpoint being restored), the commit writes a **choice checkpoint**: `parents = [installed, previous head]`, files equal to the installed checkpoint (no extra storage). "Dismiss" writes `parents = [current head, dismissed]` with the current head's files and leaves playable files untouched.

Because the decision is part of ancestry, every other device sees the set-aside checkpoint as an ancestor of the new head and does not raise the same conflict again. If the other device's current save has the same bytes, it simply adopts the choice as its head without a review.

## 4. Capture

Purpose: copy the playable bundle into the vault. Never writes playable files.

| Step | What happens | Interrupted here → outcome |
|---|---|---|
| C1 Preflight | Bundle lock taken; no open install journal allowed (`RecoveryRequired`) | Nothing written. |
| C2 Source check | Folder reachable? Missing → `SourceUnavailable`; denied → `PermissionLost` | Nothing written. |
| C3 Copy | Each file streamed into `tmp/`, hashed, flushed, renamed into `objects/` | Orphan objects only; invisible (I8), removed by `clean`. |
| C4 Confirm stable | Folder re-listed and re-hashed; any difference → `SourceChanging` | As C3. No checkpoint. |
| C5 Store | Checkpoint written atomically after all its objects exist | Either stored or not. |
| C6 Move head | Bundle record replaced atomically | If C5 happened but not C6, the next capture of the same bytes adopts that checkpoint instead of creating a twin. |

Rules: identical bytes to head → "No changes since …", nothing stored. Empty folder → `BundleIncomplete`. `capture_mode` records the actual certainty: `emulator-stopped-confirmed` when the user confirmed the game is closed, `quiescence-heuristic` for automatic checkpoints.

**Automatic checkpoints** (`Engine::auto_check`, on per game by default): the folder is hashed on every look (every 15 seconds in the app and the Android watcher). A state different from head is remembered with the time it was first seen; any further change restarts the wait. Only when two looks at least the settle time apart (30 seconds) see the same bytes does it run the capture above, whose C4 re-read still refuses a folder that moves during the copy (then the wait starts over). A folder that cannot be read, a held lock or an open journal skips that game for that look; it is never an error and never a write. Automatic checkpoints are a heuristic: a game that writes its save in pieces more than the settle time apart can be captured between pieces, which is why every checkpoint stays in history and nothing is ever installed without review.

## 5. Receive and proposals

### 5.1 Receiving a handoff package

1. Read and bound the index (≤ 64 MiB, ≤ 100 000 entries, ≤ 16 GiB declared).
2. Validate every checkpoint (3.1) and confirm every referenced object is in the package or already in the vault, before storing anything.
3. Stream each object into the vault, verifying hash and size; reject trailing data.
4. Store checkpoints, then record offers.

Any failure in 1–3 leaves no checkpoint and no proposal (I8). Objects already stored by a failed import are unreferenced and removed by `clean`.

### 5.2 Classifying an offer against this device

| Relation of incoming to head | Result |
|---|---|
| Same, or Behind (already in history) | Nothing to review. |
| Ahead with identical bytes | Head adopts it silently (no files change). |
| Ahead | Incoming progress: "Progress from your PC is ready." |
| Divergent | Incoming progress: "You played on both devices." The nearest shared checkpoint is shown. |
| Unrelated | Incoming progress: "These saves do not share history." Never treated as Ahead. |
| Unknown (history missing) | Incoming progress marked unknown; never guessed. |
| Game not set up here | Kept as "not set up here" until the user links it explicitly. Doki never matches games by name or path. |

A newer checkpoint that descends from a pending proposal supersedes it. Re-delivery of the same checkpoint is a no-op.

### 5.3 Proposal outcomes

```
Reviewable ── Use this save ──▶ Applied (install, section 6)
     ├────── Keep alongside ──▶ Parked (no prompt; in history; can be reopened)
     ├────── Dismiss ─────────▶ Dismissed (choice checkpoint keeps the local save)
     └─ newer descendant arrives ▶ Superseded
```

Keep alongside records nothing in ancestry, so the other device may offer the same divergence again later; Dismiss records the decision so it never re-appears.

### 5.4 Review

Review re-verifies the incoming checkpoint's stored bytes. If the playable files differ from head, it first captures them (mode `quiescence-heuristic`, with a note) so the comparison and the backup include that progress. The review result records the head and the playable bytes it saw (I7).

## 6. Install (Use this save / Restore a checkpoint)

| Step | Persisted | Playable files touched | Interrupted here → outcome |
|---|---|---|---|
| A1 Recheck: head and playable bytes equal the review; else capture the change and return `LocalChangedSinceReview` | – | no | Nothing changed. |
| A2 Backup: playable files equal head; head's objects re-verify; else `BackupUnverified` | – | no | Nothing changed. |
| A3 Journal `Prepared`: op id, target, backup, previous head, intended choice | journal | no | Next operation discards the staging and journal automatically. |
| A4 Stage every differing file in the adapter's staging area (`<folder>/.doki-staging/<op>/` locally, `.doki-<op>-<name>` siblings on Android), verify staged bytes | staging | no | As A3. |
| A5 Journal `Installing` | journal | no | Recovery (6.1). |
| A6 Replace/create each differing file (staged rename), then remove files the target lacks | – | **yes** | Recovery (6.1). Game marked "Needs recovery"; capture and other installs refused. |
| A7 Journal `Committing`; re-read folder, must equal target exactly | journal | no | Recovery (6.1). |
| A8 Commit: store the choice checkpoint if any, move head, mark proposal Applied, discard staging, delete journal | records | no | Recovery sees the folder equals target and repeats A8 (idempotent). |

An I/O error before A5 cleans up and reports the cause; nothing changed. An I/O error from A5 on returns `InstallInterrupted` and leaves the journal. Head never moves before A7 verifies.

Each adapter reports how it replaces a file (`ReplaceMode`): `Atomic` (rename over), `DeleteThenRename` (remove the old file, then move the verified staged copy into place) or `Unknown`. Installs are refused with `StrategyUnsupported` unless the mode can never leave a partially written file; both known modes qualify. Under `DeleteThenRename` an interruption can leave a file missing between the two steps; recovery classifies it as `missing`, and both the target and the backup copy are in the vault, so Finish and Put back restore it. A multi-file bundle can be mixed between A6 steps; the journal makes that state identifiable and recoverable.

### 6.1 Recovery after an interrupted install

1. Classify every path: matches **target**, matches **backup**, **missing**, or **other** (something else wrote, e.g. the emulator was opened).
2. If the folder already equals the journal's target, commit (A8), whatever the user picked.
3. If any path is **other**, capture the whole folder as a `backup` checkpoint first, so nothing is lost.
4. **Finish**: make the folder equal the target, verify, commit. **Put back**: make the folder equal the backup (or empty, if it was empty), verify, restore the previous head.
5. Both are the same idempotent procedure, so recovery can be interrupted and resumed.
6. If recovery cannot complete, the journal stays, the game stays blocked, and the user can copy any checkpoint out with `export-files`.

Doki never resolves an interrupted install on its own; the user chooses Finish or Put back.

## 7. Retention, cleanup and deletion

- Pinned checkpoints are never removed by retention and cannot be deleted by hand until unpinned.
- Checkpoints are kept forever by default. Nothing is deleted automatically unless the user picks a retention time. `clean` removes only temporary files and objects no checkpoint references.
- **Deleting a checkpoint** (`Engine::delete_checkpoint`) and **retention** (`Engine::apply_retention`, a time the user chose under "Keep checkpoints for") remove only a checkpoint's *files* on this device. The checkpoint record (parents, file list) stays, so ancestry keeps deciding how other checkpoints relate; the checkpoint is marked as having its files removed and can no longer be used or restored here. Packages carry such checkpoints as history only (`without_files`), and a receiver that lacks their files marks them the same way, so a later handoff still fast-forwards instead of looking like a conflict.
  - Deleting by hand is refused for the current save, pinned checkpoints and incoming progress still waiting for a decision.
  - Retention runs under the vault's exclusive cleanup lock and every bundle lock, previews what goes before the user confirms a shorter time, and always keeps per game: the 10 newest checkpoints by ancestry, the current save, pins, history tips, set-aside progress and choices, pending proposals and anything an install journal refers to. Capture times only decide what counts as old, never which save wins. It runs after the user confirms, and after automatic checkpoints, never because a folder is missing, another device is absent or the disk is full.
- **Deleting a removed game** (`Engine::forget_game`) is only for games not set up here. It deletes the game's checkpoints, their files and any proposals on this device, and records the game as forgotten so offers and announcements from other devices are ignored, until `unforget`. The save folder is never touched.
- **Removing a game** (`Engine::remove_bundle`) only removes its setup on this device: the save folder is untouched, every checkpoint stays in the vault, reviewable offers are dropped (they come back with the next sync) and the game is listed for setup again, which brings its history back. Refused while an install waits for recovery. Editing a game (name, folder, automatic checkpoints) never reads or writes save files; after a folder change the next capture records the new folder's contents as a child of head.
- Deleting is local to the device by default. Deleting everywhere will be an explicit, reviewed request; no device acts on it automatically.

## 8. Integrity versus validity

| Level | Evidence | Claim allowed |
|---|---|---|
| Bytes verified | Every file's SHA-256 and size match | "Verified copy" |
| Bundle complete | Files the profile requires are present, none outside the folder | "Complete save files" |
| Format checked | A format inspector parsed it (not built yet) | "Save format looks valid" |
| Load observed | User or integration confirmed the game loaded it | "Loaded on <device>" |
| Unknown | Nothing beyond bytes | No validity claim (default for binary saves) |

Doki never infers "more progress" from size, time or hash. A compatibility-matrix entry needs "Load observed" on a named emulator, version and device.

## 9. Platform guarantees and limits

| Concern | Desktop local folder (`LocalDir`) | Android Storage Access Framework |
|---|---|---|
| Missing folder | `SourceUnavailable` | Adapter throws `FileNotFoundException`, mapped to `SourceUnavailable`; revoked access throws `SecurityException`, mapped to `PermissionLost`. Not yet measured on hardware. |
| Per-file atomic replace | Staged file renamed over the target on the same volume (`MoveFileExW` with replace on Windows, `rename(2)` on POSIX). Not established for network shares or FAT removable media. | No rename-over. Adapter uses `DeleteThenRename` (delete the target, then `renameDocument` the verified staged sibling) only when the provider advertises delete, rename and create; otherwise `Unknown` and installs are refused. Behaviour on real providers is not yet measured. |
| Truncating overwrite | Not used | Not used: staged copies are always new documents, never overwritten in place |
| Multi-file atomicity | Not provided; journaled | Not provided; journaled |
| Durability | `fsync` of files and, on POSIX, directories | Not tested |
| Links | Refused (could escape the granted folder) | n/a |
| Emulator writing during capture | Detected only as a changed re-read | Not tested |
| Permission persistence | n/a | Persisted URI grants; survival after reboot must be measured |

No platform claim is made beyond this table. Supported emulators are listed only after an end-to-end restore on real hardware.

## 10. Transport

- **Handoff package file**: one file the user moves themselves, carrying checkpoints, objects and offers (format in `crates/doki-engine/src/package.rs`). Not encrypted; for trusted channels only.
- **Device to device** (`crates/doki-peer`, see [peer.md](../peer.md)): devices connect directly over QUIC (iroh), authenticated by per-device Ed25519 keys and encrypted end to end. A device accepts sessions only from devices in its trusted book; pairing needs a code open for 15 minutes, proven over the encrypted connection, and 20 wrong attempts close every open code. Removals spread through the group and are never undone by another device's book; only a newer pairing of that device brings it back. A session sends handoff packages (above) and imports them as in 5.1: vault only, never a save folder. Messages are capped at 16 MiB and packages at 16 GiB.
- **Relay** (`crates/doki-relay`, client in `crates/doki-sync`): stores the same packages, sealed on the device with XChaCha20-Poly1305 under a key derived from the pairing secret, so the relay never sees save bytes or names. It never decides which save wins, never deletes on a device's behalf, and expires packages after its retention period; expiry is not deletion. Receiving through the relay is the same import as a file (5.1): vault only.
- Interim limit: paired devices share one secret, so revoking one device means re-pairing the rest (see [relay.md](../relay.md)). Per-device relay keys are required before the relay is recommended for use over the internet.
- Sync is not an independent backup.

## 11. Error identifiers

| Code | Playable files changed? | Next action |
|---|---|---|
| `SourceUnavailable` | No | Reconnect the folder or storage |
| `PermissionLost` | No | Allow folder access again |
| `SourceChanging` | No | Close the game and retry |
| `BundleIncomplete` | No | Check the folder holds the save |
| `VaultFull` / `SaveLocationFull` | No | Free space |
| `IntegrityFailed` | No | Receive again; report if repeated |
| `IncompatibleSave` | No | Explained mismatch; no Use action |
| `LocalChangedSinceReview` | No | Review again; newer progress was captured |
| `BackupUnverified` | No | Retry; install blocked |
| `StrategyUnsupported` | No | Use an export/import route |
| `InstallInterrupted` | **Maybe (mixed)** | Recover (6.1) |
| `RecoveryRequired` | **Maybe (mixed)** | Recover before anything else |
| `RecoveryFailed` | **Maybe (mixed)** | Keep blocked; export checkpoint files; retry |
| `Busy` | No | Wait for the other operation |
| `NotFound`, `InvalidInput`, `UnsupportedVersion`, `Io` | No | Shown with details |

## 12. Required tests

- Every step in sections 4, 5.1 and 6 has an interruption test asserting the outcome in its table.
- Ancestry is independent of timestamps; choice checkpoints prevent re-raised conflicts; cleanup never removes checkpoint content.
- Brief fault list: concurrent edits, partial writes, disk full, permission loss, missing storage, clock skew, interrupted installation.
- End to end: play on both devices, detect divergence, preserve both, choose one, restore the other.

## 13. Implementation map

| Area | Code | Tests |
|---|---|---|
| Model, paths, ids, ancestry, errors | `crates/doki-core` | unit tests per module |
| Vault (objects, checkpoints, records, locks) | `crates/doki-vault` | `src/tests.rs` |
| Save-folder adapter and fault injection | `crates/doki-platform` | `local.rs` tests; engine `split_replace` recovery tests |
| JSON command interface | `crates/doki-api` | `src/tests.rs`, including tampered reviews |
| Desktop app bridge (C ABI) | `crates/doki-ffi` | unit test; `app/test/ffi_backend_test.dart` drives the real engine from Dart |
| App | `app/` | widget tests: review never installs, the review is passed back unchanged, errors state whether the save changed |
| Java bridge (JNI) | `crates/doki-jni`, `android/bridge` | `tools/jni-desktop-test.sh`: restore through both replace modes, refusal on unknown mode, missing folder |
| Capture, review, install, recovery, packages | `crates/doki-engine` | `src/tests.rs`: crash at every install step with Finish and Put back, I/O failure at every folder write, crash during recovery, rescue of files written mid-recovery, tampered and truncated packages |
| Device-to-device sync | `crates/doki-peer` | `src/tests.rs`: trust spreads through pairing, both-played divergence, only missing checkpoints are sent, removal shuts a device out everywhere, a removed device paired again, bad, expired and wrong short codes refused, identity survives restart |
| Relay server | `crates/doki-relay` | auth, listing, expiry, size limits |
| Relay client, pairing, encryption | `crates/doki-sync` | wrong key, tampering, relay stores only ciphertext, two-device handoff |
| Command line | `crates/doki-cli` | `tests/handoff.rs`: the end-to-end milestone, by file and by relay |

Not yet covered: disk-full and permission-loss injection on the vault side, the Android adapter on real hardware, peer sync between real Android and Windows devices, the Linux Flatpak on real hardware (it uses `LocalDir` on host folders bind-mounted into the sandbox, not the document portal, so the desktop column applies; SD cards formatted exFAT or FAT fall under its removable-media limit), and per-device relay keys. Deleting and retention are covered by engine tests (files removed, records kept, ancestry intact across a handoff).
