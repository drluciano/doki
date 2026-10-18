# Engineering plan

Status: proposed architecture; subject to feasibility research and user acceptance. No application implementation is authorized by this document alone.

## Proposed module boundaries

- `apps/doki`: Flutter UI, navigation, accessible design system, tutorial, proposal review, history and device management. Keep filesystem and protocol decisions outside widgets.
- `crates/core`: Rust domain types, checkpoint ancestry, conflict detection, compatibility decisions, proposal lifecycle and apply/recovery state machine. Independent of Flutter and network providers.
- `crates/vault`: immutable content-addressed objects, manifests, verification, retention planning and metadata transactions. SQLite is a candidate index, not the sole copy of history.
- `crates/sync`: versioned exchange protocol, bounded retries, resumable transfer, authentication and device permissions; no direct writes to playable saves.
- `crates/platform`: interfaces for safe capture, staging, replacement, durable journal storage and permission checks. Native implementations must report capabilities rather than claim universal atomicity.
- `packages/emulator_profiles`: versioned declarative discovery, bundle membership and compatibility rules. No arbitrary downloaded executable plugins in the initial release.
- `packages/inspectors`: optional format validators and human-readable save metadata; no generic binary merge promise.
- `services/vault`: candidate authenticated remote object/proposal service. Hosting and encryption design require an accepted architecture decision before implementation.
- `docs`: architecture decisions, contracts, schemas, threat model, contributor guide, user help and recovery procedures.

Flutter/Rust interoperability, packaging complexity, platform capabilities and team maintenance cost must be measured in a spike before this stack becomes final. Keep generated binding code distinct and document ownership and error conversion.

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

`Received -> Verified -> Reviewable -> UserApproved -> RecheckLocal -> BackupCurrent -> VerifyBackup -> StageIncoming -> JournalPrepared -> Install -> VerifyInstalled -> Committed`

Each transition has documented preconditions and idempotency behavior. Persist authorization bound to the exact incoming manifest and reviewed local baseline; a changed proposal invalidates approval. Network completion is never installation completion.

Before any destructive write, persist sufficient recovery information durably. On restart, resolve incomplete journals before another apply. Platform adapters choose an evidenced capability-specific strategy: atomic directory/file replacement when supported, otherwise a journaled recoverable sequence. A multi-file install may be temporarily mixed; the UI must prevent use until verification or restoration succeeds. If recovery cannot complete, stop and expose the verified backup and clear instructions. Do not claim universal atomicity or guaranteed recovery from physical media failure.

State tests cover every transition, failed precondition and injected interruption, including after each file operation. Cancellation before installation is safe; during installation it follows the journal's recovery protocol.

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

Only the user accepts a story. Accepted implementation is merged according to repository protection; then the story is Done. Requested changes return the same story to engineering or design. Passing CI, agent self-review, labels and PR creation do not constitute user acceptance.

Exactly one engineering story may be active, including a story Awaiting user review. The engineer may prepare evidence or respond to review on that story, but cannot start another implementation story until the user accepts the current work. PM and Designer may prepare a bounded queue asynchronously without bypassing dependencies. Approval is explicit and recorded; silence or elapsed time never approves work.
