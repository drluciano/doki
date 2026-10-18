# Instructions for coding agents

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [safety contract](docs/spec/safety-contract.md) before changing code. [docs/engineering.md](docs/engineering.md) explains the modules; [docs/design.md](docs/design.md) the app's wording, look and motion.

## How work is delivered

- Work in a branch and open a pull request that references its issue (`Refs #N`, not `Fixes #N`). The maintainer merges pull requests and closes issues.
- Batch related code, docs and tests into one meaningful pull request rather than many small ones.
- Run the checks in CONTRIBUTING.md before pushing. A check that did not run is not a pass.

## Rules that protect saves

- Receiving progress never installs it. Keep the verified backup before every replacement, the recheck against the review, immutable history and journaled recovery.
- Any code that writes playable files goes through `crates/doki-engine/src/install.rs`, with a crash-point test for every new step.
- Never claim a platform or emulator is supported without a real-device restore, and never describe two binary saves as mergeable.
- Update the safety contract, the docs and the wiki (`docs/wiki`) in the same pull request as the behaviour they describe. Comments on safety-critical code explain why the order matters.
- Keep real saves, ROMs, credentials and identifying diagnostics out of the repository; use synthetic fixtures.
