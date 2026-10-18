# Automatic checkpoints

You do not have to remember to make checkpoints. Doki watches each save folder and saves one on its own after you play.

## How it works

1. Doki looks at each save folder about every 15 seconds.
2. When a save changes, Library shows **New progress, checkpoint coming soon**.
3. Once the save has stayed the same for **30 seconds**, Doki makes a checkpoint, checking every file as it copies. If the files change again while it copies, it waits and tries later.

Waiting for the save to settle means Doki does not catch a save halfway through being written. Automatic checkpoints are on for every game by default.

Nothing is ever replaced by an automatic checkpoint. It only copies your save into Doki.

## Checkpoint now

To save one right away (for example, before a hard boss), close the game in the emulator, open it in Library and choose **Checkpoint now** (the button says **Create checkpoint** when automatic checkpoints are off). You can add a label, such as "Before the gym".

## Keep it running while you play

Doki can only watch while it is running:

| Platform | What happens |
|---|---|
| Android | A background watcher keeps checking and syncing while you play in an emulator, with a notification showing. Turn it off under **Devices → Keep working in the background**, or from the notification. With it off, Doki only works while it is open. |
| Windows | Closing or minimizing Doki hides it in the tray (the notification area), where it keeps checking and syncing. Click the icon to open it, or right-click for **Sync now** and **Quit**. Turn this off under **Devices → Keep Doki in the tray**; then closing Doki quits it. **Start Doki with Windows** opens it in the tray when you sign in. |
| Linux | Doki watches while it is open (minimized is fine). |

## Turn it off for a game

Open the game, open its menu, choose **Edit game** and turn off **Automatic checkpoints**. Then use **Create checkpoint** yourself after playing.

You might do this for a game that writes its save in several pieces with long pauses between them, so a checkpoint is never taken between pieces. Even then, nothing is lost: every checkpoint stays in History, and nothing is used without your review.

## Notifications

Doki can tell you when it saved a checkpoint, when a save starts changing, and when progress arrives from another device. See [Settings and notifications](Settings-and-Notifications.md).
