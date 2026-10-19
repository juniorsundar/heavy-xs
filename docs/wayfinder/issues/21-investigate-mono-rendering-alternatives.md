# Investigate Rust-native mono rendering alternatives

Parent: ../freetype-map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-font-mono-research
Blocked by: 13

## Question

After the user rejected the 50% Zeno Alpha-to-mono conversion, which existing Rust-native APIs or small outline/raster compositions provide defensible alternatives for a new mono experiment, without writing a rasterizer or silently changing hinting policy?

Distinguish binary output storage from the rasterization algorithm: unpacked 0/255 alpha is acceptable as a representation; the observed failure is thin-stroke loss, not the absence of packed one-bit storage. Investigate whether an existing non-antialiased raster path provides different sampling or dropout behavior, and whether a clearly disclosed coverage-preserving conversion is a bounded alternative rather than an assertion of native-mono equivalence.

Use primary versioned documentation/source. Start with the current Skrifa/Zeno composition and at most two tightly relevant existing Rust-native raster alternatives; do not repeat the whole backend survey. Exclude C/C++ bindings, forks, custom font engines, implicit forced hinting for hint `none`, synthetic-weight changes to conceal loss, and format exclusions. Keep Skrifa's existing parse/hint/metric control routing unless the sources expose a concrete reason it cannot work.

Return exact versions/APIs, algorithmic and integration differences, whether font-specific dropout/thin-stroke handling is actually established, relevant native-dependency/licence constraints, and the smallest useful next experiment. Explain why the current FreeType sample can retain strokes that the rejected coverage threshold loses, only as far as source evidence supports. Research cannot prove legibility, accept any new conversion or select a backend; mark runtime/visual claims as untested.

No implementation, new dependency adoption, compilation, font capture or performance qualification. Capture a public-only report on a throwaway research branch, push non-force without a PR or merge, and leave the live tracker as the decision owner.

## Comments

Created from the user's explicit rejection recorded in [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md), after its first native screen. Use [Survey Rust-native font backends](13-survey-rust-font-backends.md) and [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) as the existing context. This investigation supplies facts for the next HITL experiment; it does not reopen that rejection or waive the acceptance criteria.

### Resolution

Primary-source research is captured in [Mono rendering alternatives](../assets/font-mono-alternatives.md), published as a report-only commit [`cd137d06`](https://github.com/juniorsundar/heavy-xs/blob/cd137d0668c932a7c5d91bbbdaa8f44c2feb8d1e/docs/wayfinder/assets/font-mono-alternatives.md) on `research/font-mono-alternatives`. The branch was pushed non-force, without a PR or merge; the coordinator index and production inputs were untouched.

- Zeno 0.3.3 has no shipped mono mask mode. A new **any-coverage mono conversion** (`alpha > 0`) would retain positive quantized samples, but could thicken strokes and is not native-mono equivalence or an approved replacement.
- Archived code-path inspection establishes that saved grayscale/**hint-none** PPMs contain the same unhinted Alpha coverage used before the rejected mono threshold. They can supply a cheap postprocessing experiment. **Hinted** grayscale images cannot stand in for Mono-target hints, whose configured outlines differ.
- tiny-skia 0.12.0 exposes an existing non-AA fill path (`Paint.anti_alias = false`) and documented path verbs compatible with the Skrifa outline feed. Thin-stroke quality is untested; no font-dropout handling was established in the examined entry points. Declared licence/dependency information is recorded, not a completed supply-chain qualification.
- FreeType 2.13.2 documents a dedicated B/W rasterizer with dropout controls, unlike smooth coverage thresholding. This establishes a capability difference, not a proved causal account of the captured CFF2 pixels.

An independent primary-source/code-path review passed. No new conversion, library, code, capture, performance result, geometry change or appearance is approved. The research question is resolved; the separate comparison remains claimed and unresolved. The recommended next discussion is the smallest unhinted any-coverage image experiment; obtain the human's bounded workload approval before producing it.
