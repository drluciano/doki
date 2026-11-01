# Installing on Linux

Doki for Linux comes as a Flatpak, built for both kinds of handheld and PC:

| File | For |
|---|---|
| `doki-…-linux-x86_64.flatpak` | Steam Deck (SteamOS), most PCs and x86 handhelds |
| `doki-…-linux-aarch64.flatpak` | ARM handhelds such as the AYN Odin 3 running Armada, and other ARM Linux devices |

> Linux builds are experimental. They pass automated checks, but nobody has restored a save with them on a Steam Deck or an Odin 3 yet, so keep a copy of your saves the first time.

## Install

1. Download the file for your device from the latest release under [Releases](https://github.com/drluciano/doki/releases). (Test builds of every change are under **Artifacts** in the [App workflow runs](https://github.com/drluciano/doki/actions/workflows/app.yml); unzip one to get `doki-linux-….flatpak`.)
2. On SteamOS or Armada, switch to **Desktop Mode** (Steam menu → Power → Switch to Desktop).
3. Open a terminal (Konsole) in the folder with the file and run:

   ```sh
   flatpak install --user doki-0.1.0-linux-x86_64.flatpak     # or …-linux-aarch64.flatpak
   ```

   Flatpak downloads the GNOME runtime Doki uses from Flathub the first time (a few hundred MB). On KDE you can also open the file with Discover instead of using the terminal.
4. Open **Doki** from the application menu, under Utilities.

Then carry on with [Getting started](Getting-Started.md) from step 2.

## Running in the background

Closing Doki's window hides it in the tray (the icons by the clock), where it keeps saving checkpoints and syncing. Click the icon to open it, or open its menu for **Sync now** and **Quit**. The first time, a notification says Doki is still running.

- **Keep Doki in the tray** (under Devices) turns this off; then closing quits.
- **Start Doki when you sign in** opens Doki in the tray each time you sign in to the desktop. Doki asks the desktop for this through the Background portal, which may ask you to allow it; where the desktop has no such portal, Doki adds the entry to `~/.config/autostart` itself.
- Opening Doki again while it is in the tray brings back the running one.
- If your desktop shows no tray (plain GNOME without the AppIndicator extension), closing quits, so Doki never runs where you cannot reach it.

### SteamOS and Armada Game Mode

Game Mode (the Steam interface) has no tray and does not run sign-in entries, and switching from Desktop Mode to Game Mode ends everything running in the desktop, Doki included. So in Game Mode:

- Doki does not run in the background on its own.
- You can add Doki to Steam (in Desktop Mode, Steam → **Add a Non-Steam Game**, pick Doki) and open it from your library. While it is open it saves checkpoints and syncs. Steam runs one game at a time, so open Doki before or after playing, not during.
- Checkpoints still catch up: the next time Doki runs, it checkpoints whatever changed since, and nothing is lost, but you get one checkpoint for that stretch of play rather than one per session.

For steady background checkpoints, play in Desktop Mode with Doki in the tray, or keep Doki open on another paired device so progress reaches it when you next open Doki here.

## What Doki can see

Flatpak apps only reach the folders they ask for. Doki asks for:

| Permission | Why |
|---|---|
| Your home folder | Emulators keep saves in many places under it (`~/.local/share`, `~/.config`, `~/Emulation/saves` for EmuDeck). |
| `~/.var/app` | Saves of emulators installed as Flatpaks (RetroArch, Dolphin and others), which the home permission leaves out. |
| `/run/media`, `/media`, `/mnt` | SD cards and other drives; SteamOS mounts SD cards under `/run/media`. |
| Network | Syncing directly with your other devices and finding them on the same Wi-Fi. |
| Tray | Showing the tray icon. |

Doki asks for whole folders rather than using the portal file picker for each save folder, because it replaces save files in place (renaming a verified copy over each file) and keeps watching them, which picked-file access through the portal does not do reliably. Doki still only reads and writes the save folders you connect, and only changes a save when you choose to (**Use this save**, **Restore this checkpoint**, or finishing an interrupted change). To narrow the access, use Flatseal or `flatpak override --user --nofilesystem=… dev.doki.doki`; Doki reports an error for a save folder it can no longer reach and leaves it unchanged.

## Updating

Download the newer release and install it the same way:

```sh
flatpak install --user doki-0.2.0-linux-x86_64.flatpak     # the newer version's file
```

Flatpak replaces the installed Doki in place. Quit Doki from its tray icon first, or the running copy keeps going until you quit it. Your checkpoints, paired devices and settings are kept: they live in `~/.var/app/dev.doki.doki`, which installing never touches. Installing the very same build twice says it is already installed, which is harmless.

## Uninstalling

```sh
flatpak uninstall --user dev.doki.doki
```

This removes the program and leaves Doki's data (`~/.var/app/dev.doki.doki`) and your game saves alone, so installing again brings back your checkpoints. To remove Doki's data too, add `--delete-data`. If you turned on **Start Doki when you sign in**, turn it off before uninstalling, or delete `~/.config/autostart/dev.doki.doki.desktop` afterwards.
