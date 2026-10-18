# Delivery workflow

## Owner direction, 2026-10-18

The owner asked for a working MVP built without waiting on per-step approval. Work proceeds in incremental draft PRs, each proving a few steps (engine first, then relay, app and Android adapter); the owner reviews and merges. Engine work uses synthetic fixtures and does not wait for F01 device evidence; any claim about a specific device, emulator or provider still requires that evidence. Where the rules below conflict with this direction, this direction wins.

## Low-overhead operating model

One primary agent handles product management, design, engineering and coordination as needed. The role guides describe responsibilities, not continuously running workers. Do not spawn separate PM, Designer, Engineer or Orchestrator agents by default. Use an independent PR Reviewer once a substantial PR is ready. Extra specialist agents require an explicit owner request or a concrete need explained before delegation.

The one-minute heartbeat is paused. Do not restart it, replace it with another polling loop or schedule agent work without a new owner request. Resume on an owner message or an actual handoff/check result available during an active run. GitHub Actions performs deterministic builds/tests; no event-to-agent wake integration is currently installed or implied.

## Story and acceptance gates

The owner alone accepts and completes stories. Keep exactly one application engineering story active across implementation, corrections, PR review and waiting for owner acceptance, even if some scoped PRs have merged. Start another engineering story only after the owner accepts the current story and its implementation is merged. Silence and elapsed time never count as acceptance. PM/design preparation may occur when it directly helps; keep the proposed queue at three or fewer and avoid speculative backlog expansion.

Backlog -> Needs design -> Designing -> Ready for engineering -> Engineering -> PR review -> Awaiting owner review -> Accepted -> Done.

Nonvisual work can skip visual design with a documented interface/error contract. Record Blocked or Changes requested with a concrete cause and next action. Do not weaken criteria to fit an implementation.

## Work in meaningful batches

Choose a bounded user outcome, finish its design/code/documentation and relevant checks, then create one substantial PR. Keep interim research and notes local until they support that delivery. Do not create separate PRs or agent handoffs for every planning update. A scoped partial delivery is allowed when it has independent value or an actual blocker warrants it; say clearly what remains incomplete.

The primary agent reads AGENTS.md, the selected issue and relevant contracts, not the entire backlog each run. Reuse context and inspect only changed GitHub state. Do not repeat unchanged reviews, checks, source retrieval attempts or status reports. Maintain a compact handoff with story, branch/PR, head revision, completed evidence, unresolved blockers and next action. GitHub issues/PRs remain the durable project record; local handoff notes are a cache.

## Ready and done

Before engineering: accepted/merged prerequisites, bounded scope, observable criteria, design where applicable, specified error/recovery/accessibility states, and a validation/documentation plan. Isolate unresolved technical questions as explicit research rather than inventing platform guarantees.

Before PR review: complete declared PR scope, full diff, criterion-to-evidence map, relevant check results, updated contracts/help, reproducible review steps, and runnable artifact or screenshots when applicable. Prose-only changes do not need application runtime tests. No missing check or unrun device test is a pass.

Done requires owner acceptance of the complete story, merged implementation, satisfactory relevant checks and documentation, and owner-completed issue. Use Refs #N rather than auto-closing keywords. A merged partial-research PR does not complete its parent story.

## One independent technical review

Dispatch the Reviewer once for the ready batch. Return actionable discrepancies or confirmed merge conflicts to the primary agent in its engineering role on the same story. Re-review corrected/changed material; revalidate current revisions/checks before merge without rereading unchanged material unnecessarily. WAITING means missing evidence for the declared scope, an unfinished or owner-held draft, pending required checks or unresolved mergeability, not a conflict claim.

The owner authorizes the Reviewer to mark ready and merge technically passing PRs whose scope is complete and repository requirements are satisfied. Record an attributed verdict and exact head/base SHAs. Use expected-head protection and never bypass rules. If read metadata remains unknown after a bounded check, an authorized guarded merge request may act as GitHub's authoritative conflict/rule check; handle rejection honestly.

The shared drluciano identity can mark ready and merge. Only a formal APPROVE review on its own PR is unavailable; an agent verdict comment is not that review or human story acceptance. A separate identity is needed only if an actual rule requires a distinct formal reviewer. Do not invent credentials or relax protections.

## Reporting and GitHub Project

Report completed deliverables, new failures or necessary owner actions. Stay quiet while waiting on an unchanged blocker or approval. Ask once for missing information, pursue useful authorized independent work, and stop dependent work until it arrives. Do not keep agents alive merely to wait.

Project: https://github.com/users/drluciano/projects/2 . The owner's GitHub workflow already adds issues to the board. The Backlog view is /views/1; /views/5 filters My items and can hide unassigned issues. Project edits require an available authorized interface; do not claim fields or protection rules were configured without evidence. Status belongs in one Project field, with the issue handoff as fallback when edits are unavailable. Labels classify component/platform/type/risk rather than duplicate status.
