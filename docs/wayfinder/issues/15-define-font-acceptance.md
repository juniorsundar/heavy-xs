# Define font compatibility and acceptance criteria

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-font-acceptance
Blocked by: 12, 13, 14

## Question

With the inventory, Rust-library survey and reference evidence available, which font behaviors must remain equivalent, which measured differences may be reviewed, and what acceptance checks must a replacement pass?

Discuss concrete scenes and gaps with the user. Cover bundled/custom-font scope and exclusions, Lua font API/options, drawing-versus-measurement consistency, text advances/line geometry, hit testing/cursor alignment, fallback and missing glyphs, hinting/subpixel modes, synthetic styles, loading errors/lifetimes and cache/memory behavior. Permission to consider changed metrics or pixels is not approval of any particular change.

Calibrate performance, responsiveness, memory/retention and visual-review criteria from the baseline; separate load/first-glyph and warm paths, ordinary and stress, headless evidence and live-window review. Agree comparison/build identities and what counts as a no-go. Old regex thresholds and exact FreeType pixels are not implicit gates.

Identify the candidate prototype/evidence questions now concrete enough to ticket, create those follow-ups and wire them as explicit blockers of backend selection. Resolve only through the user's actual answers. This resolution approves criteria, not a library or implementation.

Prerequisites: [Inventory the observable font contract](12-inventory-font-contract.md), [Survey Rust-native font backends](13-survey-rust-font-backends.md), and [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md). This decision gates [Select the Rust-native font backend](16-select-rust-font-backend.md).

## Comments

### Acceptance discussion — round 1 (user answers)

The user chose all four recommended answers:

- **Geometry:** changed advances and line heights, including changed wrapping, are eligible for explicit human review. Drawing, selection, cursor placement and hit testing must remain internally consistent. This is not approval of any particular change.
- **Minimum formats:** bundled fonts and configurable custom fonts spanning the baseline's TrueType/glyf, OpenType/CFF and variable CFF2 collection are required. The collection is used at face 0 and its default instance; no new face/axis API is required. Other formats remain unresolved coverage, not approved removals.
- **Controls:** all existing AA, hinting and style options must remain functional. Backend-native algorithms may be compared; exact FreeType algorithms are not required. Mappings must be disclosed, with no silently ignored options or new aliases. Existing equivalent modes need not become distinct.
- **Visual gate:** selection requires the user's real-window review at normal density and on a genuine HiDPI surface, in addition to headless comparisons. Enlarging the logical font does not satisfy the HiDPI gate. Unavailable hardware leaves the gate pending.

[Text geometry and surface density](../../../GLOSSARY.md) distinguish layout changes from pixel changes, and genuine HiDPI from enlarged text. Cost, retention, remaining format coverage and detailed behavior gates are still open; this ticket is not resolved.

### Acceptance discussion — round 2 (user answers)

The user chose all four recommended answers:

- **Valid-call behavior:** preserve call/return shapes, copy/resize and inherited-option behavior, tab-stop rules, fallback order, whitespace using the first font, last-font `.notdef` and deferred-draw rooting. Geometry remains subject to the round-1 review rule; changing the backend does not authorize new layout policy or shaping.
- **Performance policy:** ordinary warm editing/scrolling must meet a tested real-display frame budget. Report full-viewport p95/p99 and longest blocking time, and separate cold/first-glyph, warm and stress measurements. Repeatable regressions and long stalls require review with absolute and relative differences; there is no universal percentage allowance. Small-scene timings alone cannot pass this gate.
- **Memory policy:** candidate-owned leaks or unbounded fixed-workload cache growth are no-go conditions. Explained, bounded allocator RSS retention may be accepted. Inherited shared-renderer defects need an explicit migration disposition rather than automatically disqualifying a backend. Unexplained growth cannot be recorded as a memory pass; attribution remains with the parallel investigation.
- **Additional formats:** relevant support gaps beyond the mandatory baseline require the user's explicit disposition as required support, approved exclusion or blocker before selection. Missing investigations are not compatibility evidence, and untested formats are not silently excluded.

These are agreed policies, not a candidate approval. Detailed frame-budget checks, failure behavior and comparison/build requirements remain to settle.

### Acceptance discussion — round 3 (user answers)

The user chose all four recommended answers:

- **Frame gate:** ordinary warmed editing/scrolling must have p95 full-frame processing at or below one tested display interval (about 16.7 ms at 60 Hz or 8.3 ms at 120 Hz). Hold display settings fixed across reference/candidate runs. Report p99 and maximum blocking times; overruns and repeatable reference regressions require explicit review before selection. Cold loading/first-glyph and stress cases are separate; aggregate averages must not hide stalls.
- **Failure behavior:** preserve Lua argument checks, result arity and error prefixes; FreeType-specific diagnostic details need not match. Corrupt fonts must fail cleanly, and malformed UTF-8 text must use defined safe replacement behavior, consistent in measurement and drawing, rather than crashing or aborting the draw. Test corrupt/truncated fonts and bounded hostile cases. A timeout or external kill is not a passing robustness result.
- **Comparison identity:** freshly pair reference/candidate release builds from the same base, compiler/profile, non-font code, SDL, features, scenes and font hashes. Pin reference FreeType and candidate dependency versions/features, disclose intentional dependency changes, alternate same-host runs and repeat noisy cases. The historical baseline remains evidence, not the sole performance control. Do not rewrite goldens to manufacture a pass.
- **Build coverage:** both default surface rendering and the optional `sdl_renderer` feature must build and pass relevant API, geometry and lifetime checks. Demonstrate rendering on both and perform live density review on supported real-window paths. Missing execution coverage is a blocker unless the user explicitly waives it.

The criteria still require a final shared-understanding confirmation before resolution. No backend, concrete geometry difference or format exclusion has been approved.

### Acceptance discussion — round 4 (user answers)

The user chose all three recommended clarifications:

- **Timing boundary:** the frame-budget percentile measures active whole-frame update/layout/draw work, excluding intentional idle/frame pacing. Presentation blocking and observed delivery stalls are reported separately, along with p99/max and missed/stalled frames. CPU timers do not establish input-to-photon latency.
- **Derived metrics:** preserve rules, not frozen derived numbers. Explicit requested sizes, option inheritance and tab/fallback rules stay intact. Heights, baselines and advances, and their effects under those rules, may change through the geometry-review process. For example, omitted-size `copy()` must still use font height, but its resulting size may change with an approved height change. Leave reference goldens unchanged during comparison; classify candidate differences and obtain approval rather than treating changed hashes alone as automatic failures or rewriting expected output to hide regressions.
- **Footprint:** repeatable increases in peak and retained memory require explicit human review even if resources release and the working set stabilizes. Report absolute/relative footprints for matched workloads, including repeated loads/copies of the large collection. Leak-free is necessary, not sufficient; no universal byte cap is presumed.

Only final confirmation of the complete criteria and evidence handoff remains.

### Resolution — confirmed by the user

The user explicitly confirmed the complete criteria and evidence handoff after the four discussion rounds above. These are acceptance requirements, not approval of a backend, concrete metric/pixel differences or format exclusions.

#### Compatibility and safety

- **Required inputs:** the bundled fonts and configurable custom fonts covering TrueType/glyf, OpenType/CFF and the baseline's variable CFF2 collection at face 0/default instance. No new face/axis-selection API is required.
- **Required controls:** all current antialiasing, hinting and style flags remain functional. Compare backend-native algorithms rather than demanding FreeType's exact algorithms. Disclose mono construction, LCD filtering, hinting and style mappings; do not introduce silent no-ops or new aliases. Existing equivalent modes need not become distinct.
- **API rules:** retain valid-call argument/result contracts, explicit requested sizes, copy/resize and option-inheritance rules, tab stops, fallback order, first-font whitespace, last-font `.notdef`, and deferred-draw rooting/release. No implicit shaping or fallback redesign.
- **Geometry and appearance:** advances, baselines, heights, wrapping and pixels may differ only through explicit review of the actual candidate evidence. Preserve the rules that derive further results from those metrics: omitted-size `copy()` still uses font height, for example. Drawing, returned pen positions, measurements, selection, cursor placement and hit testing must agree; classify rounding separately from cumulative drift or inconsistent layout.
- **Errors and invalid data:** retain Lua argument checks, return arity and error prefixes, not FreeType-specific diagnostic details. Corrupt/truncated fonts must fail cleanly; malformed UTF-8 text must use defined safe replacement behavior consistently in measurement and drawing. Crashes, hangs, or external containment kills do not pass bounded hostile-input tests.

#### Responsiveness and memory

- **Ordinary warm work:** p95 active whole-frame update/layout/draw processing must fit one interval of the tested display (`1000 / refresh_Hz` milliseconds). Declare representative full-viewport editing/scrolling workloads and count actual active work rather than filling the percentile with idle frames. Exclude deliberate idle/frame-pacing waits; report presentation blocking and observed delivery stalls separately. Do not infer input-to-photon latency from CPU timers.
- **Regressions and stress:** report p99/max and blocking stalls, and compare absolute and relative costs. Repeatable reference regressions and overruns require human review even when p95 passes. Separate fresh load, first glyph/metrics/raster, warmed caches/drawing and stress. Report sample counts, warm-up and workload units; averages or tiny scene timings cannot stand in for viewport responsiveness. No universal slowdown percentage was approved.
- **Ownership and release:** candidate-owned leaks and unbounded fixed-workload cache growth are no-go conditions. Explained bounded allocator retention may be acceptable; unexplained growth is not a memory pass. Use the parallel retention investigation to distinguish inherited shared-code defects, which need an explicit migration disposition rather than automatically disqualifying a backend.
- **Footprint:** compare absolute/relative peak and retained usage for matched working sets, including repeated loads/copies of the large collection. Stable or leak-free does not automatically mean acceptable: repeatable footprint increases require human review. No universal byte cap was approved.

#### Comparison and approval gates

- Freshly pair reference/candidate release builds from the same base, compiler/profile, non-font code, SDL, feature path, scenes and font hashes. Pin reference FreeType and the candidate graph/features; disclose intentional dependency changes. Alternate same-host runs and repeat noisy cases. Keep the historical baseline as evidence, not the sole control for newer builds.
- Preserve reference golden expectations during comparison. Separate invariant failures from metric/pixel differences needing approval; do not rewrite reference results to manufacture a pass. This planning resolution does not authorize production golden updates.
- Both default surface rendering and `sdl_renderer` must build, pass relevant API/geometry/lifetime checks and demonstrate rendering. Selection requires the user's real-window code/UI/icon/fallback/style/alignment review at normal density and on a genuine HiDPI surface on a supported path. Record the feature path and actual density. Enlarged logical fonts are not HiDPI evidence.
- Missing required execution or unavailable HiDPI hardware leaves selection blocked unless the user explicitly revises the relevant gate. A future waiver of one build path would not implicitly waive the separate HiDPI requirement.
- All relevant additional-format gaps and unknowns must receive explicit user disposition as required support, approved exclusion or blocker before selection. Neither a library's advertised format support nor an untested current-format case is an automatic pass. The user did not require every FreeType format and did not approve silent exclusions.

**No-go conditions** include failed mandatory inputs/controls, changed API rules without approval, inconsistent geometry, crashes/hangs, candidate-owned leaks or unbounded caches, failure of the ordinary warm p95 budget, and unresolved required evidence/approval gates. Passing automated checks cannot substitute for the user's appearance and compatibility answers.

#### Evidence handoff

[Establish additional font-format reference coverage](20-establish-additional-font-format-coverage.md) is a new AFK task for a bounded extra-format reference matrix and explicit unknowns. [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md) consumes that reference evidence and [Classify font memory retention in the current renderer](19-classify-font-memory-retention.md), implements these comparison checks in a disposable prototype and records the user's actual geometry, appearance and format-gap dispositions.

[Select the Rust-native font backend](16-select-rust-font-backend.md) remains blocked on that evidence. Production adoption and any fixes remain a separate handoff.
