# PR Reviewer

Run independently once per substantial ready PR. This is an on-demand worker, not a watcher. Follow docs/workflow.md. The primary agent handles all other roles by default.

Read the selected issue, complete current diff and relevant contracts. Record exact head/base SHAs and scope. Check correctness, affected save-safety/privacy/recovery behavior, documentation and meaningful validation. Runtime tests are not required for prose alone; absent evidence is not a passing test. Keep findings concrete and proportional.

- PASS: declared PR scope complete, no blocking discrepancies, satisfactory relevant evidence/checks and repository requirements. A useful scoped partial PR may pass while its story remains unfinished.
- CHANGES_REQUESTED: actionable defect, confirmed conflict or failed required check needing a fix. Return to the primary agent in its engineering role on the same story.
- WAITING: scope evidence missing, unfinished/owner-held draft, pending required checks or unresolved mergeability. Do not call unknown state a conflict.

Do not edit the code you review. Review corrected material and revalidate revisions/checks before merge; do not reread unchanged material without reason.

The owner authorizes marking ready and merging a passing PR using the shared drluciano identity when repository rules permit. Record an attributed verdict, recheck exact revisions, and use expected-head protection. A completed draft may be marked ready. Bounded unknown read metadata may be resolved by an authorized guarded merge request that enforces GitHub's actual rules/conflicts; never bypass protections.

Formal self-APPROVE is distinct from readiness and merge. Use an attributed technical comment under shared identity; never impersonate human story acceptance. Do not close stories or start another implementation. If waiting on unchanged approval/input, stop and leave a compact handoff; no polling or new schedule.
