# D01 #31 — proposed visual concepts

Status: design preparation only, **not owner accepted**. No production functionality or engineering completion is claimed. F01 remains the sole engineering story. These original static concepts do not expand or replace the issue's acceptance criteria.

Open `index.html` locally to review four scalable sheets. Individual SVGs are directly viewable in a browser. Each of the first three sheets shows a desktop layout and a compact 406-logical-pixel-wide Android handheld concept; neither is a claim of tested device support. All games, times, sizes, and device names are sample data.

1. `01-library.svg`: warm surfaces, original geometric artwork fallbacks, one clear incoming-save action, and local protection status distinguished from sharing.
2. `02-review.svg`: named continuations, common ancestor, unchanged playable-save status, and explicit backup/apply action. This does not promise binary merging or an ability to read game progress.
3. `03-history.svg`: checkpoint identity, previous progress preservation, and explicit restore confirmation.
4. `04-components.svg`: palette, typography, spacing, button variants, focus, disabled state, textual badges, permission error, progress, and motion annotations.

## Reference and provenance

Owner reference: https://github.com/inssekt/CocoonFE. Reviewed its README during initial direction research: game-library organization, adaptable layout, themes, and setup wizard inform the broad approach. These concepts do not reproduce its exact visuals. Every illustration here consists of original simple SVG shapes. No reference artwork, game imagery, proprietary font, icon pack, or screenshot is included. Typeface is the device's Segoe UI/Arial/system sans; glyph availability must be verified if chosen for production.

## Reusable states and responsive rules

Buttons require default, hover, focus-visible, pressed, disabled, and busy states in a later component implementation. This gallery visually represents default, focus, disabled, and primary/secondary variants; it does not claim a working interaction implementation. Busy actions retain a name and a textual step, prevent duplicate submission for the same bundle, and surface persistent final results. Cards require default, focus, selected, empty, loading, and error designs before their engineering handoff. Missing/failed artwork falls back to the original geometric illustration. Status badges always include text and a symbol.

At compact widths stack comparison cards; wide layouts may place them side by side while preserving source-first reading order. At 320 logical pixels and large text, use a scrolling single column, wrap action labels, and preserve full safety copy. Bottom navigation becomes a rail when content space permits. Fixed sheet coordinates are illustrative, not responsive CSS or proof that text scaling passes. The compact history sheet demonstrates a confirmation surface; production must make it scroll with long copy and constrain focus while open.

## Motion and accessibility annotations

No animation runs in these artifacts. Proposed tokens: 160 ms control feedback and 220 ms panels. Follow the OS reduced-motion preference; remove movement/bounce and retain status text and visible results. Avoid indefinite decorative animation and invented transfer progress.

Production handoff must include 48×48 logical-pixel touch targets, complete keyboard/controller focus paths, screen-reader action names with device/checkpoint context, and one completion announcement per operation. The SVG sheet has a descriptive title but its internal text and shapes are a static visual artifact, not a screen-reader-ready application. Destructive confirmation needs labelled context, initial non-destructive focus, Escape/back behavior before installation, and focus return. Verify scaling, high contrast, focus visibility, keyboard behavior, and assistive technology on actual implementations; no compliance or device pass is claimed here.

## Review prompts for owner

- Does this soft, warm, rounded direction feel right for Doki?
- Are incoming progress and the current playable save easy to distinguish?
- Does the backup-and-restore copy make the outcome clear?
- Which density feels comfortable on your handheld?

Reviewing these concepts does not accept implementation or change F01 scope. Exact supported emulator/storage behavior will follow feasibility evidence.

## Generation

Run `python generate.py` from this directory (Python standard library only). It recreates all SVG sheets and `index.html`. Source uses original paths and static strings; it touches no emulator data and performs no networking.
