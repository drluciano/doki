# Remaining GitHub Project setup

Verified project: https://github.com/users/drluciano/projects/2 . Public viewing now works. Available connector tools do not expose Projects mutations; the available browser is signed out. Repository issue/branch writes work after the owner corrected installation access. The following steps are not yet applied.

1. Open the repository issues list, select all 49 issues (including additional pages if needed), and use the bulk Projects action to add them to the existing Project 2. Alternatively use Add item in the project and select this repository's issues. Do not create a duplicate project.
2. Use Backlog view https://github.com/users/drluciano/projects/2/views/1 . The supplied /views/5 URL is My items and contains an assignee:@me filter; unassigned issues will be hidden.
3. Configure one Status field: Backlog, Needs design, Designing, Ready for engineering, Engineering, Awaiting owner review, Changes requested, Blocked, Accepted, Done. Preserve existing items and views. Set G01 (#1) to Awaiting owner review; other items start Backlog.
4. Existing observed fields include Priority, Linked pull requests, Sub-issues progress, Size and Estimate. Retain useful fields. Configure Priority P0/P1/P2 and add Phase, Role and Epic only if absent. Populate from backlog.json / issue-map.json.
5. Create the proposed classification labels listed in workflow.md. Do not duplicate workflow statuses as labels. Epics and child dependencies already link by GitHub URL in issue bodies; native sub-issue relationships are not yet configured.
6. Review repository protection and agent identity setup described in workflow.md. main was observed unprotected. No ruleset, required checks or protected branch settings have been installed. The connection authors as drluciano; owner self-review cannot satisfy an independent approval requirement. Choose a separate agent identity before enforcing that review mechanism.

GitHub Project status fields do not enforce engineering WIP or owner approval. Those are orchestrator rules until an explicitly authorized workflow runner is configured. No always-on runner or schedule was created.
