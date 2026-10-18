# Recovering an interrupted change

Replacing a save can involve several files. If Doki is interrupted part of the way through (power loss, the app being closed, storage removed), the save folder may hold some old files and some new ones. Doki keeps a record of every change while it happens, so it always knows how to finish or undo it.

## What you will see

The game shows **A change to this save was interrupted** and **Needs your choice: finish or put back**.

> Do not play the game until you have chosen. The save may be part old, part new.

Doki will not create checkpoints or install anything else for that game until this is resolved. Both versions are safe inside Doki.

## Your choices

- **Finish the change** puts the save you chose in place and checks it. You will see **Finished. The chosen save is now in place.**
- **Put my save back** returns the save from before the change and checks it. You will see **Put back. Your save from before the change is in place.**

Doki never decides for you. The one exception: if every file is already in place, it says **All files are already in place. Finish to record it.**

If something else wrote to the folder in the meantime (for example the emulator was opened), Doki saves those files as a checkpoint first, so nothing is lost either way.

Recovery can itself be interrupted and resumed. Just choose again.

## If recovery cannot finish

You will see **Recovery could not finish**. The game stays protected and blocked. Try again once the cause is fixed (reconnect the storage, free space, allow folder access). In the meantime you can copy any checkpoint's files out (command line: `doki export-files`).

## Command line

```sh
doki games                       # shows "Needs recovery"
doki recover "Emerald"           # lists each file: new save, previous save, or changed by something else
doki recover "Emerald" --finish  # or --put-back
```

A `doki` command that exits with code `3` means the save may be mixed and `doki recover` is needed.
