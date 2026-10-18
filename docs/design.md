# Design

How the Doki app looks, reads and behaves, and why. The code is in `app/lib`; colors live in `app/lib/theme.dart`, motion in `app/lib/motion.dart` and `app/lib/widgets/focus_halo.dart`, and user-facing wording in `app/lib/copy.dart` and the screens.

## Intent

Doki should feel like a friendly place to keep your progress: soft, warm and game-first, in the spirit of [CocoonFE](https://github.com/inssekt/CocoonFE)'s game library. The friendliness never gets in the way of accuracy. Every screen that can change a save says exactly what will happen to the files, and the core promise is visible throughout: receiving progress never changes your save until you choose to use it.

Doki's look is its own. No assets, trademarks or code are taken from CocoonFE.

## Words

Doki is built like version control, but never uses version-control words on screen.

| Concept | On screen |
|---|---|
| Commit | **Checkpoint** |
| Branch | **Separate progress** |
| Incoming commit to review | **Incoming progress** |
| Checkout | **Use this save** |
| Revert | **Restore a checkpoint** |
| Divergence | "You played on both devices. Choose which save to continue. Both versions will stay in your history." |

Buttons say what will happen: **Back up and use this save**, **Checkpoint now**, **Put my save back**. Saves are never described as merged; the user always picks one whole version and the other is kept.

## Visual system

- **Colors** (light theme): warm cream page `#FAF7F2`, white cards, ink text `#272335`, muted text `#655D70`, lavender accent `#7354A3`, mint for "protected", amber for "needs attention", red only for failures and confirmed destructive actions. Dark and OLED themes use soft, low-glare equivalents; OLED puts true black behind cards. All three are in `DokiPalette`.
- **State is never color alone.** Success, attention and error always come with a word and an icon.
- **Shape**: rounded cards (16 to 24 logical pixels), pill buttons, soft shadows, no text over translucent surfaces.
- **Spacing**: multiples of 4 (8, 12, 16, 24, 32). Content is capped at about 900 logical pixels wide.
- **Type**: the platform's system font. Body text 16 logical pixels, metadata at least 14, and layouts survive large system text sizes without clipping.
- **Game art** is optional. Without a cover, a soft tile with the game's initials stands in. Art comes from SteamGridDB with the user's own key, or from images they pick, and is stored on the device only.

## Navigation

Five tabs: **Library**, **Incoming** (with a badge counting what waits), **History**, **Devices** and **Settings**. Below 720 logical pixels wide they sit in a bottom bar; wider windows use a side rail. Every tab keeps its state when you switch away.

- **Library** shows games as covers grouped by system, with a status line for each (protected, new progress, needs attention), plus games and systems set up on other devices.
- A **game page** has a banner, the current save, **Checkpoint now**, and the game's history.
- **Incoming** lists only progress waiting for a decision. Opening one shows the review: your save and the incoming one, named by device, never by left or right position alone.
- **Devices** handles pairing, sync, background running, start with Windows, and notification switches.
- **Settings** has theme, border color, and Help and how-tos.

On narrow screens and at large text sizes, comparison cards stack in one column with explicit headings. Safety text is never shrunk or hidden behind a menu.

## Motion

Motion shows where things went; it never fakes progress.

- **Focus border.** One rounded border (lavender unless changed in Settings), drawn once above the whole app, marks the control you are about to use. It glides on a slightly springy curve (about 250 ms) as the mouse hovers, as Tab, the arrow keys or a controller D-pad move focus, and under a finger on touch screens. Pressing squeezes it by 2 px. Its corners match the control it surrounds (pill buttons get a pill, cards their card corner, text fields their field corner, read from the control's own shape or the theme); around text fields it hugs the field, not its helper text. It tracks every enabled tappable control and every focused control, so new screens get it with no extra code. Keyboard focus is shown only while keys drive focus. It stays inside the scrolling list it belongs to, pulls in rather than being cut off at the window edge, and lets go when a screen is covered. It has no semantics and takes no input, so screen readers and hit testing are unaffected.
- **Screens.** Pushed screens fade in while rising slightly (260 ms in, 220 ms back); the screen underneath settles to 97%. Tabs fade through: the old tab fades out, the new one fades in from 98%.
- **Arrivals.** Empty states fade and rise in; status banners blend between kinds.
- **Splash.** Starts once the vault has loaded and a frame is on screen. The letters arrive over about 1.1 s, the tagline follows, the wordmark holds about 0.6 s, then it fades into the app (480 ms). The Android launch screen before it uses the same page color.
- **Themes** switch instantly rather than cross-fading, so Doki's colors and Material's change together.
- **Reduced motion.** When the system asks for it, the border jumps instead of gliding, screens and tabs only fade, and the splash is shorter and still. With high contrast on, the border is drawn in the text color at 3.5 px.
- Progress shows real steps ("Copying and verifying the save…", "Backing up, copying and verifying…"), never invented percentages.

## Accessibility

- Everything works by keyboard, touch, mouse and controller, with a predictable focus order and no focus traps. Enter, Space or A chooses; Escape or the back button closes the topmost thing first.
- Touch targets are at least 48 × 48 logical pixels.
- Screen readers hear control names, which device a checkpoint came from, when it was saved, whether it is verified, and when an operation finishes, without every progress tick.
- Body text aims for 4.5:1 contrast and focus indicators for 3:1 (WCAG 2.2).
- Dates are shown as absolute local times; a relative time is never the only way to tell checkpoints apart.

## States the app must handle

Every error says what happened, whether the playable save changed, what is still protected, and what to do next. Technical details and paths sit under **Details**. Controls for unsupported actions are not shown as if they work.

| Situation | What the user sees |
|---|---|
| No games yet | What a save folder is, and **Add game** |
| Folder picker cancelled | Back to the form, with what was typed kept |
| Folder access revoked | "Folder access needs updating", with a way to choose the folder again |
| Folder missing or storage removed | "Save location unavailable"; never treated as a deleted save |
| Empty folder | "This folder has no save files"; no checkpoint is made |
| Save still being written | "New progress, checkpoint coming soon" while waiting; "Waiting for the save to finish" if a manual checkpoint catches it moving |
| Other device unreachable | Which devices were reached; nothing suggests local progress was lost |
| Storage full | Whether Doki's storage or the save folder is full |
| Received copy damaged | Install blocked; sync again |
| Both devices played | Both versions named, the checkpoint they share, and both kept |
| Played since the review | The new progress is checkpointed first, and the review starts over |
| Install in progress | Backup, copy and verify steps; no second install at the same time |
| Install interrupted | A persistent "A change to this save was interrupted" until the user chooses **Finish the change** or **Put my save back** |
| Shorter **Keep checkpoints for** | How many checkpoints would go, before anything is deleted |
| Nothing incoming | A calm empty state with when Doki last checked |

## Not built yet

- **Practice tutorial.** Disposable sample saves, clearly labelled "Practice saves", that walk through creating a checkpoint, receiving one, seeing two separate paths, choosing one and restoring the other. It must never touch a real save folder, and can be skipped or replayed.
- **Folder preview.** Before connecting a folder, show the files Doki will treat as the save and their size.
