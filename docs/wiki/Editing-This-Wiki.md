# Editing this wiki

The wiki's source lives in the main repository under [`docs/wiki`](https://github.com/drluciano/doki/tree/main/docs/wiki), so it is reviewed in pull requests like code. The **Wiki** workflow (`.github/workflows/wiki.yml`) publishes it whenever those files change on `main`. Edits made directly on the wiki are overwritten by the next publish.

- One Markdown file per page. The file name is the page name: `Getting-Started.md` becomes *Getting Started*.
- Link to other pages by file name, such as `[Pairing](Pairing-Devices.md)`. That works when browsing the repository, and the publish step removes the `.md` so it works on the wiki too.
- Link to repository files with full `https://github.com/drluciano/doki/blob/main/…` URLs.
- `_Sidebar.md` and `_Footer.md` appear on every page.
- Use the words the app uses: Checkpoint, Incoming progress, Use this save, Restore a checkpoint, Separate progress. Keep internal terms (vault, head, bundle) out of player pages.
- Do not claim a platform or emulator works until it has been verified on real hardware.

