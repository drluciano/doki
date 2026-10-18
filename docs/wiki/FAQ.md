# FAQ

### Does Doki need an account or a server?
No. Devices pair with a code and talk to each other directly. A [self-hosted relay](Self-Hosted-Relay.md) is optional.

### Will a sync ever overwrite my save?
No. Synced progress waits in **Incoming**. Your save changes only when you choose to use or restore something on that device, and only after a verified backup.

### Can Doki merge two saves?
No. Save files are binary and merging them would corrupt them. When you played on two devices, you choose one whole version and the other is kept in History.

### Which emulators are supported?
Doki works with any emulator that keeps a game's save as files in a folder. No emulator is listed as supported yet, because Doki only lists one after a full restore has been tested on a real device. The first one being tested is Eden on an AYN Odin 3 (Android 15).

### Do I need to close the game before creating a checkpoint?
Yes. Doki asks you to confirm the game is closed so the save is complete. If files change while it copies, it stops and asks you to try again.

### What happens if my devices' clocks are wrong?
Nothing bad. Doki uses each save's history, not clocks, to decide how saves relate. Times are shown only to help you.

### Do both devices need to be on at the same time?
For direct sync, yes, with Doki open on both. If that rarely happens, use a [relay](Self-Hosted-Relay.md) or a handoff file.

### Does Doki sync in the background on Android?
Not yet. It syncs while the app is open.

### Is my data encrypted?
Direct sync and the relay are end-to-end encrypted. Handoff files are not encrypted yet; move them only over channels you trust.

### Does Doki delete old checkpoints?
Not yet. There is no automatic cleanup in this version.

### I lost a device. What now?
Remove it under **Devices** on any other device. The removal spreads to all of them, and the lost device can no longer sync with yours.

### Is it on iOS or macOS?
Not today. Android and Windows are the first targets.
