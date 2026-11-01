# Changelog

Each release of Doki has a section here. The release workflow copies the section for the version being released into the GitHub release notes, so write it for players, not developers. See [docs/release.md](docs/release.md) for how a release is made.

## 0.1.0

The first preview of Doki: save handoff between your devices with a history you can go back to. Progress from another device never replaces your save until you choose it.

This is an early preview. Use copies of your saves, or a game you do not mind experimenting with. Android and Linux are experimental until they are verified on real hardware.

**What's in it**

- Checkpoints: exact, verified copies of a game's save, made on their own once a save has stopped changing for 30 seconds.
- Whole emulators: add an emulator's save folder once and every game in it is tracked on its own.
- Incoming progress waits for you. Review it, then **Use this save**, keep it alongside your own, or keep your save. If you played on both devices, Doki tells you.
- **Use this save** and **Restore a checkpoint** verify a backup of your current save first, and an interrupted change can always be finished or undone.
- Pair devices with a short code. They sync directly over an encrypted connection, with no account and no server. An optional self-hosted relay is available.
- Delete checkpoints or keep them for a set time, with a preview of what goes.
- Game covers and banners, Light, Dark and OLED themes, notifications, and keyboard and controller support.
- Windows installer, Android app, and Linux Flatpak for x86_64 (Steam Deck, PCs) and aarch64 (ARM handhelds such as the Odin 3).

**Known limitations**

- The Windows installer is not signed yet, so Windows may show "Windows protected your PC". Choose **More info**, then **Run anyway**.
- Android: if you installed an earlier test build, uninstall it first. This release is signed with Doki's own key, so later releases will install over it and keep your data. Read [Updating Doki](https://github.com/drluciano/doki/wiki/Updating-Doki) before you uninstall.
- Not built yet: the practice tutorial with sample saves, QR codes for pairing, and encrypted handoff files.
