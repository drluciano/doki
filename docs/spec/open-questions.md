# Open questions for the owner

Each item has the default the code uses today, so work is not blocked. Change any of them by saying so; the default is cheap to change unless noted.

| # | Question | Current default |
|---|---|---|
| Q1 | The brief arrived in two versions with different wording (e.g. "Separate progress" vs "Save path", "Incoming progress" vs "Save proposal", screens Library/Incoming/History/Devices vs Library/Reviews/Devices). Which wins? | The first version (the newer brief): Checkpoint, Separate progress, Incoming progress, Use this save, Restore a checkpoint; screens Library, Incoming, History, Devices. `docs/design.md` and the concept sheets now use it. |
| Q2 | What exactly do "Keep alongside" and "Dismiss" mean? | Keep alongside: stop prompting, keep it in history, can be reopened; records nothing, so the other device may raise the same divergence later. Dismiss: keep this device's save and record that choice in ancestry, so no device asks again. |
| Q3 | After an interrupted install, may Doki resolve it automatically on next launch? | No. Doki blocks the game and asks: Finish installing, or Put back the previous save. If all files are already in place it finishes without asking. |
| Q4 | Retention: what should cleanup keep by default, and what storage limit? | No automatic deletion in the MVP. Pins exist; `clean` only removes leftovers. Proposed later default: keep pinned, current, separate progress, last 10 per continuation, and backups for 30 days. |
| Q5 | Should the relay keep history (a vault) or only forward until every paired device has fetched it? | Forward-and-expire relay with a retention window; not presented as a backup. Expiry never propagates as deletion. |
| Q6 | End-to-end encryption: should the relay see file names and game names? | No. Content and metadata are encrypted on the device; the relay sees opaque ids and sizes. Needs the S01 design before any internet-facing use. |
| Q7 | Which emulators and games form the first compatibility set? | Eden 0.2.1 on AYN Odin 3 (Android 15) as already reported. A Windows emulator and a second Android emulator are still needed; the engine works on any folder of save files meanwhile. |
| Q8 | The brief's tutorial section is cut off after "The tutorial should use **sample saves". | Use the second version's text: disposable example saves teaching capture, receive, divergence and restore before connecting a real folder (`docs/design.md`, "Tutorial"). |
| Q9 | Should a save deleted in the emulator ever be recorded as "deleted on purpose"? | Not in the MVP. An empty or missing folder is never captured; the last checkpoint stays available. |
| Q10 | The repository's delivery rules (one story at a time, F02 waiting on F01 device evidence, 39 issues) slowed delivery to zero application code. | Following your 2026-10-18 instruction: build the MVP in incremental draft PRs; the engine is proven on synthetic saves now, and device-specific claims still wait for real-device evidence. See `docs/workflow.md`. |
