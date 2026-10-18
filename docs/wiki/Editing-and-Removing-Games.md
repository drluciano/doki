# Editing and removing games

Open a game in Library and open its menu.

## Edit game

**Edit game** changes:

- the game's **title**,
- its **save folder**, for example after moving the emulator,
- whether **Automatic checkpoints** are on.

Editing never reads or changes your save files. Changing the folder copies and deletes nothing: from then on, Doki checkpoints whatever the new folder holds, as the next step in the game's history.

A game that is part of a [whole system](Tracking-a-Whole-Emulator.md) takes its folder from the system; change the system instead.

## Remove from this device

**Remove from this device** stops Doki watching the game here.

- Its save files stay where they are.
- Every checkpoint is kept.
- Your other devices keep the game.
- It is listed under **On your other devices** in Library, so you can set it up again later with its history.

Doki will not remove a game while a change to its save is waiting to be finished or put back. See [Recovering an interrupted change](Recovering-an-Interrupted-Change.md).

## Delete a removed game's checkpoints

To free the space a removed game uses, open the menu (⋮) on it under **On your other devices** and choose **Delete from this device**. This deletes its checkpoints on this device and stops listing it here.

For a game that was only ever set up on another device, the same menu says **Don't set up on this device**.

Either way, the save folder is never touched and your other devices keep their copies. See also [Freeing up space](Freeing-Up-Space.md).

## Command line

```sh
doki edit "Emerald" --name "Pokémon Emerald" --folder "D:\new\saves" --auto off
doki remove "Emerald"
doki forget <id>       # delete a removed game's checkpoints here
doki unforget <id>     # list it again
```
