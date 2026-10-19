# Command line

The `doki` command drives the same engine as the app and uses the same words. The full reference is [docs/cli.md](https://github.com/drluciano/doki/blob/main/docs/cli.md).

## Install

There is no packaged release yet. Build it with Rust 1.91 or newer:

```sh
git clone https://github.com/drluciano/doki
cd doki
cargo build --release
# the binary is target/release/doki
```

The command line keeps its data in `%APPDATA%\Doki` on Windows and `$XDG_DATA_HOME/doki` (normally `~/.local/share/doki`) elsewhere. Override it with `--vault <folder>` or `DOKI_VAULT`. Keep it outside every save folder. The app keeps its own data separately, so checkpoints made with the command line do not appear in the app on the same device unless you point `--vault` at the app's folder.

## Everyday use

```sh
doki init --name PC
doki add "Emerald" "D:\Emulators\mGBA\saves"
# play, then close the game
doki capture "Emerald" --closed --note "Beat Roxanne"
doki history "Emerald"
doki watch                  # make a checkpoint whenever a save stops changing (Ctrl+C to stop)
```

Manage games and storage:

```sh
doki edit "Emerald" --name "Pokémon Emerald" --folder "D:\new\saves" --auto off
doki remove "Emerald"                        # stop tracking here; files and checkpoints kept
doki delete-checkpoint "Emerald" <checkpoint>
doki retention 90                            # keep 90 days; previews what goes and asks
doki forget <id> / doki unforget <id>        # delete a removed game's checkpoints here / list it again
```

None of these change save files. See [Editing and removing games](Editing-and-Removing-Games.md) and [Freeing up space](Freeing-Up-Space.md).

Pair and sync with your other devices:

```sh
doki devices invite         # shows a short and a long code
doki devices join CODE      # on the other device
doki devices sync           # sync with every paired device that is online
doki devices serve          # keep answering other devices until Ctrl+C
```

Review and use what arrived:

```sh
doki incoming
doki use "Emerald" <checkpoint>      # shows the review, asks, then backs up and installs
doki keep "Emerald" <checkpoint>     # keep alongside
doki dismiss "Emerald" <checkpoint>  # keep my save
doki restore "Emerald" <checkpoint>  # go back to any checkpoint in history
```

Checkpoint arguments accept any unique prefix of the id shown.

## Handoff files

To move saves by hand instead of syncing:

```sh
doki send "Emerald" -o emerald.dokipkg        # on the first device
doki receive emerald.dokipkg                   # on the other; changes no saves
doki add "Emerald" <save folder> --link <id>  # first time only; receive prints the id
```

Handoff files are not encrypted yet.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Error; your save is unchanged |
| `3` | Error; the save may be mixed, run `doki recover` |
