# Doki agent instructions

Read docs/workflow.md, docs/engineering.md and the selected issue before acting. Role specifications are in docs/roles/. The owner is the only story acceptance authority.

- Work only within an explicitly selected story. Preserve the one-story engineering limit, including awaiting owner review and accepted-but-unmerged work.
- PM and Designer may prepare independent upcoming stories concurrently. Do not start implementation until dependencies are accepted and merged and design/criteria are ready.
- Never mark owner acceptance, merge, close stories, weaken criteria, or bypass approval controls on the owner's behalf without explicit authorization for that action.
- Make application changes on a story branch and open a PR using Refs #N rather than automatic closing keywords.
- Receiving a save must never install it. Verified pre-apply backup, baseline recheck, immutable history and journaled recovery are required.
- Document every public API, configuration, schema, error and state transition; comment on reasoning and invariants. Keep guides and examples correct; avoid redundant comments that simply repeat code.
- Submit each ready PR to the PR Reviewer before owner review. Reviewer technical approval is separate from owner story acceptance; conflicts return to the Engineer on the same story. Read docs/roles/pr-reviewer.md.
- Run meaningful checks appropriate to the change; report exact commands, results and limits. UI work requires runnable review evidence and visual/accessibility checks.
- Treat save data as private; redact diagnostics. Do not commit game saves, ROMs, credentials or keys. Use synthetic fixtures or appropriately licensed test data.
- Current stack and design are proposals until owner acceptance and feasibility validation. Read architecture decision records before treating any proposal as final.
- No continuous scheduling or autonomous next-story execution is installed by these instructions. Record handoffs durably and stop at the owner review gate.
