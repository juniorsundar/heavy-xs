# Inventory the GUI and legacy Lua extension contract

Parent: ../gui-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-gui-contract-inventory
Blocked by: none

## Question

What does the current GUI do, how can Lua plugins alter it, and which observable behaviours are already protected or missing from the acceptance evidence?

Produce a bounded, source-backed inventory, not an architecture proposal. Trace startup-to-ready, input/command dispatch, view/layout ownership, update/scheduling, drawing/caching and presentation across Lua and Rust. Start with `data/core/init.lua`, `view.lua`, `rootview.lua`, `node.lua`, `docview.lua`, `command.lua`, `keymap.lua`, `style.lua`, the other built-in views, and the corresponding `src/` modules; include bundled UI plugins where they exercise extension behaviour.

Describe functions, fields, inheritance/overrides, event ordering, command/config hooks and lifecycle assumptions plugins rely on. Distinguish documented interfaces, observed de facto extension points and unobserved possibilities without declaring undocumented behaviour expendable. Use concrete examples, including bundled plugins that replace core methods. Identify candidate third-party sampling sources and their limits; do not silently define ecosystem-wide compatibility from bundled plugins alone.

Map the evidence in `tests/contract/`, `scripts/rust-port/verify.sh`, its goldens and companion checks to those behaviours. Verify the initial reconnaissance finding that primitive coverage does not exercise composed Lua GUI extension paths. Record what is not established and what additional evidence a compatibility decision needs.

Keep existing cases and expectations unchanged. Link a concise inventory asset under `docs/wayfinder/assets/` with source revision and references; no production changes, new compatibility exclusions, benchmarks or test implementation in this ticket.

## Comments

### Resolution

Completed the source-backed [GUI and legacy Lua extension contract inventory](../assets/gui-extension-contract-inventory.md) at source revision `e556d388892280d8343a82a625941137e3a6ac5f`. It traces startup, dispatch, focus/layout, scheduling, rendering/presentation and representative bundled extension patterns, distinguishing repository documentation, observed dependencies and missing evidence.

The inventory qualifies the initial reconnaissance: the 18 golden suites do not assert a composed Lua GUI, but some exercise Lua helpers/documents and the separate startup check runs normal core with a narrow `RootView` shutdown override. Existing checks are preserved; neither native-plugin ABI coverage nor that shutdown regression establishes general Lua UI-plugin compatibility.

The linked evidence records candidate ecosystem sources and the mismatch between current mod-version 4 and dated upstream/3.0 guidance. It does not select a compatibility sample, discard undocumented usage or approve new interface guarantees. Those decisions already belong to [Define the GUI compatibility evidence and extension envelope](41-define-gui-compatibility-envelope.md); runtime/development measurements belong to [Establish GUI runtime and customisation iteration baselines](42-baseline-gui-and-development-costs.md). No additional ticket or fog graduation is needed from this bounded inventory.

Validation: default-model independent source review found no blockers; its metadata-loading and drawing-hook precision notes were incorporated. All 115 local evidence references and line ranges were checked, the 18-golden/2,663-line count verified, and source/test/build-input paths confirmed unchanged. No runtime tests, benchmarks, live desktop review or external-plugin execution were performed. No architecture or compatibility exclusion is chosen, and no newly unblocked ticket was worked.
