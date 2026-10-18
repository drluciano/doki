# Save-safety and recovery contract

Status: **implemented for local folders** in `crates/` (see section 13). Android provider guarantees (section 9) still need device evidence from F01 [#12](https://github.com/drluciano/doki/issues/12). Choices that are defaults rather than owner decisions are marked **[default]** and listed in [open-questions.md](open-questions.md).

This contract is the normative version of the rules in the owner's brief and in [engineering.md](../engineering.md). It covers F02 [#13](https://github.com/drluciano/doki/issues/13): capture, proposal, apply, rollback, retention and deletion invariants; every state transition and its outcome after interruption; integrity versus validity; and platform limits.

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

Rules: identical bytes to head → "No changes since …", nothing stored. Empty folder → `BundleIncomplete`. `capture_mode` records the actual certainty; the MVP asks the user to confirm the game is closed.

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

**[default]** Keep alongside records nothing in ancestry, so the other device may offer the same divergence again later; Dismiss records the decision so it never re-appears.

### 5.4 Review

Review re-verifies the incoming checkpoint's stored bytes. If the playable files differ from head, it first captures them (mode `quiescence-heuristic`, with a note) so the comparison and the backup include that progress. The review result records the head and the playable bytes it saw (I7).

## 6. Install (Use this save / Restore a checkpoint)

| Step | Persisted | Playable files touched | Interrupted here → outcome |
|---|---|---|---|
| A1 Recheck: head and playable bytes equal the review; else capture the change and return `LocalChangedSinceReview` | – | no | Nothing changed. |
| A2 Backup: playable files equal head; head's objects re-verify; else `BackupUnverified` | – | no | Nothing changed. |
| A3 Journal `Prepared`: op id, target, backup, previous head, intended choice | journal | no | Next operation discards the staging and journal automatically. |
| A4 Stage every differing file in `<folder>/.doki-staging/<op>/`, verify staged bytes | staging | no | As A3. |
| A5 Journal `Installing` | journal | no | Recovery (6.1). |
| A6 Replace/create each differing file (staged rename), then remove files the target lacks | – | **yes** | Recovery (6.1). Game marked "Needs recovery"; capture and other installs refused. |
| A7 Journal `Committing`; re-read folder, must equal target exactly | journal | no | Recovery (6.1). |
| A8 Commit: store the choice checkpoint if any, move head, mark proposal Applied, discard staging, delete journal | records | no | Recovery sees the folder equals target and repeats A8 (idempotent). |

An I/O error before A5 cleans up and reports the cause; nothing changed. An I/O error from A5 on returns `InstallInterrupted` and leaves the journal. Head never moves before A7 verifies.

The local-folder adapter requires per-file atomic replacement (`per_file_atomic_replace = Supported`) and refuses to install otherwise (`StrategyUnsupported`). A multi-file bundle can be mixed between A6 steps; the journal makes that state identifiable and recoverable.

### 6.1 Recovery after an interrupted install

1. Classify every path: matches **target**, matches **backup**, or **other** (something else wrote, e.g. the emulator was opened).
2. If the folder already equals the journal's target, commit (A8), whatever the user picked.
3. If any path is **other**, capture the whole folder as a `backup` checkpoint first, so nothing is lost.
4. **Finish**: make the folder equal the target, verify, commit. **Put back**: make the folder equal the backup (or empty, if it was empty), verify, restore the previous head.
5. Both are the same idempotent procedure, so recovery can be interrupted and resumed.
6. If recovery cannot complete, the journal stays, the game stays blocked, and the user can copy any checkpoint out with `export-files`.

**[default]** Doki never resolves an interrupted install on its own; the user chooses Finish or Put back.

## 7. Retention, cleanup and deletion

- Pinned checkpoints are protected from any future cleanup.
- **[default]** There is no automatic deletion of checkpoints in the MVP. `clean` removes only temporary files and objects no checkpoint references.
- Checkpoint cleanup (C06) must: compute roots (pins, heads, tips, set-aside checkpoints, open journals, pending proposals, in-flight transfers) under the bundle locks; preview what goes; keep ancestry metadata; and never be triggered by missing folders, remote absence or a full disk.
- Deleting is local to the device by default. Deleting everywhere will be an explicit, reviewed request; no device acts on it automatically.

## 8. Integrity versus validity

| Level | Evidence | Claim allowed |
|---|---|---|
| Bytes verified | Every file's SHA-256 and size match | "Verified copy" |
| Bundle complete | Files the profile requires are present, none outside the folder | "Complete save files" |
| Format checked | A format inspector parsed it (future, X05) | "Save format looks valid" |
| Load observed | User or integration confirmed the game loaded it | "Loaded on <device>" |
| Unknown | Nothing beyond bytes | No validity claim (default for binary saves) |

Doki never infers "more progress" from size, time or hash. A compatibility-matrix entry needs "Load observed" on a named emulator, version and device.

## 9. Platform guarantees and limits

| Concern | Desktop local folder (`LocalDir`) | Android Storage Access Framework |
|---|---|---|
| Missing folder | `SourceUnavailable` | Not tested |
| Per-file atomic replace | Staged file renamed over the target on the same volume (`MoveFileExW` with replace on Windows, `rename(2)` on POSIX). Not established for network shares or FAT removable media. | Unknown; many providers lack rename-over. Adapter must report Unknown until F01 measures it. |
| Truncating overwrite | Not used | Known hazard: some providers ignore `"wt"`; must be measured |
| Multi-file atomicity | Not provided; journaled | Not provided; journaled |
| Durability | `fsync` of files and, on POSIX, directories | Not tested |
| Links | Refused (could escape the granted folder) | n/a |
| Emulator writing during capture | Detected only as a changed re-read | Not tested |
| Permission persistence | n/a | Persisted URI grants; survival after reboot must be measured |

No platform claim is made beyond this table. Supported emulators are listed only after an end-to-end restore on real hardware.

## 10. Transport

- The first transport is a **handoff package** file the user moves themselves (format in `crates/doki-engine/src/package.rs`). It carries checkpoints, objects and offers; it is not yet encrypted or signed, so it must not travel over untrusted channels.
- The self-hostable relay is next. It stores encrypted objects and signed metadata, never decides which save wins and never deletes on a device's behalf. Pairing, encryption and revocation follow S01 before any internet-facing use.
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
| Save-folder adapter and fault injection | `crates/doki-platform` | `local.rs` tests |
| Capture, review, install, recovery, packages | `crates/doki-engine` | `src/tests.rs`: crash at every install step with Finish and Put back, I/O failure at every folder write, crash during recovery, rescue of files written mid-recovery, tampered and truncated packages |
| Command line | `crates/doki-cli` | `tests/handoff.rs`: the end-to-end milestone |

Not yet covered: disk-full and permission-loss injection on the vault side, the Android adapter, the relay, and checkpoint cleanup (C06).
