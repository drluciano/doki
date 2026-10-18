# Getting started

This page takes you from nothing to a protected save in a few minutes. You need the save folder of a game in an emulator on your device.

> Doki is pre-release. Use a copy of your save the first time, or a game you do not mind experimenting with.

## 1. Install Doki

There is no published release yet. Test builds are made automatically for every change:

1. Open the [App workflow runs](https://github.com/drluciano/doki/actions/workflows/app.yml) and pick the latest run with a green check on `main`.
2. Under **Artifacts**, download:
   - **doki-windows** for Windows. Unzip it and run `doki.exe`.
   - **doki-android** for Android. Unzip it and install the APK. Your device will ask you to allow installing apps from this source.

> **Updating on Android:** for now, a newer APK cannot be installed over an older one, and uninstalling deletes Doki's checkpoints on that device. Read [Updating Doki](Updating-Doki.md) before you update.

Prefer the command line? See [Command line](Command-Line.md).

## 2. Name this device

The first screen asks for a name, such as "Living room PC" or "Odin 3". Your other devices show this name next to progress that came from here. You can use any name. No account or network is needed.

## 3. Add a game

Go to **Library** and choose **Add game**.

1. Type the game's title.
2. Choose the folder where the emulator keeps this game's save. On Android, Doki opens the system folder picker and asks for access to that folder only.
3. Choose **Connect folder**.

Connecting a folder only records where the save lives. Doki does not change anything in it.

> Each game should point at the folder holding *that game's* save files. Keep Doki's own storage separate from every save folder; Doki refuses folders that overlap.

Play many games in one emulator? Choose **Whole system** instead and pick the emulator's save folder: every game in it is tracked on its own, and new ones are added automatically. See [Tracking a whole emulator](Tracking-a-Whole-Emulator.md). For covers in your library, see [Game art](Game-Art.md).

## 4. Play, and let Doki make checkpoints

Doki makes checkpoints on its own. After you play, once the save has stopped changing for 30 seconds, it saves a verified copy. Library shows **New progress, checkpoint coming soon** while it waits, then **Protected by a checkpoint**. See [Automatic checkpoints](Automatic-Checkpoints.md).

Keep Doki running while you play: on Android it keeps working in the background (with a notification), and on Windows it keeps working from the tray when you close its window.

To make one right away, close the game in the emulator, open it in Doki and choose **Checkpoint now**. Doki copies every file, checks each one, and shows **Checkpoint created. Verified copy kept.** If nothing changed since the last checkpoint, it tells you instead of making a duplicate.

## 5. Add your other devices

Install Doki on your other device and pair the two. See [Pairing your devices](Pairing-Devices.md). After the first sync, games you added here appear on the other device under **On your other devices** in Library; tap one and choose where it saves there.

When progress arrives, it appears under **Incoming**. Nothing changes on that device until you review it and choose. See [Reviewing incoming progress](Reviewing-Incoming-Progress.md).

## What next

- [History and restoring](History-and-Restoring.md)
- [Settings and notifications](Settings-and-Notifications.md)
- [How Doki keeps your saves safe](How-Doki-Keeps-Saves-Safe.md)
- [FAQ](FAQ.md)
