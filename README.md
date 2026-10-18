# doki
A save handoff tool with version history and review, where syncing a save and activating it are separate actions. The core promise: another device’s save never silently replaces your current progress. Incoming progress becomes something you can review, use, keep alongside your own, or dismiss.

## Status

The safety engine works today for save folders on Windows, Linux and macOS, driven from a command line. It captures checkpoints, hands them to another device as a file or through a small self-hostable relay with end-to-end encryption, detects when both devices played, backs up and verifies before replacing anything, and recovers from interrupted installs. The Flutter app and the Android adapter are next.

## Try it

Requires Rust 1.89 or newer.

```sh
cargo build --release
./target/release/doki init --name PC
./target/release/doki add "Pokémon Emerald" "C:\path\to\emulator\saves"
./target/release/doki capture "Pokémon Emerald" --closed
./target/release/doki send "Pokémon Emerald" -o emerald.dokipkg
```

On the other device: `doki init`, `doki receive emerald.dokipkg`, link the game with the `doki add … --link <id>` command it prints, then `doki incoming` and `doki use`. The full walkthrough is in [docs/cli.md](docs/cli.md).

Use copies of saves while Doki is pre-release.

## Documentation

- [Safety contract](docs/spec/safety-contract.md): what Doki guarantees, every state and what happens after an interruption.
- [Open questions](docs/spec/open-questions.md): defaults chosen where the brief left a decision open.
- [Command line](docs/cli.md), [relay](docs/relay.md), [engineering plan](docs/engineering.md), [design](docs/design.md), [contributing](CONTRIBUTING.md).
