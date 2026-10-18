# Doki product design proposal

Status: **Proposed; awaiting owner acceptance.** This document supplies design direction and story-ready criteria. It is not an approved visual specification or permission to implement later engineering stories.

## Reference and intent

The owner selected [CocoonFE](https://github.com/inssekt/CocoonFE). Its README describes a customizable game library, adaptable grid layouts, modular themes, and a setup wizard. These support a game-first, personable direction. The palette, dimensions, and interactions below are original proposals, not claims about Cocoon's exact appearance. Review actual visual mockups with the owner before treating appearance as accepted. Do not copy assets, trademarks, or code without checking their licenses.

Doki should feel like a friendly place to keep progress. Cute styling must coexist with precise descriptions of what will happen to files. The primary promise is: receiving a checkpoint never silently changes the playable save.

## Proposed design system

- Surfaces: warm cream background (#FAF7F2), white cards, ink text (#272335), muted text (#655D70), lavender accent (#7354A3), and mint secondary accent. Validate final token combinations for contrast before release.
- Semantic states: success, attention, and error always include a word and icon; color alone never conveys safety. Reserve error styling for failed operations and confirmed destructive actions.
- Shape: 16–24 logical-pixel card corners, rounded buttons, restrained soft shadows. Avoid translucent text surfaces unless contrast remains stable over every background.
- Spacing: 4-unit base, with 8/12/16/24/32 steps. Content width caps around 1200 logical pixels; paragraphs around 65 characters.
- Type: licensed bundled font or native system stack, friendly headings and highly legible body. Body default 16 logical pixels, metadata at least 14; accommodate platform scaling without clipped controls.
- Components: game card, device chip, checkpoint card, status banner, proposal comparison, chronological timeline, operation progress, inline validation, empty state, confirmation sheet, and persistent result panel.
- Copy follows the owner's vocabulary: “Checkpoint” (commit), “Separate progress” (branch), “Incoming progress” (proposal), “Use this save” (checkout) and “Restore a checkpoint” (revert). Divergence reads “You played on both devices. Choose which save to continue. Both versions will stay in your history.” Never imply binary saves can be merged unless a compatible inspector actually supports that operation.
- Game imagery is optional. Initials and platform symbols provide attractive fallbacks; no onboarding requirement for game-art services.

## Navigation and responsive behavior

The main screens are Library, Incoming, History and Devices. Compact windows use bottom navigation for Library, Incoming, History and Devices, with Settings in the top-level menu; History opens for the selected game. Medium and wide windows use a navigation rail; wide game detail supports a history column beside checkpoint details. Breakpoints should follow available content space, not OS identity; test narrow Windows windows and landscape Android handhelds.

Library cards show title, emulator/profile, latest protected checkpoint, and textual status. Selecting a card opens game details with current playable save, capture button, incoming progress, and history. Incoming shows only progress waiting for a decision. Devices show last successful exchange and connection status separately. Settings expose retention, vault location, accessibility, diagnostics, and transport settings.

At 320 logical pixels and 200% text scaling, use a single scrolling column. Never shrink safety text or hide the destination behind an overflow menu. In comparison view, stack save cards with explicit device headings; do not rely on left/right positions. Large screens may show them side by side. Preserve reading and focus order.

## Interaction and motion

- Primary actions describe the result: “Create checkpoint” and “Back up and use this save.” An incoming checkpoint notification opens review; it never applies the save.
- Transition durations: approximately 120–180 ms for control feedback and 180–260 ms for panels. Use modest fade/position changes. Motion demonstrates completion or movement between states, never simulates progress.
- Progress reflects measurable steps (reading, verifying, saving); unknown duration uses a labeled indeterminate indicator. Do not manufacture percentages.
- Reduced-motion setting follows the OS by default and removes translation, bounce, and celebration. Success remains apparent through text and icon.
- Cancel is available before file replacement begins. Once a recoverable installation begins, explain why closing the app does not cancel the operation; show recovery state on reopening.
- Escape/back closes transient UI first. Never dismiss a nonrecoverable error without leaving a way to find its details.

### Motion as built (app/lib/motion.dart, app/lib/widgets/focus_halo.dart)

- **Focus border.** One rounded lavender border, drawn once above the whole app, marks the control you are about to use. It glides on a slightly springy curve (about 250 ms to settle) between controls as the mouse hovers, as Tab, arrow keys or a controller D-pad move focus, and under a finger on touch screens. Pressing squeezes it 2 px. It tracks any enabled tappable control and any focused control, so new screens get it without extra code. Keyboard focus is shown only while keys drive focus. It stays inside the scrolling list it belongs to and lets go when a screen is covered.
- **Screens.** Pushed screens fade in while rising slightly (260 ms in, 220 ms back); the screen underneath settles to 97%. Tabs fade through: the old tab fades out, then the new one fades in from 98%. Every tab keeps its state.
- **Arrivals.** Empty states fade and rise in; status banners blend between kinds.
- **Reduced motion.** When the OS asks for it, the border jumps instead of gliding or popping, and screens and tabs only fade. High contrast draws the border in ink at 3.5 px.
- The border has no semantics and takes no input; screen readers and hit testing are unchanged.

## Accessibility acceptance baseline

Every action supports keyboard and touch, with predictable focus order and visible focus. Target minimum 48×48 logical pixels for touch controls. Controller support uses directional navigation with no focus traps. Screen readers announce control names, checkpoint device, capture time, verification state, and operation completion without repeating every progress tick. State changes do not steal focus unless the current control disappears. Body text aims for 4.5:1 contrast and meaningful graphics/focus indicators 3:1. Validate against the applicable WCAG 2.2 criteria and platform accessibility tooling. Test high contrast, text scaling, reduced motion, keyboard-only use, screen readers, and long translated strings. Dates use local formatting with absolute timestamp available; relative dates must not be the sole identity of a checkpoint.

## Tutorial

Use isolated sample data and clearly label every page “Practice saves.” Steps: create a checkpoint, receive a checkpoint, inspect two divergent paths, choose a continuation, and restore the other path. Show the preserved history after each action. Back, Skip tutorial, and Replay tutorial remain available; a skipped tutorial never blocks the app. No tutorial operation may touch a user's configured save folder. Completion explains that local protection, successful transfer, and independent backup are distinct conditions.

## Screen and state inventory

| Context | Required presentation and action |
| --- | --- |
| Fresh install | Welcome; practice tutorial; connect a save folder; no sign-in required for local use |
| Empty library | Explain folder connection and show Add game action |
| Folder picker canceled | Return to setup without error; retain entered values |
| Folder inaccessible/revoked | “Folder access needs updating”; reconnect; retain vault/history |
| Folder missing / SD card removed | “Save location unavailable”; retry/reconnect; never infer deletion |
| Empty folder / unrecognized format | Explain result and supported profile options; no false checkpoint success |
| Active or changing files | “Waiting for the save to finish”; retry and clear recovery guidance |
| Offline | Local checkpoint remains usable; pending transfer count; last successful exchange |
| Device unreachable | Retry/pairing help; no suggestion that local progress was lost |
| Authentication expired / device revoked | Reconnect or pair; no automatic credential disclosure in diagnostics |
| Storage full | Identify vault vs destination; offer storage settings; preserve current save |
| Integrity failure | Quarantine/refetch affected data; block apply; show recoverable explanation |
| Compatibility unknown/mismatch | Name emulator/profile difference; block known incompatibility and explain unknown status |
| Divergence | Two named continuations plus common ancestor; choose one and preserve both |
| Local files changed after review | Capture new progress and refresh comparison; require new review |
| Apply in progress | Backup → stage → install → verify; actual operation state; no competing apply |
| Interrupted installation | Persistent recovery screen reporting restored/continued outcome, with details |
| Backup or recovery failure | Block replacement; persistent action guide and diagnostics export |
| Successful capture | New verified checkpoint in history; distinguish unchanged-save deduplication |
| Successful apply | Current playable checkpoint identified; restore previous checkpoint action |
| Retention approaching limit | Explain policy and pinning before cleanup; no surprise history removal |
| Remote deletion proposal | Explicit scope and consequences; no implicit deletion from missing folders |
| No proposals | Calm empty state with last checked time, refresh, and offline qualifier when relevant |

Errors must include what happened, whether playable files changed, what remains protected, and a concrete next action. Raw paths and technical details live in expandable details; avoid sensitive contents in telemetry/export. Unsupported capabilities are explained, never represented by controls that appear functional.

## First story: onboarding to local use

User story: As a new player, I can understand Doki's safety promise and open the library without configuring a server.

Flow: Welcome → optional practice tutorial → local device name → empty library. Save-folder registration is a separate story so onboarding does not assume Android access is solved.

Welcome wireframe: small original Doki emblem; heading “Keep your progress, on your terms”; three short statements (“Keep checkpoints,” “Review incoming saves,” “Restore earlier progress”); primary “Try practice saves”; secondary “Go to my library.” Device-name entry suggests a local name and clearly labels it editable; never display hardware identifiers as a user-friendly name.

Acceptance: back/resume preserves completed non-sensitive steps; skip and replay work; no account/network permission required; practice data is isolated; keyboard/screen-reader/text-scaling behavior passes; no promise of cross-device sync before a device/transport is configured. Owner review includes compact and wide mockups and a tutorial recording. This document alone does not satisfy visual review.

## First story: register a save folder

Flow: Add game → choose supported emulator/profile → enter game title → platform folder picker → inspect selected files → confirm mapping. Explain Android limitations before presenting unsupported locations; the native picker remains the access authority.

Review card shows game, profile, displayed folder location, selected file bundle, and estimated size. “Connect folder” only registers access; it does not alter saves. Warn against placing the vault inside the watched save folder. Duplicate mappings identify the existing entry. Cancellation retains draft data but no permission is assumed to persist without platform confirmation.

Acceptance: selected location and bundle are visible before confirmation; unsupported/empty/inaccessible locations have actionable states; multi-file bundles are not split silently; native save vs save-state compatibility is explicit. Owner review uses Android and Windows real folder-picker flows with both accessible and denied examples.

## First story: create and inspect a local checkpoint

User story: As a player, I can preserve my current save and see evidence that the checkpoint was captured.

Flow: Game details → Create checkpoint → read stable bundle → verify → write vault object/metadata → history entry. An optional label field defaults to blank and never blocks capture. While running, keep the game title and source folder visible; disable duplicate capture only for the affected bundle. Do not claim an emulator is closed unless integration verifies it.

Result card shows label/fallback name, device, absolute capture time, bundle size, file count, and “Verified copy.” Clarify this means copied bytes verified, not validated game progress. Unchanged data says “No changes since [checkpoint]” and links to it. A checkpoint details sheet exposes manifest/hashes under Details, without making those prerequisites for normal use.

Acceptance: source files are unchanged; cancellation/failure never produces a success state; partial objects are not selectable as checkpoints; unstable files receive bounded retry and guidance; full-disk/read-error states preserve history and source; refresh/restart retains completed checkpoint; screen-reader completion is announced once. Review evidence includes successful capture, unchanged capture, unavailable source, and failed-write scenarios. Restore/apply receives its own safety-critical story and review.

## Design handoff and approval

A story enters Ready for engineering only with applicable flows, copy, all states, responsive behavior, accessibility expectations, and mockups/prototype references. Technical stories can mark visual design Not applicable with a reason, but still require observable acceptance and error behavior. The designer labels this as a proposal; only the owner can accept completed work. Engineering cannot advance to another story while the current engineering story awaits owner acceptance under the agreed work-in-progress rule.

Each implementation review includes screenshots at agreed compact/wide sizes, an interaction recording where motion matters, accessibility results, and a short manual acceptance script. Owner-requested changes remain on the same story with updated criteria and design revision notes.
