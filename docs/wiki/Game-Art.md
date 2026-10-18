# Game art

The library shows each game as a cover, and each game's page has a banner across the top. Without art, Doki shows a soft placeholder with the game's initials.

Art is decoration only: it is kept on this device, is not synced, and never touches your saves or checkpoints.

## Find art on SteamGridDB

[SteamGridDB](https://www.steamgriddb.com) is a free, community-made library of game art used by many emulation front ends. Its API needs a personal key, which you get for free:

1. Sign in at steamgriddb.com, open **Preferences**, then **API**, and copy your key.
2. In Doki, open **Library**, choose **Game art** (the palette button), paste the key and choose **Save key**. Doki checks the key with one search before keeping it.

With **Find art automatically** on, Doki looks up a cover and banner once for every game without a cover, including games found in a [whole system](Tracking-a-Whole-Emulator.md). It searches by the game's name without region tags such as "(USA)". Names it cannot search, such as a Switch title id folder, keep the placeholder until you rename the game or pick art yourself. **Find art for games without it** runs the lookup again.

The key is stored only on this device, in Doki's own app folder. It is sent only to SteamGridDB.

## Choose art yourself

Open a game, open the menu and choose **Change art**:

- **Choose image** under Cover or Banner picks a PNG, JPEG, WebP or GIF file.
- **Find on SteamGridDB** searches by name; pick a result, then tap any cover or banner to use it.
- **Remove** goes back to the placeholder. Doki does not fetch art again for that game on its own.

## Cover shape

**Tall** covers (2:3, like a game case) suit most libraries. **Wide** covers (SteamGridDB's 920×430 capsules) suit consoles with landscape boxes. Changing the shape looks up art again for games that used the old shape the next time Doki checks.
