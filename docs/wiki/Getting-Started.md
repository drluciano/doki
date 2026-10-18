# Getting started

This page takes you from nothing to a protected save in a few minutes. You need the save folder of a game in an emulator on your device.

> Doki is pre-release. Use a copy of your save the first time, or a game you do not mind experimenting with.

## 1. Install Doki

There is no published release yet. Test builds are made automatically for every change:

1. Open the [App workflow runs](https://github.com/drluciano/doki/actions/workflows/app.yml) and pick the latest run with a green check on `main`.
2. Under **Artifacts**, download:
   - **doki-windows** for Windows. Unzip it and run `doki.exe`.
   - **doki-android** for Android. Unzip it and install the APK. Your device will ask you to allow installing apps from this source. The APK is debug-signed, so a later build may need you to uninstall the previous one first.

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

## 4. Create your first checkpoint

1. Close the game in the emulator, so the save is complete.
2. Open the game in Doki and choose **Create checkpoint**.
3. Confirm the game is closed.

Doki copies every file, checks each one, and shows **Checkpoint created. Verified copy kept.** If nothing changed since the last checkpoint, it tells you instead of making a duplicate.

Do this whenever you finish a session you want to keep or hand off.

## 5. Add your other devices

Install Doki on your other device, add the same game there, and pair the two. See [Pairing your devices](Pairing-Devices.md).

When progress arrives, it appears under **Incoming**. Nothing changes on that device until you review it and choose. See [Reviewing incoming progress](Reviewing-Incoming-Progress.md).

## What next

- [History and restoring](History-and-Restoring.md)
- [How Doki keeps your saves safe](How-Doki-Keeps-Saves-Safe.md)
- [FAQ](FAQ.md)
