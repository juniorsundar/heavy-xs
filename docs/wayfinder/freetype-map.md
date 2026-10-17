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
- [Collect the FreeType rendering and cost baseline](issues/14-collect-freetype-baseline.md): build-pinned reference sheets, metrics, fresh/warm cost and retention for bundled and public custom fonts are recorded, including unmeasured paths; RSS retention across font cycles remains unclassified and is tracked by [Classify font memory retention in the current renderer](issues/19-classify-font-memory-retention.md).

## Not yet specified

- Whether further per-format, malformed-font or retention investigations are needed beyond the initial comparison depends on the baseline, agreed criteria and actual candidate results.
- Any compatibility accommodations or migration hazards exposed by those experiments. A gap triggers a decision, not a silent sacrifice or an emulation project.

## Out of scope

- Implementing/adopting the replacement during this planning effort; removing FreeType before a separate implementation handoff.
- Replacing SDL3 or Lua, changing native-plugin ABI, restoring non-Linux platforms, reopening regex-engine selection or investigating its accepted slowdowns.
- Writing a font parser/rasterizer, maintaining a fork, or retaining a hidden C fallback without a new explicit user decision.
- Adding a shaping stack, a font-management service or unrelated rendering features merely because a candidate supports them.
