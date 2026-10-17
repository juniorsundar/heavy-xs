# Select the Rust-native font backend

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Blocked by: 13, 15, 18

## Question

Which Rust-native font backend or small composition satisfies the agreed criteria on actual candidate evidence, and which specific observed differences does the user accept?

Use the survey, agreed acceptance criteria and subsequent user-reviewed prototype evidence. The survey alone cannot select a backend. Any needed prototype/measurement tickets must be explicit blockers before this ticket is worked; link their reports rather than assuming results.

Record the chosen dependency versions and native-dependency audit, ownership seam against the existing Rust renderer/cache/Lua API, concrete approved metric/pixel/option/format differences, rejected alternatives and remaining risks. Explain how the FreeType dependency and raw FFI leave the build while SDL3/Lua stay.

If no existing Rust-native composition meets the reviewed contract, return concrete conflicts to the user and pause. No implicit fork, parser/rasterizer project, translator or C fallback. Selection requires live user approval and does not authorize production implementation.

Prerequisites: [Survey Rust-native font backends](13-survey-rust-font-backends.md), [Define font compatibility and acceptance criteria](15-define-font-acceptance.md), and [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md). Any additional evidence blockers exposed by that comparison must also be resolved before selection. Selection unblocks [Define the FreeType replacement handoff](17-plan-font-migration.md).

## Comments
