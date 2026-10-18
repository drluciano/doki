# F01 / issue #12 — save-access feasibility test protocol

Status: **NOT EXECUTED; story incomplete. No emulator has been tested.** The owner selected [Eden](https://git.eden-emu.dev/eden-emu/eden) as the first target for Windows (W1) and Android (A1). The owner reports Eden 0.2.1 on an AYN Odin 3 running Android 15. These details are not independently verified. Windows build/OS and save locations remain pending. Second emulator targets (W2/A2) remain unselected. This is a test protocol, not evidence of supported access.

## Acceptance criteria and exact mapping

| ID | Acceptance criterion | Required evidence | Pass gate |
| --- | --- | --- | --- |
| AC1 | Test at least two representative emulators per platform and record versions and locations. | Four completed setup records minimum: Windows W1/W2 and Android A1/A2; emulator identity/version proof, OS/device details, tested locations and access method. | Both emulator setups on each platform tested; unselected, unavailable, and not-run cells remain incomplete. |
| AC2 | Demonstrate read, capture, backup, apply and restore on each supported setup. | Operation log, complete bundle manifests/hashes, verified pre-apply backup, observed emulator loading of disposable A/B saves, and restoration evidence per setup. | Every operation demonstrated for each workflow claimed supported; matching hashes alone do not prove emulator validity. |
| AC3 | Record Android restricted-directory limitations and supported import/export alternatives. | Picker observations, permission lifecycle results, exact restricted locations observed, emulator import/export configuration and round-trip proof where supported. | Claims explicitly scoped to tested OS, provider and emulator; inaccessible routes marked unsupported, alternatives tested or marked unverified. |
| AC4 | Unsupported access explains a concrete next step without claiming protection. | Screenshots or captured exact messages for denial, restricted folders, revoked grants and missing storage; next-step results. | No unsupported setup reports protected/synced; steps accurately lead to a supported alternative or explain that none is verified. |
| AC5 | All experimental replacement and restore operations use disposable fixtures/copies, never the owner's only real save. | Isolation record, approved disposable test root, fixture provenance, complete pre-operation manifests and operation logs identifying source/destination paths. | Every replacement/restore targets verified disposable fixtures or copies; if isolation cannot be proved, stop before mutation. The owner's only real save is never an experimental target. |

The design requirement is permission and unsupported-location screens with friendly recovery guidance. Documentation deliverables are the compatibility matrix and reproducible feasibility report. Prototype messages are acceptable evidence only when labeled prototype; they cannot establish implemented UI behavior.

## Target selection and fixture safety

1. Proceed with Eden research and isolated feasibility preparation for W1/A1 now; a second-emulator selection does not block that work. Confirm exact Windows/Android devices, OS versions and Eden builds before executing device tests. Seek a second emulator per platform for W2/A2; keep their records unselected until confirmed. The two-emulator-per-platform acceptance criterion remains unchanged.
2. Record why targets represent different useful save-access patterns. Do not substitute one emulator running two games for two emulators.
3. Use a dedicated test profile and legal disposable test software/save fixtures. Never open, overwrite, relocate or grant this test write access to real progress.
4. Record the approved test root and confirm that the emulator is configured to use it. If isolation cannot be proved, stop that setup.
5. Make disposable progress states A and B distinguishable inside the emulator (for example different test slots or values). Record the exact visible distinction. Include all related save files, not just one convenient file.
6. Stop the emulator before filesystem capture/apply unless an explicitly separate stability experiment is authorized. Record stop evidence. Do not claim live-write safety from closed-emulator tests.
7. Record fixture origin/license and hash without publishing copyrighted game payloads, personal paths or secrets. Store only sanitized evidence in the repository.

## Evidence record template

Copy once per platform/emulator/access workflow. Every blank is pending, never an implicit pass.

- Record ID: [W1/W2/A1/A2 plus route]
- Tester, date/time/timezone:
- Evidence commit or immutable attachment references:
- Device model / OS edition, version and build / architecture:
- Emulator name, official source, exact version/build:
- Version evidence (About screen plus package/build record where available):
- Disposable game/test fixture identity, version, legal provenance:
- Save type (ordinary save / save state), required bundle files:
- Configured save location (sanitized), actual location type:
- Access route (filesystem / document tree / single document / emulator import-export):
- Provider name/version, storage medium/filesystem:
- Test tool/prototype revision, build variant, permissions:
- Scope/isolation verification:
- Known limitations and unsupported configurations:
- Per-step result: PASS / FAIL / BLOCKED / NOT RUN:
- Expected outcome, actual outcome, artifact references, reproduction steps:
- Final classification: supported direct / supported import-export / unsupported / inconclusive:
- Reviewer and remaining work:

Bundle manifest template: relative path | byte count | SHA-256 | fixture state | acquisition time | record ID. Record missing and extra files as well as expected files.

## Common operation sequence (repeat for all four setups)

| Step | Procedure | Required result and evidence |
| --- | --- | --- |
| T01 Identity | Record OS/device/emulator and source/version proof; verify isolated location. | No real save path targeted. Version and location evidence attached. |
| T02 Read | Create state A through emulator, exit cleanly, enumerate the complete save bundle using the proposed access route. | All required files readable; relative paths, lengths and hashes recorded. Permission failure is explicit. |
| T03 Capture | Copy A to a separate immutable test checkpoint location; hash source and captured bundle. | Complete matching manifests; source unaffected. Partial copies are not called checkpoints. |
| T04 Distinct progress | Reopen emulator, create B, exit; capture B manifest and visible state proof. | A and B are distinguishable; A checkpoint remains unchanged. |
| T05 Backup | Before applying A, back up current B to a separate location and re-read its complete manifest. | Backup equals current B. If verification fails, stop without replacement. |
| T06 Apply | Stage A, then install the whole bundle using the tested route; record operation order and replacement limitations. | Installed bytes match A; no extra stale required files; B backup preserved. |
| T07 Load verification | Launch emulator against the installed bundle. | Emulator actually loads A without reporting corruption. Record screenshot or observation log; hash equality alone is insufficient. |
| T08 Restore | Exit emulator, preserve current state, restore verified B through the same backup/apply discipline. | Installed manifest matches B, emulator loads B, both historical copies remain available. |
| T09 Repeat | Repeat capture/apply/restore after app/device restart as applicable. | Results reproducible or limitations explicitly recorded. |
| T10 Report | Record unsupported routes, errors, next steps, and proof for each supported alternative. | Classification limited to evidence; any failed required step blocks a support claim. |

A standalone filesystem script can prove filesystem operations only. It cannot prove Android picker behavior, a future Doki UI, background safety, or emulator validity. Label the implementation used for every observation.

## Android document-picker and permissions matrix

Run against A1 and A2 routes using a real device or clearly labeled emulator. Evidence must identify Android version, target SDK/build of the test app, document provider and selected tree/document.

| ID | Test | Pass/record requirement |
| --- | --- | --- |
| A01 | Launch system picker; choose the dedicated supported test location and grant read/write access. | Record picker UI, returned access type/flags and actual read/write result. Never infer write capability from selection alone. |
| A02 | Persist the grant through supported platform APIs; fully stop/relaunch the test app. | Same URI can be reopened and operations succeed without another picker, or limitation recorded. |
| A03 | Reboot device; repeat enumeration/read and disposable write/restore. | Persisted access verified after reboot; failure triggers reauthorization rather than protected status. |
| A04 | Cancel picker and deny available permission choices. | No capture/apply; clear explanation and retry action. |
| A05 | Revoke access through available system/provider controls, then retry. | Revocation detected; no silent replacement or deletion inference; reselect-folder action verified. |
| A06 | Rename/move test folder, remove storage or disconnect provider where applicable. | Missing/unavailable state distinguished from empty save; test data preserved. Mark not applicable with reason if setup cannot perform action. |
| A07 | Inspect relevant restricted-directory picker behavior, including the emulator's configured location if inaccessible. | Record actual disabled/hidden/rejected selection behavior. No bypass, all-files permission escalation, or assumed access to another application's private directory. |
| A08 | Test any emulator-supported configurable save location. | Configure isolated accessible folder; complete T02–T08; record changed emulator settings. |
| A09 | Test emulator-supported export/import alternative when direct access is unavailable. | Export B, preserve it, import A, verify A in emulator, restore B and verify B. Record user interactions and format limitations. |
| A10 | Test provider create/overwrite/rename/delete capabilities with disposable files. | Report capabilities separately; successful reads do not establish atomic replacement. No atomicity guarantee without evidence. |

If no import/export or configurable-location feature exists, document that fact with version-specific UI/documentation evidence. Do not invent a workaround. A route can be unsupported while the record is complete, but AC2 still requires proven supported workflows for the promised initial platforms; owner review decides scope changes explicitly.

## Windows access matrix

Run W1 and W2 with isolated folders. Record normal-user access, read-only/denied access, missing test root, and a held-open disposable file where feasible. Verify errors leave checkpoint and backup copies intact. Record filesystem and replacement semantics; do not extrapolate local NTFS results to network shares, cloud placeholders or removable storage. Test those only as separately scoped configurations.

## Compatibility summary template

| Record | Platform/OS build | Emulator/version | Save type | Location/provider | Route | Read | Capture | Backup | Apply + load | Restore + load | Restart/grant | Limitations | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | Windows; exact device/OS build pending | Eden; exact version/build pending | Pending | Pending | Pending | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Pending | None |
| W2 | Windows; device/OS build pending | Unselected | Pending | Pending | Pending | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Pending | None |
| A1 | Android 15 / AYN Odin 3 (owner-reported) | Eden 0.2.1 (owner-reported) | Pending | Pending | Pending | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Pending | None |
| A2 | Android; device/OS build pending | Unselected | Pending | Pending | Pending | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | NOT RUN | Pending | None |

## Completion and review gate

- Required coverage remains two distinct emulators on Windows and two on Android.
- A PASS requires actual observed evidence linked to the record. Proposed steps, source-code review and mocked tests are not real-device evidence.
- A FAIL records expected/actual behavior, preserves artifacts and creates follow-up work; it is never silently converted into reduced criteria.
- BLOCKED/NOT RUN remains incomplete. Unsupported routes have explicit next-step guidance and do not advertise protection.
- Report byte integrity separately from emulator loading, persistence and provider replacement capabilities.
- Submit full matrix, operation logs, sanitized manifests, version evidence, screenshots, limitations and next-step copy for owner review.
- **F01 stays incomplete until real evidence satisfies the acceptance criteria and the owner accepts the story.** This protocol alone cannot close issue #12.
