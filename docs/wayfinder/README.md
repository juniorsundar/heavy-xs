# Local Wayfinder tracker

The user chose `docs/wayfinder/` instead of the default `.scratch/` location. Each effort has its own map and child issues:

- [Rust/Lua responsibilities and a Rust-native regex design](map.md): completed regex effort.
- [A Rust-native font backend to replace FreeType](freetype-map.md): completed; implemented in the Rust port.
- [Replace SDL3 with Rust-native platform crates](sdl3-map.md): planning complete and implemented through the [handoff](assets/sdl3-migration-handoff.md)'s Step 5: SDL is gone from the build, package and binary ([final artifact evidence](assets/sdl3-step5-artifact-evidence.md)). Accepted costs and pending checks are listed in the handoff's Step 4 dispositions.

## Wayfinding operations

- Each canonical map is labelled `wayfinder:map` (`map.md` for the original effort, `freetype-map.md` for fonts). It records the destination, standing preferences, resolved-decision links and remaining fog, not a list of open tickets.
- Child issues live in `issues/NN-<slug>.md`. Each has a title, `Parent`, `Labels`, `Type`, `Mode`, `Status`, `Assignee` and `Blocked by`.
- Use only `wayfinder:` labels. Types are `grilling`, `research`, `prototype` and `task`. Modes are `HITL` or `AFK`.
- Status is `open`, `claimed` or `resolved`. An open ticket with no assignee is unclaimed.
- A `Blocked by: 01, 02` line refers to issue filename prefixes. `none` means no blockers. This local tracker has no native dependency mechanism.
- The frontier is open, unclaimed tickets whose blockers are all resolved. Filter child issues by their `Parent` map, then scan by numeric filename order. Issue filename prefixes are unique across this tracker. Read the selected map first, not every issue body.
- Before working a ticket, set its status to `claimed` and its assignee to the developer/session driving it, and save. Do not take a ticket another session has claimed.
- Use the ticket type's skill; grilling requires both grilling and domain-modeling. HITL tickets require the user's actual answers.
- Resolve by appending a resolution comment under `## Comments` / `### Resolution`, setting status to `resolved`, and adding one linked gist to the map's Decisions so far. The resolution lives in the ticket, not the map. Link evidence instead of copying it.
- Refer to maps and tickets by linked title in prose, never by bare numbers. Numeric IDs are only tracker metadata.
- Create issues before wiring dependencies and links. Check for cycles. Read current files before updates; other sessions may be working concurrently.
- Charting creates the map; it does not resolve decision tickets. In a working session, resolve at most one non-research ticket.
- Planning is the default; any execution override must be explicit in that effort's Notes. The SDL3 effort is planning only: implementation, adoption and feature removals require a later handoff.

The SDL3 route is mapped and implemented; see the [handoff](assets/sdl3-migration-handoff.md) for the accepted differences and follow-ups. The completed regex and FreeType maps remain separate historical efforts.
