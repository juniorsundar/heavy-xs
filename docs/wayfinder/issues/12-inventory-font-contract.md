# Inventory the observable font contract

Parent: ../freetype-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-freetype-contract
Blocked by: none

## Question

What observable behavior does the current FreeType-backed font layer provide, which callers rely on it, and which parts already have useful regression evidence?

Inspect `rust/src/renderer.rs`, `rust/src/ft.rs`, `rust/src/renderer_api.rs`, renderer/cache/window callers, `data/core/style.lua`, font configuration/fallback consumers and the existing renderer/hit-test fixtures. Record Lua object shapes/signatures, font loading/copy/resizing, supported options and defaults, advances/heights/baselines, tabs and fractional positioning, synthetic styles, fallback/missing glyphs, Unicode handling, invalid-font errors, lifetime/cache behavior and scale-dependent paths.

Separate observable behavior, implementation detail, tested fact and unknown. Identify what formats the editor exposes through arbitrary font paths without claiming every FreeType feature is used. Include all three bundled fonts and a plan for representative public/custom-font coverage; private fonts/configuration must not be published as evidence.

This is bounded evidence collection that unblocks acceptance and baseline decisions, not a port, a new rendering abstraction or a feature cleanup. Write an evidence asset under `docs/wayfinder/assets/`, link it on resolution and identify test/format gaps. Do not rewrite existing baselines.

Context: [Survey Rust-native font backends](13-survey-rust-font-backends.md) proceeds independently; this inventory unblocks [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md) and [Define font compatibility and acceptance criteria](15-define-font-acceptance.md).

## Comments

### Resolution

Recorded the source-backed [observable font contract inventory](../assets/font-contract-inventory.md), including Lua object/API shapes, options/defaults, unhinted advances versus auto-hinted bitmaps, group/fallback/path quirks, scale/cache/lifetime/error behavior, bundled font identities and caller dependencies. The timed-out worker's draft was independently checked and corrected; its timeout was not treated as a completed result.

Fresh isolated debug validation passes the existing renderer/rencache golden suites and 1,239,848 hit comparisons. A bounded [23-assertion probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-contract-check.lua) with [raw output](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-contract-check.txt) verifies the less-obvious group/copy/path/fallback/advance observations. Production sources, dependency configuration and goldens are unchanged.

This unblocks [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md). Real-window/HiDPI/optional-renderer, custom-format and performance/memory evidence remains explicit follow-up; no replacement, feature exclusion or acceptance threshold is chosen. The independent library survey is not resolved by this inventory.
