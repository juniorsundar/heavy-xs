# Compare candidate font controls and rendering

Parent: ../freetype-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: pi-font-candidate-comparison
Blocked by: 14, 15, 19, 20, 21

## Question

Can an existing Rust-native library or small composition provide the agreed font controls and internally consistent rendering/measurement, and which measured differences does the user accept after seeing the comparison?

Use the prototype skill and agree a bounded experiment with the human before building a disposable harness. Apply [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) to the [Survey Rust-native font backends](13-survey-rust-font-backends.md), then test the viable candidate paths against [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md). Direct Skrifa + Zeno and stock Swash have different control/plumbing trade-offs; Fontdue's unhinted/subpixel path is a possible comparison only if the agreed criteria make it relevant. This ticket does not preselect a backend or require adopting all surveyed libraries.

Record the exact versions/features and demonstrate the real routing of hinting none/slight/full and AA mono/grayscale/subpixel. Stock Swash's fixed target/AutoFallback policy is not the editor's FORCE_AUTOHINT policy; Zeno Alpha is not native mono; Fontdue's RGB sampling is not evidence of the current LCD filter. Missing controls, thresholding or alternate algorithms must be explicit proposed differences, not silent aliases.

Use the baseline's chosen bundled/custom fonts and scenes to compare metrics and glyph placement together: three fractional phases, actual ASCII-mapped icons, genuine cross-font fallback versus .notdef, tabs, clipping, cursor alignment and the agreed formats. Show X-only bold, smoothing, italic and editor-owned decorations separately. Include bounded cold/warm, malformed-input and retention evidence using the agreed criteria; do not invent thresholds from the regex effort.

Consume [Establish additional font-format reference coverage](20-establish-additional-font-format-coverage.md) rather than inferring editor support from parser claims. Produce a current-versus-candidate format matrix, with fixture identity, load/measure/draw results, known unsupported cases and explicit unknowns. Obtain the user's required-support / approved-exclusion / blocker disposition for relevant gaps before claiming qualification.

The [confirmed criteria](15-define-font-acceptance.md#resolution--confirmed-by-the-user) are authoritative. Agree a workload manifest with the human before building, then produce:

- Paired release comparisons on the same base/compiler/profile/environment, with exact non-font inputs and intentional dependency differences recorded. Preserve reference golden expectations; distinguish invariant failures from candidate geometry/pixel differences awaiting approval.
- API-rule, geometry/hit-test/cursor, fallback/tab, style, safe-malformed-text, error-contract and lifetime checks. Show actual mode routing, not just option labels or identical-looking samples.
- Both default surface and `sdl_renderer` build/runtime evidence. Record feature paths and physical/logical density; normal-density and genuine HiDPI real-window review are selection gates, not deferred implementation checks.
- Ordinary full-viewport editing/scrolling measurements: active update/layout/draw p95 against the display interval, p99/max, separate presentation/pacing and observed stalls, and reproducible reference differences. Declare warm-up, sampling, workload units and visible work. Separate cold/first-glyph and stress cases; do not pass using tiny scene timings or idle frames.
- Matched resource-release, fixed-working-set retention and peak/steady footprint measurements, including repeated large-collection loads/copies. Use the retention investigation for ownership attribution; do not treat an inherited defect as either a candidate leak or a free memory pass. Record any shared-defect migration disposition explicitly.
- Bounded corrupt/truncated-font and hostile-input checks with clean error outcomes; external timeouts/kills are containment results, not passes.

Present concrete images, measurements, reproduction instructions and limitations for the user's actual review. Automatic checks or an agent's opinion cannot substitute for the human's appearance/compatibility answers. Link the artifact and record accepted/rejected differences precisely; unresolved conflicts must block selection.

No production backend patch, dependency adoption, fork, custom parser/rasterizer, shaping/fallback redesign or hidden C fallback. If the necessary adapter exceeds a small existing-library composition, stop and ask rather than implementing an engine inside the prototype.

This evidence is a prerequisite for [Select the Rust-native font backend](16-select-rust-font-backend.md); it does not itself authorize production implementation.

## Comments

Created from the completed library survey: candidate controls, mono/LCD behavior and synthetic-style equivalence are now precise enough to test. The baseline and live acceptance discussion remain prerequisites, and no candidate prototype or user approval has occurred yet.

Now also blocked by [Classify font memory retention in the current renderer](19-classify-font-memory-retention.md). Candidate retention cannot be interpreted until it is known whether the baseline's RSS growth comes from FreeType-specific or shared renderer code.

The user has confirmed the acceptance criteria. [Establish additional font-format reference coverage](20-establish-additional-font-format-coverage.md) is also a prerequisite; it records current extra-format behavior and unknowns, while this HITL ticket owns candidate execution and the user's actual compatibility/appearance decisions. No candidate or compatibility loss was approved by the criteria discussion.

[Classify font memory retention in the current renderer](19-classify-font-memory-retention.md) is resolved. Its results for this ticket's memory evidence:

- The baseline's RSS growth is NVIDIA driver memory from SDL's accelerated offscreen framebuffer. It grows per frame and by window area, even with no fonts.
- FreeType itself plateaus.
- Retention comparisons should run with `SDL_FRAMEBUFFER_ACCELERATION=0`, or the `dummy` driver for non-pixel phases. They should read the anonymous RSS and malloc in-use columns.
- Raw accelerated-offscreen RSS would charge driver growth to whichever backend is under test.
- The [classification probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention-classify.lua) and its variants can be reused unchanged against a candidate.
- No shared-code defect was found, so there is no inherited leak needing a migration disposition.

[Establish additional font-format reference coverage](20-establish-additional-font-format-coverage.md) is resolved; use [its report and fixtures](../assets/font-format-reference.md) for the format matrix. Type 1/PFB and safe mono-BDF samples render real mapped glyphs; COLRv1/SVG samples exercise monochrome outline fallback. Several bitmap cases instead produce generic rectangles even for mapped targets, and requested sizes can fail before drawing. Do not score parser success or those rectangles as successful glyph rendering.

The report also identifies an existing packed-mono grayscale/subpixel overread path. Do not replay it as a compatibility target or silently copy it into a candidate. Use the safe reference modes, disclose any changed behavior and follow the confirmed safety/compatibility criteria. This evidence task did not authorize fixing production code or dropping BDF/Type 1; those support-gap dispositions remain the user's answers in this HITL comparison. Remaining untested/unavailable families are explicit unknowns, not approved exclusions.

### Prototype discussion — candidate path

The user chose **direct Skrifa + Zeno first** for a small native feasibility screen, rather than comparing both stacks immediately or investigating legacy formats first. This chooses the first experiment only: it does not select a backend, approve any rendering/metric difference or format exclusion, waive qualification gates, or authorize a production patch. The bounded workload and disposable harness were subsequently confirmed below.

### Prototype discussion — approved first experiment

The user approved a **standalone native screen** before editor integration: the seven bundled/public baseline fonts (including CFF and default-instance CFF2), size 16, all nine AA/hint combinations and three fractional positions. Produce native glyph images, unhinted metrics and actual mode routing against a fresh editor reference. The user also approved including **explicitly labelled thresholded-alpha mono as an experiment**; this is not approval of the resulting rendering or a claim of FreeType-native mono equivalence.

The screen cannot prove real editor cursor alignment, fallback/tab/clipping policy or deferred lifetimes. It does not qualify performance or memory and does not waive styles, format dispositions, either renderer path or live normal-density/genuine-HiDPI review. Use a disposable branch and isolated artifacts; no production dependency/renderer patch or golden update. Record an unavailable control as a gap; stop before writing a new font engine, parser or hinter. No candidate or concrete compatibility loss is approved.

### First native feasibility screen — artifact, not resolution

A standalone harness rendered the agreed cases natively and produced contact sheets and raw metric and routing tables. The results it established are below; this ticket is the approval record.

- Fresh reference source inputs remain `5bd90b47`; default offscreen surface/density 1 with acceleration off. Exact candidate versions are Skrifa 0.48.0 and Zeno 0.3.3, built in the same release/compiler environment with a separate lockfile.
- Both sides execute all **189 cases** (seven fonts × nine AA/hint combinations × three positions), with full repeats matching. Real slight/full output differs for grayscale/subpixel on these samples; the existing mono slight/full equivalence is retained. Unhinted metrics are mode-independent; sampled integer heights match and advance deltas are at most 0.007813 pixels per glyph.
- The proposed alpha-threshold mono has a concrete thin-stroke concern: unhinted CFF2 at p0 renders 中 with **6 vs 42** lit pixels and 国 with **9 vs 77**. All glyph regions being nonempty is not a quality pass. This is evidence for the human's next decision, not an accepted rendering loss or a reason to skip the required CFF2 input.
- Native data/hash/crop/mono/grayscale/mode-effect checks, repeat checks, syntax/format/privacy checks and two independent source/report reviews passed. A successful execution is not qualification; source pixels differ across every reference/candidate frame.

No human result review or next mono-construction decision has occurred. The screen does not prove editor placement/layout/cursor/fallback/lifetimes, styles, footprint/performance/hostile-input safety, the optional renderer, live normal/genuine-HiDPI review or extra-format dispositions. **Keep this ticket claimed and unresolved; backend selection remains blocked.**

### Human review pause

The user initially chose **inspect the sheets before deciding**. At that point the 0.5-threshold mono construction had not been accepted or rejected. The subsequent decision below supersedes that uncertainty for this conversion only; the other results remain unapproved.

### Human decision — reject the 50% alpha-to-mono conversion

After the explanation of thin-stroke loss, the user explicitly said **“ok in that case reject this conversion.”** Reject the current Zeno Alpha conversion (`coverage >= 128 → 255`, otherwise `0`) as the candidate mono implementation. It must not be promoted unchanged or treated as a qualified AA `none` path.

This does **not** reject Skrifa + Zeno as a whole, approve grayscale/subpixel results, drop mandatory CFF2 support, or relax any acceptance gate. The snapshot at `c1949277` remains evidence of the rejected construction, not an implementation to adopt. A replacement mono construction needs a new bounded proposal agreed with the human before further implementation or captures; none is authorized by this rejection alone.

Keep the comparison claimed and unresolved, with backend selection blocked. No production files or reference goldens change.

### Follow-up — establish alternatives before another experiment

[Investigate Rust-native mono rendering alternatives](21-investigate-mono-rendering-alternatives.md) now owns the sharp factual question exposed by this rejection. Its research blocks this comparison until existing-library alternatives and their limitations are established. Further prototype work still requires a newly agreed bounded experiment; no new rendering code or captures are authorized by this investigation.

That research is now [resolved](21-investigate-mono-rendering-alternatives.md#resolution), unblocking the next HITL discussion. Its [report](../assets/font-mono-alternatives.md) establishes a cheap **unhinted-only** any-coverage test using existing raw grayscale coverage, and an alternative tiny-skia non-AA path needing new native work. Neither is an accepted fix. At the research handoff, no further experiment had been agreed or run.

### Approved follow-up — saved-data any-coverage screen

The user chose **“Run the saved-data screen”**: seven existing baseline fonts × three fractional positions = **21 cases**, size 16, **hint none only**. Apply `alpha > 0 → 255`, otherwise `0`, to the saved grayscale coverage. Compare it with current FreeType mono, the rejected 50% conversion as a negative control, and unchanged grayscale context.

This authorizes only derived native-pixel inspection artifacts: no new dependency, font-backend build or capture, no hinted-mono substitution, and no production or golden change. Label the new rule any-coverage mono conversion, not native-mono equivalence. The user has not approved its appearance, weight, geometry or suitability; all qualification gates remain intact.

### Any-coverage screen — captured, awaiting human verdict

The 21-case artifact produced seven four-column contact sheets and 21 native-size derived images, with source and output hashes and counts.

All 63 source hashes match the first screen. Reapplying the rejected 50% rule to the saved grayscale reproduced its mono pixels byte-for-byte in **21/21 cases**. The any-coverage outputs retain every positive quantized sample; bundle-only replays are deterministic on this host. Independent checks verified all 21 native PNGs and all 84 contact tiles against the exact derived/source pixels, with captions outside the retained-ink crop. Focused source/data review passed.

For CFF2 p0 (中/国/A), current FreeType mono has `42/77/26` lit pixels, rejected 50% has `6/9/15`, and any-coverage has `88/121/43`. More retained ink is not a readability score: the new rule may make strokes/counters too heavy. These counts identify what the human should inspect; they do not accept the new appearance.

The 50% construction stays **rejected**. Any-coverage remains **unapproved**, unhinted-only and evidence-only. No font-backend build/capture, dependency, production code or reference golden changed. Keep this ticket claimed and unresolved, with backend selection blocked, until the human's actual review and the remaining qualification work.

### Approved follow-up — tiny-skia non-AA trial

The user chose to follow the recommendation and **run the non-AA experiment**, adding that they cannot make a sound judgement from the glyph images. This authorizes trying the next raster path, not appearance acceptance or delegation/waiver of the existing live-editor review gates. Do not ask the human to infer correctness from pixel counts; provide a technical assessment and, if a candidate survives, use actual editor text for subsequent usability review.

Reuse the bounded slice: **seven baseline fonts × three phases = 21 cases, size 16, hint none only**, with unchanged Skrifa 0.48.0 outlines/metrics and placement. Add pinned **tiny-skia 0.12.0 only to a separate experiment crate**, using its existing non-antialiased fill (`anti_alias = false`; the equivalent explicit `Mask::fill_path` flag is acceptable) rather than thresholding Zeno coverage. Record the dependency graph, routing, binary and source identity, native output and repeats. Compare against the frozen FreeType mono and prior constructions without claiming fresh matched-reference qualification.

No production/dependency adoption, custom rasterizer, forced hinting, synthetic-weight repair, layout change, optional-renderer/performance qualification or format exclusion. The 50% conversion stays rejected; any-coverage is not selected for adoption. The trial and its appearance remain unapproved pending evidence and further decision.

### tiny-skia non-AA trial — captured, technically unsuitable for CFF2 mono

tiny-skia `=0.12.0` (`default-features = false`, `std` + `simd`; no `links`, two Rust build-script crates) was added only to the experiment crate. Recorded Skrifa unhinted outlines were converted verb-for-verb and rasterized with `Mask::fill_path(.., FillRule::Winding, false, identity)`; no Zeno raster, threshold, hinter or emboldening ran on that path.

All 21 cases ran twice with byte-identical output; output is binary and inside the crop; widths/height equal the frozen candidate in 21/21 cases; all 63 frozen inputs and seven font hashes verified. A fresh locked rebuild in another target directory regenerated all 32 artifacts byte-identically.

For glyf/CFF fonts, per-glyph counts sit near FreeType mono and visual inspection found continuous, recognizable glyphs. For the mandatory **CFF2** sample, FreeType keeps `42/77/26` (中/国/A) at every phase, while tiny-skia keeps `12/33/18`, `32/29/17` and `38/38/18` by phase. Image inspection found missing strokes: at p0, 中 is reduced to roughly a horizontal bar and 国 to a broken partial box; p1/p2 remain fragmentary. This fails the same required AA-`none` functionality as the rejected 50% conversion, and it is phase-dependent where FreeType is stable.

Assistant assessment, not a human decision: neither examined Rust-native route (threshold/any-coverage on Zeno, tiny-skia non-AA) gives usable mono for the mandatory CFF2 font. Per the research boundary, this is escalated to the human rather than solved with a custom rasterizer or relaxed requirements. Ticket stays claimed and unresolved; backend selection stays blocked.

### Human decision — tiny-skia non-AA accepted as the mono construction

The user accepted some degradation in really thin fonts and chose the best construction tested so far. Recorded choice: **tiny-skia 0.12.0 non-AA fill** (`fc143432`) is the candidate's AA-`none` (mono) construction, a disclosed backend-native algorithm rather than FreeType-equivalent mono.

Accepted difference: thin-stroke loss in very thin fonts under AA `none`, as observed for the Noto Sans CJK CFF2 default (Thin) instance at size 16 unhinted, including phase-dependent output and characters that can lose whole strokes. This does not drop CFF2 support: the CFF2 font must still load, measure and draw, and grayscale/subpixel must still render it.

Scope limits: approval covers only the unhinted mono construction tested. Hinted mono (slight/full via Skrifa `Target::Mono` + tiny-skia), other sizes, regular-weight stroke loss, editor integration and every remaining gate (geometry/cursor, performance, memory, malformed input, `sdl_renderer`, normal and genuine HiDPI real-window review, format dispositions) remain open. The 50% conversion stays rejected; any-coverage is not selected. Ticket stays claimed and unresolved; backend selection stays blocked.

### Approved follow-up — hinted mono through tiny-skia

The user approved the next trial: the same seven fonts × three phases at size 16, now with AA `none` and hinting `slight` and `full` (42 cases). Skrifa 0.48.0 hints with the existing `Engine::Auto(None)` + `Target::Mono` route; tiny-skia 0.12.0 non-AA fill rasterizes the hinted outline. Advances stay unhinted, as in the editor. Mono slight and full must stay equivalent, as they are in the current renderer. Compare against the frozen FreeType hinted-mono reference. Throwaway branch only; no production, dependency adoption, forced-weight repair or golden change. The results' appearance is not pre-approved.

### Hinted mono trial — captured, strokes retained

Skrifa `Engine::Auto(None)` + `Target::Mono` hinted outlines, rasterized by the accepted tiny-skia 0.12.0 non-AA fill; advances unhinted.

42/42 cases ran twice byte-identical, with binary output inside the crop and widths/height equal to the frozen candidate. Hinting slight and full produce identical images in 21/21 pairs, preserving the editor's mono slight = full equivalence. The 84 FreeType/rejected inputs are frozen captures hash-checked against the published table, not a fresh paired run.

CFF2 中/国/A: FreeType `51/83/26` at every phase; tiny-skia hinted `51/83/17`, `51/96/16`, `59/97/17`. Visual inspection found 中 and 国 continuous with open counters, near FreeType; the thin-stroke loss accepted for unhinted mono does not appear in hinted mono at this size. Remaining differences are phase-dependent single-pixel changes (lighter CJK-font A, heavier TeX Gyre `g`). This is evidence, not approval; the hinted-mono appearance has not been reviewed by the human. The mono construction now covers all three hint modes, so the next step is live-editor integration. Ticket stays claimed and unresolved; backend selection stays blocked.

### Human decision — scope split and next steps

The user chose:

- **Next experiment:** an in-editor prototype integration, because glyph sheets do not let them judge readability.
- **Ticket structure:** resolve this ticket on per-glyph decisions; move live review, geometry, performance, memory, malformed-input, `sdl_renderer` and HiDPI evidence to a new in-editor prototype ticket.
- **Format gaps:** research first. Skrifa 0.48.0 outlines are only Glyf/Cff/Cff2/Varc, so BDF and Type 1/PFB (both working under FreeType today) have no candidate path; bitmap strikes are readable by Skrifa but untested. No exclusion is approved.
- **Synthetic bold/smoothing:** try a Zeno stroke-and-fill approximation in the editor prototype, disclosed as a different algorithm from FreeType's outline embolden. Italic is a shear transform.

## Resolution

Per-glyph outcome for **Skrifa 0.48.0 + Zeno 0.3.3 + tiny-skia 0.12.0** (no backend selected):

- **Parsing and metrics:** glyf, CFF/OTTO and CFF2 collection face 0 load; heights match FreeType for all seven fonts and unhinted advances differ by ≤0.008 px ([first screen](#first-native-feasibility-screen--artifact-not-resolution), `c1949277`).
- **Control routing:** hinting none/slight/full and AA mono/grayscale/subpixel route through Skrifa `Engine::Auto(None)` with `Target::Mono` / `SmoothMode::Light` / `SmoothMode::Normal`, mirroring the editor's FORCE_AUTOHINT policy. Mono slight = full is preserved; grayscale/subpixel slight ≠ full as in the reference.
- **Mono construction (user-accepted):** tiny-skia non-AA fill (`Mask::fill_path`, `anti_alias = false`) on Skrifa outlines, hinted or unhinted, disclosed as backend-native rather than FreeType-equivalent. Accepted difference: thin-stroke loss in very thin fonts in **unhinted** mono (Noto Sans CJK Thin at size 16 loses strokes, position-dependent). In hinted mono the same strokes were retained (`07d96bee`). Rejected: 50% alpha threshold; not selected: any-coverage.
- **Grayscale/subpixel:** candidate images execute in all cases but are not approved. Zeno subpixel is not the editor's five-tap LCD filter; that and appearance in general are judged live in [Prototype the candidate backend inside the editor](23-prototype-candidate-in-editor.md).
- **Styles:** italic = shear; bold/smoothing = Zeno stroke-and-fill approximation, to be tried in the editor prototype.
- **Formats:** BDF and Type 1/PFB have no candidate path yet; EBDT/CBDT strikes are readable by Skrifa but unexercised. Facts go to [Investigate Rust-native paths for legacy and bitmap font formats](22-investigate-rust-native-legacy-and-bitmap-formats.md); dispositions belong to [Select the Rust-native font backend](16-select-rust-font-backend.md).

Evidence: the 189-case native screen, the any-coverage conversion artifact, and the unhinted and hinted tiny-skia mono comparisons, all described above.

Everything in the question beyond per-glyph behavior (live normal/HiDPI review, geometry/cursor, fallback/tabs, performance, memory, malformed input, both renderer paths) is **not** resolved here; it moved, unchanged, to the in-editor prototype ticket. No acceptance gate is relaxed, no production code changed, and backend selection stays blocked on the two new tickets.
