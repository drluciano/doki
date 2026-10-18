# Doki agent instructions

Read docs/workflow.md and the selected issue. Consult docs/engineering.md, docs/spec/safety-contract.md and role guides only as relevant.

- Owner direction of 2026-10-18 in docs/workflow.md takes precedence over the rules below: build the MVP in incremental draft PRs without waiting on per-step approval; the owner merges.

- Default to one primary agent performing PM, design, engineering and coordination. Role names are responsibilities, not persistent workers. Use one independent PR Reviewer for a substantial ready PR; no other delegation unless explicitly requested or concretely justified.
- The one-minute heartbeat is paused. Do not restart schedules, poll GitHub periodically or wake idle roles. Resume on owner input or actual results available in an active run.
- Keep one engineering story active through owner acceptance, even after partial PR merges. Only the owner accepts/completes stories. Do not weaken criteria or auto-close issues.
- Batch design/code/docs/checks into meaningful PRs. Keep intermediate notes local; avoid planning-only micro-PRs and repetitive handoffs.
- Reviewer may mark ready and merge clean, technically passing PRs under standing owner authorization and repository rules. Technical PASS/merge is not human story acceptance. Shared identity permits readiness/merge; formal self-APPROVE is a different action.
- Run relevant meaningful checks, preferably deterministic CI. Inspect changed state only; do not repeat unchanged checks/reviews or interpret missing evidence as passing.
- Document public APIs, configuration, schemas, errors, state transitions and supported user workflows. Explain safety-critical ordering; keep comments useful.
- Receiving a save never installs it. Preserve verified pre-apply backup, baseline recheck, immutable history and journaled recovery. No generic binary-merge or untested platform-support claims.
- Keep personal saves, ROMs, credentials and identifying diagnostics out of the repository; use synthetic/licensed fixtures.
- Preserve current work and compact handoff notes. Report only meaningful progress, failures or required owner actions; no repeated unchanged blocker reports.
