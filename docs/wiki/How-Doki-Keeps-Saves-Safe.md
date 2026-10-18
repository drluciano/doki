# How Doki keeps your saves safe

Doki's one promise: **another device's save never silently replaces your progress.** These are the rules that keep it. The full technical version is the [safety contract](https://github.com/drluciano/doki/blob/main/docs/spec/safety-contract.md).

## Receiving is not using

Syncing only stores progress inside Doki. Your save folder changes only when you choose **Use this save** or **Restore this checkpoint** on that device, or finish a recovery.

## Checkpoints never change

A checkpoint is an exact copy of a save's files, checked with a fingerprint (SHA-256) of every file. Once made, it is never edited. A checkpoint that is only partly copied or received is never shown or offered.

## A verified backup before every replacement

Before replacing your save, Doki makes sure your current save is safely in a checkpoint and that the checkpoint's files still read back correctly. If it cannot confirm that, it replaces nothing (**Could not confirm a backup**).

## Your approval is for exactly what you saw

Right before replacing anything, Doki re-reads your save folder. If you played since the review, it saves that new progress as a checkpoint and asks you to review again, rather than overwriting it.

## History decides, never the clock

Doki decides whether one save continues another by following which checkpoint each one came from. It never uses clocks, file dates or sizes, so a device with the wrong time, or a save that got smaller, cannot trick it into picking the wrong one. When both devices played, it says so and asks you.

## Choosing keeps both

Choosing one version never deletes the other. It stays in History as *Separate progress*. Your choice is recorded, so your other devices accept it without asking again.

## Interruptions are recoverable

Every replacement is written to a journal step by step. If it stops halfway, Doki knows exactly which files are old and which are new, blocks the game, and asks you to **Finish the change** or **Put my save back**. See [Recovering an interrupted change](Recovering-an-Interrupted-Change.md).

Every step of this process is tested by simulating a crash at that exact point.

## Missing is not deleted

A missing folder, revoked permission, removed SD card or empty folder is treated as "not available right now", never as "the save was deleted". Doki never records an empty save, and never deletes anything because another device no longer has it.

## Nothing is deleted on its own

There is no automatic cleanup of checkpoints yet. When cleanup arrives, it will show you what goes first, and pinned checkpoints will always be kept.

## Honest errors

Every error says what happened, **whether your save changed**, and what to do next. See [Troubleshooting](Troubleshooting.md).

## What Doki does not promise

- A matching fingerprint proves the files are exactly as they were. It does not prove the game can load them. Doki makes no claim that a save works in a particular emulator until that has been tested on a real device.
- Sync is not a backup. Keep your own backups of saves you care about.
- Android folder access is still being verified on real hardware. Treat Android installs as experimental for now.
