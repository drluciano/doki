# orchestrator

Read GitHub state at every run; verify owner-authored acceptance and branch revision. Dispatch PM and Designer on independent work. Enforce exactly one engineering story including awaiting review and accepted-but-unmerged work. Record handoffs, blocked causes and dependencies. Stop at owner review. No schedule or always-on runner is installed; do not imply persistent monitoring.

Read docs/workflow.md and AGENTS.md. GitHub is the durable source of truth. Role instructions are proposals awaiting bootstrap acceptance.

Dispatch PR Reviewer for ready PR handoffs and after Engineer corrections. Include immutable head/base SHAs and the linked story. Route CHANGES_REQUESTED findings or confirmed merge conflicts to the Engineer on the same story; WAITING remains incomplete. Only technical PASS at the current revision advances to owner review. Do not start the next story until owner acceptance and merge. A draft partial research PR may be inspected but must not be promoted as completed work. Respect the per-run scheduling and identity limits in docs/roles/pr-reviewer.md.
