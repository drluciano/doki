# Updating Doki

## Windows

Download the newer `doki-windows` test build, quit Doki (right-click the tray icon and choose **Quit**), and replace the old folder with the new one. Your checkpoints and settings are kept, because they live in Doki's own data folder, not next to `doki.exe`.

## Android

> **Known issue ([#82](https://github.com/drluciano/doki/issues/82)):** each Android test build is signed with a different key, so Android refuses to install a new one over the old one ("App not installed as it conflicts with an existing package"). Until this is fixed, updating means **uninstalling first, which deletes Doki's data on that device**: its checkpoints, paired devices and settings.

Your emulator's save files are not affected by uninstalling Doki. Only Doki's own copies are.

To update without losing history:

1. Make sure the handheld's latest progress has a checkpoint (Library shows **Protected by a checkpoint**).
2. Open Doki on another paired device, such as your PC, and choose **Sync now** on both, so the PC has every checkpoint from the handheld.
3. Uninstall Doki on the handheld and install the new APK.
4. Set it up again: name the device, and pair it with the PC using a new code.
5. Under Library → **On your other devices** (or **Systems on your other devices**), set up each game again, choosing the same save folders. Its checkpoints come back from the PC, and anything newer than the save on the handheld waits in Incoming for you.

If you have no other paired device, uninstalling loses the handheld's checkpoint history. Keep a copy of your save folders before you update.

Once a fixed signing key ships, one last uninstall will be needed, and updates after that will install over the old version with everything kept.
