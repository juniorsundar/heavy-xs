# Define font compatibility and acceptance criteria

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Blocked by: 12, 13, 14

## Question

With the inventory, Rust-library survey and reference evidence available, which font behaviors must remain equivalent, which measured differences may be reviewed, and what acceptance checks must a replacement pass?

Discuss concrete scenes and gaps with the user. Cover bundled/custom-font scope and exclusions, Lua font API/options, drawing-versus-measurement consistency, text advances/line geometry, hit testing/cursor alignment, fallback and missing glyphs, hinting/subpixel modes, synthetic styles, loading errors/lifetimes and cache/memory behavior. Permission to consider changed metrics or pixels is not approval of any particular change.

Calibrate performance, responsiveness, memory/retention and visual-review criteria from the baseline; separate load/first-glyph and warm paths, ordinary and stress, headless evidence and live-window review. Agree comparison/build identities and what counts as a no-go. Old regex thresholds and exact FreeType pixels are not implicit gates.

Identify the candidate prototype/evidence questions now concrete enough to ticket, create those follow-ups and wire them as explicit blockers of backend selection. Resolve only through the user's actual answers. This resolution approves criteria, not a library or implementation.

Prerequisites: [Inventory the observable font contract](12-inventory-font-contract.md), [Survey Rust-native font backends](13-survey-rust-font-backends.md), and [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md). This decision gates [Select the Rust-native font backend](16-select-rust-font-backend.md).

## Comments
