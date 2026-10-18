# doki
A save handoff tool with version history and review, where syncing a save and activating it are separate actions. The core promise: another device’s save never silently replaces your current progress. Incoming progress becomes something you can review, use, keep alongside your own, or dismiss.

## Status

The safety engine captures checkpoints, hands them to another device directly (peer-to-peer, end-to-end encrypted, no host or account), as a file, or through an optional self-hostable relay, detects when both devices played, backs up and verifies before replacing anything, and recovers from interrupted installs. You can drive it from the Flutter app on Windows and Android (Linux for development) or from the command line. The Android folder adapter is built but not yet tested on real hardware.

## Try it

The app: see [docs/app.md](docs/app.md). CI publishes test builds for Windows and Android.

The command line:

Requires Rust 1.91 or newer.

```sh
cargo build --release
./target/release/doki init --name PC
./target/release/doki add "Pokémon Emerald" "C:\path\to\emulator\saves"
./target/release/doki capture "Pokémon Emerald" --closed
./target/release/doki send "Pokémon Emerald" -o emerald.dokipkg
```

To pair devices instead, run `doki devices invite` on one and `doki devices join CODE` on the other, then `doki devices sync`. With a file: on the other device, `doki init`, `doki receive emerald.dokipkg`, link the game with the `doki add … --link <id>` command it prints, then `doki incoming` and `doki use`. The full walkthrough is in [docs/cli.md](docs/cli.md).

Use copies of saves while Doki is pre-release.

## Documentation

- [Safety contract](docs/spec/safety-contract.md): what Doki guarantees, every state and what happens after an interruption.
- [Open questions](docs/spec/open-questions.md): defaults chosen where the brief left a decision open.
- [App](docs/app.md), [command line](docs/cli.md), [device-to-device sync](docs/peer.md), [relay](docs/relay.md), [engineering plan](docs/engineering.md), [design](docs/design.md), [contributing](CONTRIBUTING.md).
