# Updating Doki

New versions are published under [Releases](https://github.com/drluciano/doki/releases). Every release installs over the previous one and keeps your checkpoints, paired devices and settings. Update all your devices to the same release when you can.

## Windows

Download the newer `doki-…-windows-setup.exe` and run it. Setup sees the installed Doki, quits it if it is running, and updates it in place. Your checkpoints, paired devices and settings are kept, because they live in Doki's own data folder (`%APPDATA%\dev.doki\doki`), not next to `doki.exe`.

If you used the older unzipped `doki-windows` folder, quit Doki from its tray icon, run the installer, and delete the old folder afterwards. Your data carries over.

### Uninstalling

Use **Settings → Apps → Installed apps → Doki → Uninstall**. This removes the program, its shortcuts and its sign-in entry. It leaves your game saves and Doki's data folder alone, so installing again later brings back your checkpoints and paired devices. To remove those too, delete `%APPDATA%\dev.doki\doki` after uninstalling.

## Linux

Install the newer Flatpak over the old one with `flatpak install --user doki-…-linux-….flatpak`. It updates in place and keeps your checkpoints, paired devices and settings, which live in `~/.var/app/dev.doki.doki`. Uninstalling with `flatpak uninstall --user dev.doki.doki` keeps them too, unless you add `--delete-data`. See [Installing on Linux](Installing-on-Linux.md#updating).

## Android

Open the newer `doki-…-android.apk` on the device and choose **Update**. Releases are signed with Doki's own key, so the new version installs over the old one and keeps your checkpoints, paired devices and settings.

### Moving from a test build made before the first release

Test builds made before the first release ([#82](https://github.com/drluciano/doki/issues/82)) were each signed with a different, throwaway key, so Android refuses to install a release over one of them ("App not installed as it conflicts with an existing package"). Moving to the releases needs **one last uninstall, which deletes Doki's data on that device**: its checkpoints, paired devices and settings. After that, every update installs in place.

Your emulator's save files are not affected by uninstalling Doki. Only Doki's own copies are.

To move without losing history:

1. Make sure the handheld's latest progress has a checkpoint (Library shows **Protected by a checkpoint**).
2. Open Doki on another paired device, such as your PC, and choose **Sync now** on both, so the PC has every checkpoint from the handheld.
3. Uninstall Doki on the handheld and install the release APK.
4. Set it up again: name the device, and pair it with the PC using a new code.
5. Under Library → **On your other devices** (or **Systems on your other devices**), set up each game again, choosing the same save folders. Its checkpoints come back from the PC, and anything newer than the save on the handheld waits in Incoming for you.

If you have no other paired device, uninstalling loses the handheld's checkpoint history. Keep a copy of your save folders before you update.
