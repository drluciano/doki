# Contributing to Doki

Start with the [safety contract](docs/spec/safety-contract.md) and the [engineering overview](docs/engineering.md). Coding agents also follow [AGENTS.md](AGENTS.md).

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on Linux and Windows (`.github/workflows/rust.yml`).

If you changed the app, also run `tools/app-test.sh` (format, analyze, widget tests and real-engine tests). If you changed the Android bridge, run `tools/jni-desktop-test.sh`. If you changed the wiki, run `python3 tools/wiki.py check docs/wiki`. Building the app is described in [docs/app.md](docs/app.md).

## Layout

| Path | What it is |
|---|---|
| `crates/doki-core` | Domain model: checkpoints, bundle paths, ancestry, error codes. No I/O. |
| `crates/doki-vault` | Local vault: content-addressed objects, checkpoints, records, locks. |
| `crates/doki-platform` | Save-folder adapters (`SaveLocation`) and fault injection. |
| `crates/doki-engine` | Capture, automatic checkpoints, whole systems, review, install with verified backup, recovery, deletion and retention, handoff packages. |
| `crates/doki-peer` | Device-to-device sync: pairing, trusted device book, direct encrypted sessions (iroh). |
| `crates/doki-relay` | Self-hostable relay server (`doki-relay`). |
| `crates/doki-sync` | Relay client: pairing, encryption, sync. |
| `crates/doki-cli` | The `doki` command. |
| `crates/doki-api` | JSON command interface used by every non-Rust front end. |
| `crates/doki-jni` | JNI bridge exposing `doki-api` to Java with Java-supplied save folders. |
| `crates/doki-ffi` | C ABI over `doki-api` for the desktop app (Dart FFI). |
| `app` | Flutter app; `tools/app-test.sh` runs its checks. See [docs/app.md](docs/app.md). |
| `android/bridge` | Java side of the bridge (`dev.doki.bridge`); `tools/jni-desktop-test.sh` runs it on a desktop JVM. |
| `tools/f01-saf-probe` | Android folder-access probe for F01. |

## Rules

- Any code path that writes playable files goes through the journaled install in `crates/doki-engine/src/install.rs`. Add a crash-point test for every new step.
- Update the safety contract in the same PR as behaviour it describes.
- Use synthetic or licensed fixtures. Keep ROMs, real saves and secrets out of the repository.
- Pull requests reference their issue with `Refs #N` rather than closing keywords. The maintainer merges pull requests and closes issues.
- Update the docs and wiki pages (`docs/wiki`) a change affects in the same pull request.

No license has been chosen yet. Do not copy code or assets from other projects into Doki.
