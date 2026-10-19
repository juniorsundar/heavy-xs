# Define the FreeType replacement handoff

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-font-handoff
Blocked by: 15, 16

## Question

Given the selected backend and approved differences, what is the smallest complete migration and validation sequence an implementation agent can execute without reopening settled decisions?

Name the exact module seam and responsibilities, changed callers/configuration/dependencies, font/cache lifetime and failure boundaries, regression fixtures and deliberate baseline changes, visual review and real-editor checks, build/native-symbol removal checks and any implementation stop conditions. Cover documentation and configurable fonts as well as the three bundled fonts. Keep Lua API/native-plugin ABI and unrelated SDL/Lua/regex behavior stable.

Distinguish decisions already made from implementation tasks. Link source-of-truth resolutions and evidence instead of copying their content. Identify any new unresolved decision as a child ticket/blocker rather than guessing. Confirm the handoff with the user; do not execute the migration. The map is complete when the backend choice and actionable handoff are agreed and no required decision remains open.

Prerequisites: [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) and [Select the Rust-native font backend](16-select-rust-font-backend.md).

## Comments

### Grilling round 1 — answers

The user answered yes to all six recommendations: plan only; a fresh port of the prototype module with every prototype hook removed; the renderer golden re-recorded in its own commit, pixel hashes only; the validation list; the stop conditions; and the doc updates, including a new `docs/fonts.md` with `PLAN.md` left as history.

## Resolution

The handoff was `docs/font-backend-handoff.md`. It was implemented in `b7413210` (backend) and `49333384` (pixel-only golden), then deleted at the user's request. It named:

- the change: three pinned dependencies in, `freetype-rs` and `src/ft.rs` out; FreeType touchpoints in `src/renderer.rs` replaced with a ported font module; prototype-only hooks listed for exclusion;
- the doc updates;
- the single golden re-record and its precondition (only pixel-hash rows change);
- the validation checklist: 16 suites in debug and release, the `sdl_renderer` build, the geometry probe against the prototype output, the inventory assertions, the 114-case malformed corpus, absence of FreeType from `cargo tree` and the binary, a headless user-config startup, the hit-test and startup checks;
- stop conditions and four non-blocking follow-ups.

The reference prototype branch `prototype/font-editor-candidate` (`9723a24b`) remains on `origin` without a PR. This ticket itself was planning only; the user authorized the implementation separately.
