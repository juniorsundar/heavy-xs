# Local Wayfinder tracker

This repository's Wayfinder map lives in [Rust/Lua responsibilities and a Rust-native regex design](map.md). The user chose `docs/wayfinder/` instead of the default `.scratch/` location.

## Wayfinding operations

- The canonical map is `map.md`, labelled `wayfinder:map`. It records the destination, standing preferences, resolved-decision links and remaining fog, not a list of open tickets.
- Child issues live in `issues/NN-<slug>.md`. Each has a title, `Parent`, `Labels`, `Type`, `Mode`, `Status`, `Assignee` and `Blocked by`.
- Use only `wayfinder:` labels. Types are `grilling`, `research`, `prototype` and `task`. Modes are `HITL` or `AFK`.
- Status is `open`, `claimed` or `resolved`. An open ticket with no assignee is unclaimed.
- A `Blocked by: 01, 02` line refers to issue filename prefixes. `none` means no blockers. This local tracker has no native dependency mechanism.
- The frontier is open, unclaimed tickets whose blockers are all resolved. Scan by numeric filename order. Read the map first, not every issue body.
- Before working a ticket, set its status to `claimed` and its assignee to the developer/session driving it, and save. Do not take a ticket another session has claimed.
- Use the ticket type's skill; grilling requires both grilling and domain-modeling. HITL tickets require the user's actual answers.
- Resolve by appending a resolution comment under `## Comments` / `### Resolution`, setting status to `resolved`, and adding one linked gist to the map's Decisions so far. The resolution lives in the ticket, not the map. Link evidence instead of copying it.
- Refer to maps and tickets by linked title in prose, never by bare numbers. Numeric IDs are only tracker metadata.
- Create issues before wiring dependencies and links. Check for cycles. Read current files before updates; other sessions may be working concurrently.
- Charting creates the map; it does not resolve decision tickets. In a working session, resolve at most one non-research ticket.
- This effort is planning only. Implementation, engine adoption and feature removals require a later handoff.

To resume: ask Wayfinder to work through [Rust/Lua responsibilities and a Rust-native regex design](map.md); it takes the first frontier ticket unless the user names another.
