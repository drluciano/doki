# The `doki` command

The command line drives the same engine the app uses, with the same words: Checkpoint, Separate progress, Incoming progress, Use this save, Restore a checkpoint.

The vault lives in `--vault <folder>`, else `$DOKI_VAULT`, else `%APPDATA%\Doki` on Windows, or `$XDG_DATA_HOME/doki` (normally `~/.local/share/doki`) elsewhere. The app keeps its own, separate vault in its app data folder. Keep it outside every save folder; Doki refuses overlapping folders.

## Everyday handoff

On the PC:

```sh
doki init --name PC
doki add "Emerald" "D:\Emulators\mGBA\saves"
# play, close the game
doki capture "Emerald" --closed
doki send "Emerald" -o emerald.dokipkg
```

Move `emerald.dokipkg` to the other device any way you like, or pair the devices and sync directly instead (`doki devices`, see [peer.md](peer.md)). There:

```sh
doki init --name Odin
doki receive emerald.dokipkg          # changes no saves
doki add "Emerald" <save folder> --link <id printed by receive>   # first time only
doki incoming
doki use "Emerald" <checkpoint>       # shows the review, asks, then backs up and installs
```

`use` re-reads your save right before replacing it. If it changed since the review, Doki captures the change and asks you to review again.

## When both devices played

`doki incoming` says "You played on both devices." `doki use` shows the checkpoint both continued from. Using one keeps the other as separate progress in `doki history`, from where `doki restore` brings it back. `doki dismiss` keeps your save instead and records that choice, so neither device asks again. `doki keep` leaves the incoming progress in history without deciding.

## If an install is interrupted

`doki games` shows "Needs recovery". `doki recover <game>` lists each file as new save, previous save, or changed by something else. Then run `doki recover <game> --finish` or `--put-back`. Files changed by something else are captured into history first. Until recovery finishes, Doki refuses to capture or install for that game; do not play it.

## Commands

| Command | Does | Changes save files? |
|---|---|---|
| `init --name N` | Create this device's vault | No |
| `add NAME FOLDER [--link ID]` | Set up a game's save folder | No |
| `games` | Games, their state and what waits for you | No |
| `capture GAME [--closed] [--note T]` | New checkpoint of the current save | No |
| `history GAME` | Checkpoints, oldest first by ancestry | No |
| `send GAME… -o FILE` | Handoff file with your current saves and history | No |
| `receive FILE` | Verify and store a handoff file | No |
| `edit GAME [--name N] [--folder F] [--auto on\|off]` | Rename, change folder, automatic checkpoints | No |
| `remove GAME` | Remove from this device; folder and checkpoints kept | No |
| `watch [--settle S]` | Save a checkpoint whenever a save stops changing | No |
| `incoming` | Incoming progress waiting for review | No |
| `use GAME CP [--yes]` | Review, back up, install, verify | **Yes**, after backup |
| `restore GAME CP [--yes]` | Same as use, for a checkpoint from history | **Yes**, after backup |
| `keep GAME CP` | Keep incoming progress alongside | No |
| `dismiss GAME CP` | Keep your save; record the choice | No |
| `recover GAME [--finish \| --put-back]` | Resolve an interrupted install | **Yes** |
| `pin` / `unpin GAME CP` | Protect a checkpoint from future cleanup | No |
| `delete-checkpoint GAME CP [--yes]` | Delete a checkpoint's files from this device; it keeps its place in history | No |
| `retention [DAYS\|forever] [--yes]` | Show or set how long checkpoints are kept here; previews what goes | No |
| `forget ID [--yes]` / `unforget ID` | Delete a removed game and its checkpoints from this device / stop ignoring it | No |
| `export-files GAME CP FOLDER` | Copy a checkpoint's files out of Doki | No (writes FOLDER) |
| `clean` | Remove temporary leftovers, never checkpoints | No |
| `devices invite` | Show pairing codes and answer devices that join | No |
| `devices join CODE` | Join the group of the device showing CODE (long or short), then sync | No |
| `devices sync` | Sync directly with every paired device online now | No |
| `devices serve` | Keep answering your other devices until Ctrl+C | No |
| `devices list` / `devices remove DEVICE` | Show paired devices / remove one from the whole group | No |
| `relay create URL` | Start a space on a relay and pair this device | No |
| `relay join CODE` | Pair this device with the code from another | No |
| `relay code` / `relay forget` | Show the pairing code / unpair | No |
| `sync` | Send changed saves to the relay and receive others' | No |

Checkpoint arguments accept any unique prefix of the id shown.

Exit codes: `0` success, `1` error with your save unchanged, `3` error where the save may be mixed and `doki recover` is needed.

Handoff files are not encrypted yet. Move them over channels you trust.
