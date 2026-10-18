# Contributing to Doki

Start with the [safety contract](docs/spec/safety-contract.md), the [engineering plan](docs/engineering.md), [workflow](docs/workflow.md) and [agent rules](AGENTS.md).

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on Linux and Windows (`.github/workflows/rust.yml`).

## Layout

| Path | What it is |
|---|---|
| `crates/doki-core` | Domain model: checkpoints, bundle paths, ancestry, error codes. No I/O. |
| `crates/doki-vault` | Local vault: content-addressed objects, checkpoints, records, locks. |
| `crates/doki-platform` | Save-folder adapters (`SaveLocation`) and fault injection. |
| `crates/doki-engine` | Capture, review, install with verified backup, recovery, handoff packages. |
| `crates/doki-relay` | Self-hostable relay server (`doki-relay`). |
| `crates/doki-sync` | Relay client: pairing, encryption, sync. |
| `crates/doki-cli` | The `doki` command. |
| `tools/f01-saf-probe` | Android folder-access probe for F01. |

## Rules

- Any code path that writes playable files goes through the journaled install in `crates/doki-engine/src/install.rs`. Add a crash-point test for every new step.
- Update the safety contract in the same PR as behaviour it describes.
- Use synthetic or licensed fixtures. Keep ROMs, real saves and secrets out of the repository.
- PRs reference issues without automatic closure. The owner accepts and completes stories.

A project license has not been selected; do not imply third-party permission or copy reference assets.
