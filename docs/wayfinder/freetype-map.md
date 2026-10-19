# A Rust-native font backend to replace FreeType

Labels: wayfinder:map
Status: open

## Destination

Choose a Rust-native replacement for FreeType and produce a concrete migration and validation plan, with the Lua font API preserved and any rendering or metric differences explicitly reviewed by the user. Implementation is a separate handoff.

## Notes

- Planning only: inventory, research and bounded evidence/prototypes support decisions; production font-backend replacement is not authorized by this map.
- The user permits considering measured appearance and metric differences, not accepting them in advance. Lua font API stability and internally consistent drawing, measurement, hit testing and cursor placement remain requirements.
- Include bundled and configurable/custom fonts. Inventory the actual exposed formats/options; neither bundled-font-only support nor every format supported by FreeType is presumed acceptable. Proposed exclusions need user approval.
- Prefer existing Rust-native libraries or a small composition. A Rust binding around a C font engine does not meet the destination. If none fits, report concrete gaps and return to the user; no fork, custom font engine or C fallback is implicitly authorized.
- Keep the current Linux-only scope, Lua runtime/native-plugin ABI and SDL3. Preserve the completed regex integration and its accepted trade-offs. New text shaping, font discovery, fallback redesign and platform backends are separate scope decisions, not incidental improvements.
- Responsiveness, bounded memory and readable/correct text matter; calibrate font-specific acceptance criteria from evidence. Do not automatically reuse regex timing/memory thresholds or promise pixel parity.
- Use grilling and domain-modeling for live decisions; research for external facts; prototype for user-reviewed candidate artifacts; codebase-design when establishing the replacement seam. Consult the [local tracker](README.md) for claims, dependencies and frontier queries.
- Reviewers use `opencode-go/deepseek-v4.1-flash` until the user changes that preference; request focused review only when useful.

## Decisions so far

- [Inventory the observable font contract](issues/12-inventory-font-contract.md): source-backed API/rendering behavior, bundled font identities, exercised regressions and concrete coverage gaps are recorded for baseline and compatibility decisions.
- [Survey Rust-native font backends](issues/13-survey-rust-font-backends.md): released Rust stacks and their native-dependency audit expose concrete control/rendering questions for candidate evidence; no backend is selected.
- [Collect the FreeType rendering and cost baseline](issues/14-collect-freetype-baseline.md): build-pinned reference sheets, metrics, fresh/warm cost and retention for bundled and public custom fonts are recorded, including unmeasured paths; the initial RSS observation is addressed by [Classify font memory retention in the current renderer](issues/19-classify-font-memory-retention.md).

- [Define font compatibility and acceptance criteria](issues/15-define-font-acceptance.md): the user confirmed required formats/controls and API rules, explicit geometry review, matched performance/memory evidence and live normal/HiDPI gates; no candidate or compatibility loss is approved.
- [Classify font memory retention in the current renderer](issues/19-classify-font-memory-retention.md): the baseline's RSS growth is NVIDIA driver memory from the accelerated offscreen framebuffer, growing per frame, not a font leak; with acceleration off, FreeType font cycles plateau and the shared release path frees everything. Measure font memory that way.

- [Establish additional font-format reference coverage](issues/20-establish-additional-font-format-coverage.md): mapped-glyph evidence separates working extra inputs, outline fallbacks and bitmap placeholders/unsafe paths; unavailable formats and user support-gap decisions remain explicit.
- [Investigate Rust-native mono rendering alternatives](issues/21-investigate-mono-rendering-alternatives.md): versioned APIs and archived code identify two bounded follow-up paths and their untested limits; the rejected 50% conversion stays rejected, with no replacement approved.
- [Compare candidate font controls and rendering](issues/18-compare-font-controls-and-rendering.md): per-glyph routing works on Skrifa + Zeno; the user accepted tiny-skia non-AA as the mono construction (unhinted thin-stroke loss accepted, hinted retains strokes); live, cost and format evidence moved to the in-editor prototype and format research.
- [Investigate Rust-native paths for legacy and bitmap font formats](issues/22-investigate-rust-native-legacy-and-bitmap-formats.md): Type 1 (read-fonts) and BDF (third-party parsers) have existing Rust paths needing glue, X11 PCF has none, bitmap strikes are readable by Skrifa; dispositions stay with the user.
- [Prototype the candidate backend inside the editor](issues/23-prototype-candidate-in-editor.md): Skrifa + Zeno + tiny-skia behind the renderer seam; the user accepted live appearance and performance, size-scaled bold is an approved difference, geometry is identical, no leak or malformed-font failure; CJK peak memory and the sdl_renderer fractional-scale crash are deferred follow-ups.
- [Select the Rust-native font backend](issues/16-select-rust-font-backend.md): Skrifa + Zeno + tiny-skia selected (ADR 0001); approved differences, excluded legacy/bitmap formats and a waived normal-density review are recorded, with four follow-ups for the handoff.
- [Define the FreeType replacement handoff](issues/17-plan-font-migration.md): the handoff gave the exact change, single pixel-hash golden update, validation checklist, stop conditions and follow-ups; it was implemented in `b7413210` and `49333384` and then deleted.

## Not yet specified

Nothing: the destination is reached. The migration is implemented (`b7413210`, `49333384`); the remaining follow-ups are listed in [Select the Rust-native font backend](issues/16-select-rust-font-backend.md#resolution).

## Out of scope

- Implementing/adopting the replacement during this planning effort; removing FreeType before a separate implementation handoff.
- Replacing SDL3 or Lua, changing native-plugin ABI, restoring non-Linux platforms, reopening regex-engine selection or investigating its accepted slowdowns.
- Writing a font parser/rasterizer, maintaining a fork, or retaining a hidden C fallback without a new explicit user decision.
- Adding a shaping stack, a font-management service or unrelated rendering features merely because a candidate supports them.
