# Tracking a whole emulator

If you play many games in one emulator, you do not have to add each game. Add the emulator's save folder once as a **system**, and Doki tracks every game in it, including games you play for the first time later.

## Add a system

1. In **Library**, choose **Add game**.
2. Under **What is in this folder?**, choose **Whole system**.
3. Give it a name, such as "Eden" or "RetroArch GBA".
4. Choose the folder where the emulator keeps every game's save, then **Connect folder**.

Like adding a game, connecting only records where the saves live. Doki does not change anything in the folder.

## What counts as a game

Doki looks at the top level of the folder:

| In the folder | Becomes |
|---|---|
| A subfolder, such as `Zelda/` or `0100F2C0115B6000/` | One game, with everything inside it |
| Loose files sharing a name, such as `Emerald.sav`, `Emerald.ss1` and `Emerald.state.auto` | One game, "Emerald" |

Hidden files and system litter (`Thumbs.db`, `desktop.ini`) are ignored. A game is named after its folder or file name; rename it on its page if you like (for example, when the folder is a title id).

Each game then works exactly like a game you added on its own: automatic checkpoints, **Checkpoint now**, Incoming, History, **Use this save** and **Restore**, with the same backup and recovery. Every new subfolder or save that appears is added the next time Doki checks the folder (about every 15 seconds while Doki is running).

## On your other devices

After a sync, a system you set up on one device appears under **Systems on your other devices** in Library on the others. Tap it and choose the same emulator's save folder there. The two systems are then linked:

- The same game in both folders is the same game in Doki, so its progress keeps in step. Doki matches games by their folder or file name inside the linked system, never by title.
- A game you have only played on the other device is set up here too, and its progress waits in **Incoming**. Choosing **Use this save** creates its folder in the emulator's save folder.
- Nothing is applied on its own. Receiving progress never changes a save.

## Stop tracking a game

Open the game and choose **Remove from this device**. Doki stops tracking it in that system and does not add it back, even though its save is still in the folder. Its files and checkpoints are kept. To bring it back with its history, open the system (tap its name in Library) and choose **Track again**.

## Change or remove a system

Open the system and use the menu:

- **Edit system** renames it, points it at another folder (every game in it moves with it) or turns automatic checkpoints on or off for all its games.
- **Remove from this device** stops tracking the system and all its games here. Save files and checkpoints are kept, and the system is listed under **Systems on your other devices** so you can set it up again.

A folder belongs to either one game or one system, never both.
