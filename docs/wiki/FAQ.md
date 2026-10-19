# FAQ

### Does Doki need an account or a server?
No. Devices pair with a code and talk to each other directly. A [self-hosted relay](Self-Hosted-Relay.md) is optional.

### Will a sync ever overwrite my save?
No. Synced progress waits in **Incoming**. Your save changes only when you choose to use or restore something on that device, and only after a verified backup.

### Can Doki merge two saves?
No. Save files are binary and merging them would corrupt them. When you played on two devices, you choose one whole version and the other is kept in History.

### Which emulators are supported?
Doki works with any emulator that keeps a game's save as files in a folder. No emulator is listed as supported yet, because Doki only lists one after a full restore has been tested on a real device. Testing starts with Eden on an AYN Odin 3 (Android 15).

### Do I have to make checkpoints myself?
No. Doki makes one on its own once a save has stopped changing for 30 seconds. See [Automatic checkpoints](Automatic-Checkpoints.md).

### Do I need to close the game before **Checkpoint now**?
Yes. Doki asks you to confirm the game is closed so the save is complete. If files change while it copies, it stops and asks you to try again. Automatic checkpoints do not need this, because they wait for the save to settle.

### What happens if my devices' clocks are wrong?
Nothing bad. Doki uses each save's history, not clocks, to decide how saves relate. Times are shown only to help you.

### Do both devices need to be on at the same time?
For direct sync, yes, with Doki running on both (in the background on Android, or in the tray on Windows). If that rarely happens, use a [relay](Self-Hosted-Relay.md) or a handoff file.

### Does Doki work in the background?
Yes. On Android, **Keep working in the background** (on by default) keeps saving checkpoints and syncing, with a notification showing. On Windows, Doki keeps working from the tray after you close its window, and can start with Windows. See [Settings and notifications](Settings-and-Notifications.md).

### Is my data encrypted?
Direct sync and the relay are end-to-end encrypted. Handoff files are not encrypted yet; move them only over channels you trust.

### Does Doki delete old checkpoints?
Only if you ask. By default every checkpoint is kept. You can delete checkpoints yourself or choose **Keep checkpoints for** a set time; the newest 10 of each game, current saves and pinned checkpoints always stay. See [Freeing up space](Freeing-Up-Space.md).

### How do I update the Android app?
For now, a new APK cannot be installed over the old one, and uninstalling deletes Doki's data on that device. Sync with another device first. See [Updating Doki](Updating-Doki.md).

### Can I change how Doki looks?
Yes. Under **Settings**, pick a theme (Follow system, Light, Dark or OLED) and the border color. Game covers come from SteamGridDB or your own images; see [Game art](Game-Art.md).

### I lost a device. What now?
Remove it under **Devices** on any other device. The removal spreads to all of them, and the lost device can no longer sync with yours.

### Is it on iOS or macOS?
Not today. Android and Windows are the first targets.
