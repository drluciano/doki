# Engineering plan

Status: the local safety engine is implemented in `crates/` (Rust workspace) and specified in [spec/safety-contract.md](spec/safety-contract.md), which is normative where this plan differs. The relay, the Flutter app (`app/`) and the Android folder adapter are built; the Android adapter awaits hardware testing (F01).

## Proposed module boundaries

- `app` (built, tutorial pending): Flutter UI with Library, Incoming, History, Devices, review and recovery. Filesystem and protocol decisions stay outside widgets: every action is one JSON request through `crates/doki-api`, reached by `crates/doki-ffi` (desktop, Dart FFI) or `crates/doki-jni` with `android/bridge` (Android). The bridge is hand-written and narrow (one function each way), so no binding generator is needed. See [app.md](app.md).
- `crates/doki-core` (built): domain types, checkpoint identity, bundle path rules, ancestry relation and the stable error taxonomy. Pure; no I/O.
- `crates/doki-vault` (built): immutable content-addressed objects, checkpoints, atomically replaced records and OS locks. JSON files are the source of truth; a faster index may be added if it stays rebuildable from them.
- `crates/doki-platform` (built): the `SaveLocation` interface for reading, staging and replacing save files, with capability reporting (`ReplaceMode`) and fault injection. The Android Storage Access Framework adapter (`app/android/.../SafSaveFolder.java`) implements the same interface through the JNI bridge.
- `crates/doki-engine` (built): capture, proposals, review, journaled install with verified backup, recovery, and handoff packages (the first transport). No network code.
- `crates/doki-cli` (built): the `doki` command, used for testing and power users until the app exists.
- `packages/emulator_profiles` (planned): versioned declarative discovery, bundle membership and compatibility rules. No arbitrary downloaded executable plugins in the initial release. Until then every bundle uses the `generic-folder` profile (all files in the chosen folder).
- `packages/inspectors` (planned): optional format validators and human-readable save metadata; no generic binary merge promise.
- `crates/doki-peer` (built): device-to-device sync with no host. Per-device keys, a trusted device book shared between devices, pairing codes (long, or short on the same network), direct encrypted sessions through iroh. This is the default transport.
- `crates/doki-relay` (built, optional): small self-hostable store-and-forward server for encrypted handoff packages. `crates/doki-sync` (built) is the client: pairing codes, end-to-end encryption, sync. Per-device keys and revocation (S01) are required before internet-facing release.
- `docs`: architecture decisions, contracts, schemas, threat model, contributor guide, user help and recovery procedures.

Flutter/Rust interoperability is settled as a JSON boundary (`doki-api`): errors convert to the stable codes of the safety contract (section 11) on every platform. Packaging is measured in CI: the desktop build compiles the engine through CMake, and the APK build checks the engine library is present for each ABI.

## Non-negotiable safety contract

1. Receiving data changes the vault only. Applying an incoming save requires explicit review in the MVP.
2. Every replacement requires a verified, recoverable capture of the current complete bundle. Failure to back up means refusal to apply.
3. Checkpoints are immutable and record their parents, manifest hashes, originating device and format/profile version. Time is descriptive; ancestry determines divergence.
4. Independent continuations remain preserved. Selecting one never implicitly deletes the other.
5. Immediately before apply, recheck local state against the reviewed baseline. Any change requires a new checkpoint and renewed review.
6. Partial capture, missing media, permission loss or an inaccessible folder cannot imply deletion.
7. Treat multi-file saves as a bundle. Stability heuristics cannot guarantee a coherent emulator write; unsupported cases require an explicit emulator-stopped workflow.
8. Never infer semantic validity from a hash. Integrity and format validation are separate claims.
9. Retention cannot collect objects referenced by active installs, recovery journals, pending proposals or pinned checkpoints. Cleanup is independently reviewed and specified.
10. Errors must name what happened, what is preserved, and what recovery action is available, without leaking sensitive save content or credentials.

## Apply state machine

Specified step by step, with the outcome of an interruption at each step, in [spec/safety-contract.md](spec/safety-contract.md) section 6, and implemented in `crates/doki-engine/src/install.rs`. In short: recheck against the review, verify the backup, journal, stage and verify, mark installing, replace files, verify the folder, then commit. Head never moves before the installed files verify, and recovery either finishes or puts the backup back using the same idempotent procedure.

## Feasibility gates

- Android: demonstrate actual access to representative emulator save locations through supported permissions/providers, provider replacement semantics, permission persistence, SD-card removal and background restrictions. Publish a tested emulator/device/OS compatibility matrix. No unrestricted `Android/data` access assumption.
- Windows: measure emulator locks and write patterns, directory permissions, path edge cases, staging/rename behavior and crash recovery on supported filesystems.
- Shared: transfer and restore at least one real save bundle in each direction with both originals retained. Test independent offline progress before reconnecting.
- Linux/macOS: separate adapter and packaging research; verify platform permission/credential storage and filesystem behavior before declaring support.
- iOS: research supported emulator import/export and authorized document integration. Background execution and cross-app access are feasibility questions, not feature commitments.
- Security: accepted threat model covers account/device authorization, transport security, untrusted manifests, resource limits, credentials, revocation, lost devices, recovery and encryption/key ownership. Do not invent custom cryptography.

## Dependency sequence

1. Accept product vocabulary, safety contract, workflow and initial supported use cases.
2. Complete Android/Windows filesystem and Flutter/Rust integration spikes; accept stack and storage decisions.
3. Implement a local checkpoint, verify, history, restore and restart-recovery vertical slice; prove fault behavior before network writes exist.
4. Define protocol/schema versioning, threat model and pairing; implement vault-only exchange with retries and integrity validation.
5. Add ancestry-based proposals, divergence review and user-approved application.
6. Integrate library, tutorial, accessible motion and error/recovery presentation with real engine states.
7. Validate real-device workflows, documentation, packaging and beta release process.
8. Expand platforms, transports, inspectors and optional automation only through separate accepted stories.

## Tests and releases

Use domain/state-machine unit tests, property-based ancestry/integrity tests, storage integration tests and fault-injection tests. Test malformed or hostile manifests, oversized inputs, concurrency, disk full, interrupted transfer/install, lost permission, clock skew and missing storage. End-to-end tests must assert original save preservation and successful restore, not only successful transfer.

Use representative disposable fixtures with clear licensing and provenance. Never commit personal saves, tokens or identifying diagnostics. CI runs formatting, static analysis, meaningful tests, secret/dependency checks and documentation checks appropriate to each change. A supported-platform matrix defines required hardware testing and explicitly separates tested support from experimental support.

Version schemas/protocols and test upgrades and unsupported versions. Release gates include reproducible build instructions, checksums/signing where configured, migration/recovery guidance, supported versions, release notes and known limitations. Do not promise byte-identical reproducibility until proven. Publishing production releases is a separately scoped action.

## Documentation standard

Document public interfaces, error codes, schemas, configuration, invariants, decisions, ownership, setup, tests, debugging, packaging, support matrix and troubleshooting. Safety-critical code comments explain why ordering matters. User documentation covers first backup, device pairing, conflict choice, restore, retention, lost access and export. Update documentation in the same PR as behavior. CI checks links/examples where practical; avoid tests that merely mirror implementation.

## Ready for engineering

A story is eligible only when its acceptance criteria are observable, scope is bounded, dependencies are accepted, design is approved where required, failure/accessibility/recovery states are specified, technical unknowns are resolved or explicitly the subject of a spike, test evidence is defined, and documentation deliverables are named. Design-exempt infrastructure stories must record why design is unnecessary.

## Ready for user review and definition of done

Ready for review requires an issue-linked PR, scoped changes, passing required checks, updated documentation, screenshots/video for visible behavior, explicit manual verification instructions, and a runnable artifact or clear local reproduction when practical. Report platform limitations and deferred criteria honestly. Never weaken acceptance criteria to make the implementation pass.

Only the user accepts a story. The PR Reviewer may mark ready and merge technically passing PRs under the owner's standing authorization and repository requirements before story acceptance. A story is Done only when its complete criteria are accepted by the user and its implementation is merged; a partial-research PR merge does not complete it. Requested changes return the same story to engineering or design. Passing CI, agent self-review, labels and PR creation do not constitute user acceptance.

Exactly one engineering story may be active, including a story Awaiting user review. The engineer may prepare evidence or respond to review on that story, but cannot start another implementation story until the user accepts the current work. The primary agent may perform bounded PM/design preparation on demand without bypassing dependencies; these responsibilities do not require separate running agents. Approval is explicit and recorded; silence or elapsed time never approves work.
