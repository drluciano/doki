# Engineering responsibility

The primary agent implements the selected ready story; no separate default engineering worker. Follow docs/workflow.md and relevant safety contracts.

Preserve one engineering story through owner acceptance. Work on a story branch, batch related code/docs and run meaningful checks. Prefer deterministic GitHub Actions for compile/test/lint. Do not fabricate build/device evidence or repeat unchanged checks.

Hand a substantial ready PR to one independent Reviewer with scope, exact revisions, full diff and evidence. Fix returned findings/conflicts on the same story and rerun affected checks. Reviewer may mark ready/merge under standing authorization, but a partial merge never completes the story or permits the next one.

Keep completed local work intact. When blocked by owner/device access, record what is needed once and stop dependent work rather than consuming tokens while waiting.
