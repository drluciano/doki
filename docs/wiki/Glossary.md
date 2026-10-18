# Glossary

Doki uses a few plain words for its ideas. The command line and the app use the same ones.

| Word | Meaning |
|---|---|
| **Save** / **current save** | The save files your emulator uses right now on this device. |
| **Checkpoint** | An exact, verified copy of a game's save at one moment. Checkpoints never change after they are made. |
| **History** | All the checkpoints of a game on this device, including those from other devices and backups. |
| **Incoming progress** | A checkpoint from another device, waiting for you to review it. It has not changed anything yet. |
| **System** | An emulator's whole save folder added once. Every game in it (each subfolder, or each set of save files sharing a name) is tracked as its own game. See [Tracking a whole emulator](Tracking-a-Whole-Emulator.md). |
| **Use this save** | Replace your current save with incoming progress, after a verified backup. |
| **Restore a checkpoint** | Replace your current save with a checkpoint from History, after a verified backup. |
| **Keep alongside** | Put incoming progress in History without using it, and stop asking about it. |
| **Keep my save** | Keep your current save and record that choice, so no device asks again. (`doki dismiss` on the command line.) |
| **Separate progress** | A version you set aside, or one nobody continued from. Still in History, still restorable. |
| **You played on both devices** | Both devices continued from the same checkpoint in different directions. You choose which to continue; both are kept. |
| **Backup before a change** | The checkpoint Doki verified before replacing your save. |
| **Pin** | Mark a checkpoint to keep forever, whatever future cleanup does. |
| **Pairing** | Connecting a new device to your devices with a short or long code. |
| **Short code** | An `ABCD-2345` code for pairing on the same Wi-Fi. |
| **Long code** | A `DOKIP1:…` code for pairing from any network. |
| **Sync** | Exchanging checkpoints with your other devices. Never changes a save. |
| **Relay** | An optional server you run yourself that holds encrypted progress until your other devices fetch it. |
| **Handoff file** | A `.dokipkg` file carrying a game's checkpoints, for moving saves by hand. |
| **Interrupted change** | A replacement that stopped partway. Resolved by **Finish the change** or **Put my save back**. |
| **Vault** | Where Doki keeps checkpoints on a device. You never need to open it. |
