# Freeing up space

Doki stores each distinct file once, so checkpoints of a save that barely changed take little space. By default it keeps every checkpoint forever. When you want space back, you can delete checkpoints yourself or choose how long to keep them.

Deleting only ever affects **this device**. Your save folder is never touched, and your other devices keep their copies.

## Delete one checkpoint

1. Open the game and find the checkpoint in its history.
2. Open the checkpoint's menu and choose **Delete from this device**, then confirm.

You cannot delete the current save, a pinned checkpoint (unpin it first) or incoming progress still waiting for your decision.

A deleted checkpoint cannot be restored on this device afterwards. Doki still remembers where it sat in the game's history, so how other saves relate to each other is unaffected.

## Choose how long to keep checkpoints

In the **History** tab, choose **Keep checkpoints for**:

- **Forever** (the default)
- 1 year, 90 days, 30 days or 7 days

Before a shorter time takes effect, Doki tells you how many checkpoints would be deleted, and you confirm.

Whatever you choose, Doki always keeps, for each game:

- the 10 newest checkpoints,
- the current save,
- pinned checkpoints,
- separate progress and the choices you made,
- incoming progress waiting for you,
- anything an interrupted change still needs.

Doki never deletes checkpoints because a folder is missing, another device is offline or the disk is full. Choose **Pin (keep forever)** on any checkpoint you want to keep no matter what.

## Delete a removed game

A game you removed is listed under **On your other devices**. Its menu has **Delete from this device**, which deletes its checkpoints here. See [Editing and removing games](Editing-and-Removing-Games.md).

## Command line

```sh
doki delete-checkpoint "Emerald" <checkpoint>
doki retention                # show the current setting
doki retention 90             # keep 90 days; previews what goes and asks
doki retention forever
doki clean                    # remove temporary leftovers, never checkpoints
```
