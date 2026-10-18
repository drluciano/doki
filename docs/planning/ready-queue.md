# Upcoming handoff refinement — F02 and D01

Planning only. No engineering start, status changes, GitHub mutations, or owner-criteria edits are authorized by this document. Current issue bodies #13 and #31 were read through the GitHub connector for this dispatch.

## Queue and gates

| Rank | Story | Dependency status | Handoff status | Dispatch rule |
| --- | --- | --- | --- | --- |
| Active | F01 #12 | Sole unfinished engineering story | Evidence incomplete | Continue F01; no other engineering dispatch while it is unfinished or awaiting owner review. |
| 1 upcoming | F02 #13 | F01 must be accepted and merged; unmet | Proposed refinement only; NOT READY | Finish evidence and interface handoff now as planning; engineer starts only after F01 acceptance/merge and owner workflow gate. |
| 2 upcoming | D01 #31 | G01 accepted/merged per dispatch; satisfied | Design handoff still proposed; NOT READY for engineering | Designer can refine asynchronously; engineer waits for complete handoff and vacant engineering WIP. |

Ready engineering stories in this document: **zero**. Upcoming refinement candidates: **two**, below the maximum of three. D01 has no formal F01 dependency; do not invent one. Its design may progress now, while platform-specific claims and final safety copy stay provisional pending F01/F02 findings. No automatic progression from acceptance to another story.

Known owner-reported target: **Eden 0.2.1, AYN Odin 3, Android 15**. These identify intended testing, not observed compatibility evidence. Exact package/build identity, provider/location, grants, disposable fixtures, and completed operation results remain unverified. Windows details and the second emulator per platform remain unresolved. Do not let the missing second target block legitimate Eden preparation; do not reduce F01's coverage requirement.

## F02 #13 — proposed safety-contract handoff

Source: https://github.com/drluciano/doki/issues/13

Owner acceptance criteria below are reproduced unchanged. Evidence details are proposed review aids, not new completion criteria or replacements.

| Exact criterion | Concrete handoff artifact | Evidence/reviewer check |
| --- | --- | --- |
| Specify capture, proposal, apply, rollback, retention and deletion invariants. | Normative safety contract with one subsection per operation, preconditions, preserved data, permitted mutations, failure outcomes and user-visible outcome. | Trace operation examples through each invariant; show that receiving does not apply, verified backup precedes replacement, missing storage does not imply deletion, and divergent progress stays represented. Label proposed policy separately from established platform capability. |
| Document every state transition and recovery behavior after interruption. | State diagrams plus transition table keyed to persistent journal/metadata states; include capture and transfer partial states, pre-apply recheck, backup, staging, replacement, verification, rollback/recovery and cleanup. | For every enumerated transition, list interruption outcome, restart action, retry/idempotency rule and user-visible error. A transition coverage checklist makes omissions reviewable; no claim that a diagram proves runtime safety. |
| Distinguish byte-integrity verification from format-validity checks. | Explicit definitions and result taxonomy for hash match, parser validation where available, emulator-load observation and unknown validity. | Provide examples where matching hashes do not establish a usable save; opaque formats retain unknown semantic validity. Record which evidence is required for a compatibility claim versus transport integrity. |
| Define guarantees and limits for Windows paths and Android document providers. | Capability matrix linked to accepted F01 records, including normalized paths, bundle containment, locked files, permission loss, provider operations and replacement/recovery limits. | Each guarantee references exact observed setup or is explicitly a design requirement awaiting implementation. No universal atomic-replacement guarantee and no extrapolation across providers/filesystems. |

Nonvisual design handoff must specify interface/error contracts: operation inputs; capability queries; progress/state outputs; cancellation semantics; stable error identifiers; retry/recovery affordances; and human-readable safety explanations. Include illustrative copy for changed local files, backup failure, incompatible save, missing storage and interrupted installation. Do not promise loss can never occur.

Suggested review packet:
1. Safety contract and platform capability appendix.
2. State diagrams and complete transition/recovery table.
3. ADRs for backup gating, bundle replacement and retained-history policies.
4. Criterion-to-section/evidence map, exact revision and reproducible walkthrough using disposable scenarios.
5. Open-questions register separated from settled guarantees.

F01 dependencies/unknowns that must remain visible:
- Which Eden Android access route actually works on the named device/version, including persisted grant after restart/reboot.
- Whether complete save bundles can be read and replaced; actual export/import alternatives and emulator load/restore results.
- Windows emulator versions, locations, locks and replacement semantics.
- Second emulator selection and real evidence on each platform.
- Provider/file operation guarantees and consequences for interrupted multi-file replacement.

F02 is NOT READY until F01 is accepted and merged and its own proposed handoff becomes concrete. Do not silently lower guarantees to fit an untested platform.

## D01 #31 — proposed design review handoff

Source: https://github.com/drluciano/doki/issues/31

| Exact criterion | Concrete review artifact | Review procedure |
| --- | --- | --- |
| Review CocoonFE reference and record visual direction without copying unlicensed assets. | Annotated reference study and original visual direction; asset/source/license ledger for every included external item. | Compare rationale to cited reference; identify original versus reused work and verify provenance. Do not claim this PM refinement has inspected CocoonFE assets. |
| Define typography, colors, spacing, components, icons and motion tokens. | Token specification and component gallery with game cards, buttons, dialogs, progress and status badges; light/dark direction if proposed. | Inspect token coverage, hierarchy, interactive feedback, motion purpose and fallback. Optional themes must not become undeclared scope. |
| Support scalable text, contrast, screen readers, keyboard and reduced motion. | Annotated scalable layouts, focus order/visible focus, semantic labels, contrast measurements and reduced-motion equivalents. | Demonstrate long labels/larger text, keyboard traversal, screen-reader announcements and motion reduction in prototype or runnable artifact; state design-only evidence limits. Numerical targets should be proposed explicitly rather than presented as existing owner criteria. |
| Do not rely on color alone for safety or synchronization states. | State legend using text and shape/icon cues; example local-only, transferring, incoming, divergent, error and recovery presentations. | Inspect monochrome rendering and distinguish states by labels/cues; avoid unverified “protected” status or freshness claims. |
| Document reusable component states and responsive rules. | Usage guide with default, hover/focus, pressed, disabled, loading, empty, success and error variants where applicable; breakpoint/layout rules and accessibility annotations. | Review small Android and Windows layouts, larger text, long game names and recovery dialogs; verify states are meaningful to each component rather than mechanically duplicated. |

Story-specific designer handoff should show a component-gallery navigation flow, component interactions, permission/error samples and responsive mockups. This is a design-system story: avoid implementing library, pairing, or conflict-review feature scope under it.

Unknowns and boundaries:
- F01's Android access/permission findings affect realistic permission copy and layouts; placeholders must be labeled.
- F02 will settle safety vocabulary and guarantees; D01 must not freeze unsupported claims.
- Platform typography availability, input modalities and actual assistive-tech behavior need evidence in the eventual artifact.
- No implicit F01 blocker for exploratory design; D01 still needs actual design handoff before engineering readiness.

Suggested owner review packet: annotated gallery, token table, motion/reduced-motion comparison, accessible interaction checklist, asset ledger, responsive mockups, exact artifact revision, and runnable build/screenshots when UI implementation is delivered.

## Ranked next actions

1. Keep engineering focused on completing F01's real disposable-data evidence, beginning with the owner-reported Eden/Odin target; retain all unverified cells.
2. PM prepares F02's section/transition/interface templates and unresolved-capability register; Designer refines D01's reference study and gallery asynchronously. These are handoff preparation only.
3. After F01 acceptance and merge, re-evaluate F02 against actual findings and request the owner's prioritization if another engineering story is proposed. Do not auto-start.
4. Promote a story only after its dependencies and story-specific handoff are satisfied; keep ready queue at three or fewer and engineering WIP at one, including awaiting review.
