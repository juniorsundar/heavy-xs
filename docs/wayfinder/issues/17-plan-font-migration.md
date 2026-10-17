# Define the FreeType replacement handoff

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Blocked by: 15, 16

## Question

Given the selected backend and approved differences, what is the smallest complete migration and validation sequence an implementation agent can execute without reopening settled decisions?

Name the exact module seam and responsibilities, changed callers/configuration/dependencies, font/cache lifetime and failure boundaries, regression fixtures and deliberate baseline changes, visual review and real-editor checks, build/native-symbol removal checks and any implementation stop conditions. Cover documentation and configurable fonts as well as the three bundled fonts. Keep Lua API/native-plugin ABI and unrelated SDL/Lua/regex behavior stable.

Distinguish decisions already made from implementation tasks. Link source-of-truth resolutions and evidence instead of copying their content. Identify any new unresolved decision as a child ticket/blocker rather than guessing. Confirm the handoff with the user; do not execute the migration. The map is complete when the backend choice and actionable handoff are agreed and no required decision remains open.

Prerequisites: [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) and [Select the Rust-native font backend](16-select-rust-font-backend.md).

## Comments
