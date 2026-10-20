# Troubleshooting

Every Doki error says what happened, **whether your save changed**, and what to do next. Technical details are under **Details**, with a code you can search for here.

## Errors

| You see | Code | Did your save change? | What to do |
|---|---|---|---|
| Save location unavailable | `SourceUnavailable` | No | Reconnect the storage (SD card, USB drive) or choose the folder again. |
| Folder access needs updating | `PermissionLost` | No | Choose the folder again to allow access. Common on Android after a reinstall. |
| Waiting for the save to finish | `SourceChanging` | No | The files changed while Doki copied them. Close the game completely, then try again. |
| This folder has no save files | `BundleIncomplete` | No | Check that you chose the folder that holds the save. Doki never records an empty save. |
| Doki storage is full | `VaultFull` | No | Free some space on this device. |
| The save folder is full | `SaveLocationFull` | No | Free some space where the save is stored. |
| A received copy is damaged | `IntegrityFailed` | No | Sync again. If it keeps happening, report it. |
| You played since reviewing | `LocalChangedSinceReview` | No | Your newer progress was kept as a checkpoint. Review again. |
| Could not confirm a backup | `BackupUnverified` | No | Nothing was replaced. Try again in a moment. |
| This folder cannot be updated safely | `StrategyUnsupported` | No | The storage cannot replace files without risk, so Doki will not. Copy a checkpoint's files out with the command line (`doki export-files`) and put them in place yourself. |
| Finish or undo the last change | `InstallInterrupted`, `RecoveryRequired` | **Maybe** | Open the game and choose Finish or Put back. See [Recovering an interrupted change](Recovering-an-Interrupted-Change.md). |
| Recovery could not finish | `RecoveryFailed` | **Maybe** | The game stays protected. Fix the cause and try again. You can also copy any checkpoint's files out with the command line (`doki export-files`). |
| Doki is busy with this game | `Busy` | No | Another action on this game is running. Wait a moment and try again. |
| Made by a newer Doki | `UnsupportedVersion` | No | Update Doki on this device. |
| Not found / That did not look right | `NotFound`, `InvalidInput` | No | Refresh, or check what you entered. |

## Pairing and sync

**"That code has expired or is wrong."** Codes last 15 minutes. Show a new code on the other device. After 20 wrong attempts a device closes all its codes; show a new one.

**"That code is from this device."** Enter it on your *other* device.

**The short code finds nothing (Looking for the other device…).** Both devices must be on the same network, and the network must allow local broadcasts. Guest, hotel and some mesh networks block them. Use the long code instead.

**"Could not reach the other device" / "None of your devices could be reached."** Make sure Doki is running on the other device (open, in the Windows tray, or with **Keep working in the background** on for Android), then choose **Sync now**.

**Progress arrived but nothing changed.** That is expected. Look under **Incoming** and review it. See [Reviewing incoming progress](Reviewing-Incoming-Progress.md).

**Progress arrived for a game I have not set up.** Choose **Set up …** under Incoming and pick that game's save folder on this device.

## Updating

**"App not installed as it conflicts with an existing package" on Android.** Each test build is currently signed with a different key ([#82](https://github.com/drluciano/doki/issues/82)), so Android will not install it over the old one. Uninstalling first works but deletes Doki's checkpoints on that device. Follow [Updating Doki](Updating-Doki.md) to keep them.

## Automatic checkpoints and the background

**Library keeps saying "New progress, checkpoint coming soon".** The save is still changing, for example because the game is still running and saving. Doki waits until it has stayed the same for 30 seconds.

**No checkpoints while I play on Android.** Check **Devices → Keep working in the background** is on, and that the notification is showing. Some devices stop background apps to save battery; allow Doki to run in the background in Android's battery settings.

**No notifications on Windows.** They come from the tray icon, so **Keep Doki in the tray** must be on.

**No tray icon on Linux.** Plain GNOME needs the AppIndicator extension to show tray icons; KDE (SteamOS and Armada desktop mode) shows them out of the box. Without a tray, closing Doki quits it. SteamOS Game Mode never shows a tray.

**Linux: Doki cannot open a save folder.** The Flatpak can reach your home folder, other Flatpak emulators' data (`~/.var/app`) and drives under `/run/media`, `/media` and `/mnt`. A save folder elsewhere needs `flatpak override --user --filesystem=/that/folder dev.doki.doki`, then restart Doki.

## Android folders

- Choose the folder that holds the game's save files, not the emulator's whole data folder.
- If the emulator stores saves in a place the folder picker cannot reach, Doki cannot use it yet. This is being investigated per emulator.
- Android support is experimental until verified on real hardware. Keep copies of your saves.

## Reporting a problem

Open an [issue](https://github.com/drluciano/doki/issues) with your device, OS version, emulator and version, what you did, and the text under **Details**. Do not attach real save files or ROMs.
