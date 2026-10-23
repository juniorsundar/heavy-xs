# Establish GUI runtime and customisation iteration baselines

Parent: ../gui-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-gui-cost-baseline
Blocked by: 39

## Question

What reproducible runtime and change-to-result costs does the current editor have, and what evidence is still needed to calibrate this effort's acceptance criteria?

Use [Inventory the GUI and legacy Lua extension contract](39-inventory-gui-extension-contract.md) to define bounded representative observations. Separate cold/warm startup, first visible frame and input-ready state; typing/scrolling/layout/redraw responsiveness; idle CPU, memory and retention; default versus explicitly listed customisation/plugin workloads. Record timing boundaries, repetitions/distributions, hardware, revision, build profile, display backend, scale and instrumentation overhead. A headless runner is not evidence of visible desktop latency.

Separately record the existing Lua edit-to-result workflow (do not assume hot reload), Rust incremental and clean build costs, and artifact/cache disk use. Keep clean and warm measurements distinct; never delete the developer's caches, change their configuration or mistake a new worktree's cold build for the usual iteration cost. Use isolated directories only for bounded, explicitly documented experiments; report resource/access constraints instead of launching unbounded builds.

Use existing runners and platform evidence where relevant, but do not copy old acceptance thresholds. Expose suspected bottlenecks as hypotheses, not proven causes. Distinguish measurements from estimates and unavailable checks. Resolve with linked, reproducible evidence and limitations, not numerical acceptance gates, an optimization or a redesign.

## Comments

### Resolution

Collected the bounded [GUI runtime and customisation iteration baseline](../assets/gui-cost-baseline.md), with [measurement data](../assets/gui-cost-baseline/results.json) and reproducible evidence-only helpers. Source reference is `e556d388892280d8343a82a625941137e3a6ac5f`; binary, build inputs, fonts, configurations, clocks, display and probe provenance are recorded in the evidence.

The record covers normal-core startup/first-frame and synthetic input-handling observations; scripted typing, scrolling and split-layout work; focused/unfocused idle; bounded GC-conditioned view/document retention; an actual Lua user-init edit/save/restart path; and isolated artifact-clean, normal-checkout no-op and core-edit Rust build costs. The final data comprise 49 runtime probes plus six Lua edit/save/restart trials, with separate build measurements. No acceptance threshold or architecture is selected.

Important corrections are preserved rather than hidden: linked-worktree `.git/logs/HEAD` watch behaviour invalidated the initial no-op-build interpretation, and X11 `PointerRoot` did not establish an unfocused editor. Normal-checkout build trials and explicit X-root/native-focus-loss checks supplied the reported replacements. No developer artifact cache was cleared or rebuilt; temporary source edits were limited to isolated copies and restored before runtime use.

The evidence explicitly does **not** establish cold-cache startup, earliest useful input-ready or physical-desktop/input-to-photon latency, real IME/fractional-density/native-Wayland behaviour, general peak/leak bounds, arbitrary plugin workload performance or robust regression tolerances. A private Xvfb first-frame marker is not physical screen evidence. These missing observations are inputs to [Define GUI performance and customisation acceptance criteria](43-define-gui-acceptance-criteria.md), not passing or waived gates. The external-plugin reference remains separately covered by [Assemble the GUI and plugin compatibility reference evidence](45-assemble-gui-compatibility-reference.md); no duplicate ticket or design-fog graduation is needed here.

Validation: pre-experiment safety advice and default-model independent evidence review found no blockers. Tables were independently recomputed from retained measurements; helper assertions/self-checks, private-display smoke/final runs, C warnings-as-errors compilation, source/binary provenance and input-cleanliness checks passed. Existing contract tests/expectations were unchanged, not rerun. Large raw artifacts remain in the owned `target/gui-cost-baseline.M60jmM/` directory (approximately 1.85 GiB), with reviewable results/probe sources retained under `docs/wayfinder/assets/`. No newly unblocked ticket was worked.
