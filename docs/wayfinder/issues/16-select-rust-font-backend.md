# Select the Rust-native font backend

Parent: ../freetype-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-font-backend-selection
Blocked by: 13, 15, 18, 20, 22, 23

## Question

Which Rust-native font backend or small composition satisfies the agreed criteria on actual candidate evidence, and which specific observed differences does the user accept?

Use the survey, agreed acceptance criteria and subsequent user-reviewed prototype evidence. The survey alone cannot select a backend. Any needed prototype/measurement tickets must be explicit blockers before this ticket is worked; link their reports rather than assuming results.

Record the chosen dependency versions and native-dependency audit, ownership seam against the existing Rust renderer/cache/Lua API, concrete approved metric/pixel/option/format differences, rejected alternatives and remaining risks. Explain how the FreeType dependency and raw FFI leave the build while SDL3/Lua stay.

If no existing Rust-native composition meets the reviewed contract, return concrete conflicts to the user and pause. No implicit fork, parser/rasterizer project, translator or C fallback. Selection requires live user approval and does not authorize production implementation.

Prerequisites: [Survey Rust-native font backends](13-survey-rust-font-backends.md), [Define font compatibility and acceptance criteria](15-define-font-acceptance.md), and [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md). [Establish additional font-format reference coverage](20-establish-additional-font-format-coverage.md) is also an explicit prerequisite; the comparison must present its candidate gaps and unknowns for the user's disposition. Any additional evidence blockers exposed by that comparison must also be resolved before selection. Selection unblocks [Define the FreeType replacement handoff](17-plan-font-migration.md).

## Comments

The user confirmed [the acceptance criteria](15-define-font-acceptance.md#resolution--confirmed-by-the-user). Selection needs both renderer paths, matched full-viewport cost/resource evidence, explicit format-gap dispositions and the user's normal-density/genuine-HiDPI review. Surveys, headless sheets or changed goldens alone cannot satisfy these gates. Missing evidence leaves selection blocked; no backend, observed rendering difference or format exclusion has been approved.

Now also blocked by [Investigate Rust-native paths for legacy and bitmap font formats](22-investigate-rust-native-legacy-and-bitmap-formats.md) and [Prototype the candidate backend inside the editor](23-prototype-candidate-in-editor.md). The per-glyph comparison is [resolved](18-compare-font-controls-and-rendering.md#resolution); live, performance, memory, renderer-path and HiDPI evidence moved to the in-editor prototype. BDF, Type 1/PFB and bitmap-strike dispositions are decided here, using the format research.

### Grilling round 1 — answers

- **Backend:** yes to Skrifa 0.48.0 + Zeno 0.3.3 + tiny-skia 0.12.0, exact versions, pure Rust.
- **Normal-density review on the ~109 dpi DP-7 monitor:** **no**. How the gate is then disposed is asked in round 2.
- **Formats:** yes. Exclude Type 1/PFB, BDF, PCF and EBDT/CBDT bitmap strikes; each must fail cleanly with `failed to load font:`.
- **Approved differences:** yes to the list (glyph pixels in all modes, unhinted thin-font mono stroke loss, Zeno subpixel method, stroke-approximated smoothing, malformed-font accept/reject split). **Changed by the user:** size-scaled synthetic bold is withdrawn. The user prefers real bold faces: "do not hard code artificial boldening into the codebase; the user loads the bold face fonts instead." The consequences for the `bold` option are asked in round 2.
- **Follow-ups for the handoff:** yes (CJK peak memory, `sdl_renderer` fractional-scale crash, cold/first-glyph timing, the mono overread disappearing with FreeType).
- **ADR:** yes, to be written once the round settles.

**Withdrawn by the user:** the real-bold-face-only request. Removing synthetic emboldening would break the `bold` font option (the bundled markdown plugin and user configs call `copy(size, {bold = true})`), a significant deviation from current behavior. Synthetic bold stays; its strength is asked in round 2. Loading real bold faces remains available through configuration.

### Grilling round 2 — answers

- **Synthetic bold strength:** yes. Keep the size-scaled synthetic bold (thicken by `pixel_size / 20`), as approved during the in-editor prototype. The `bold` option keeps working; real bold faces remain a configuration choice.
- **Normal-density review:** **(a) waived** by the user. The live HiDPI review on the laptop panel and identical text geometry are sufficient for selection. Normal-density appearance is recorded as **not reviewed**; this is an explicit revision of the visual gate in [Define font compatibility and acceptance criteria](15-define-font-acceptance.md#resolution--confirmed-by-the-user), not evidence that it passes.

The user confirmed the full selection summary ("yes").

## Resolution

Confirmed by the user. Recorded as [ADR 0001: Replace FreeType with Skrifa + Zeno + tiny-skia](../../adr/0001-rust-native-font-backend.md).

- **Selected:** Skrifa `=0.48.0` (parse, metrics, `Engine::Auto(None)` autohint), Zeno `=0.3.3` (grayscale/subpixel), tiny-skia `=0.12.0` with `default-features = false, features = ["std", "simd"]` (non-AA mono). The audited closure is pure Rust, with no `links=` and no C/C++ build ([survey](13-survey-rust-font-backends.md), [mono trial audit](18-compare-font-controls-and-rendering.md#resolution)).
- **Seam:** only the FreeType touchpoints in `src/renderer.rs` change (glyph id, advance, glyph image, face metrics, load), as demonstrated in the [in-editor prototype](23-prototype-candidate-in-editor.md#resolution). The atlas, glyph cache, fallback, tabs, blending, decorations and Lua API stay. `freetype-rs`, bundled FreeType and `src/ft.rs` leave the build; SDL3, Lua and the native-plugin ABI stay.
- **Approved differences:** glyph pixels in all AA/hint modes; size-scaled synthetic bold (`pixel_size / 20`); unhinted-mono thin-stroke loss; Zeno subpixel instead of the five-tap LCD filter; stroke-approximated smoothing; a different accept/reject split for malformed fonts (always clean). Text geometry is unchanged.
- **Excluded formats:** Type 1/PFB, BDF, PCF, EBDT/CBDT bitmap strikes. They fail cleanly with `failed to load font:` ([format research](22-investigate-rust-native-legacy-and-bitmap-formats.md#resolution)).
- **Gate revision:** the normal-density visual review is waived by the user and recorded as not reviewed. All other acceptance gates are satisfied or dispositioned in the in-editor prototype.
- **Rejected:** stock Swash, fontdue, ab_glyph, the 50% and any-coverage mono conversions, FreeType-for-mono, and real-bold-face-only.
- **Follow-ups for the handoff (non-blocking):** the CJK peak memory from per-font hinting instances; the inherited `sdl_renderer` fractional-scale abort; cold/first-glyph timing; the FreeType packed-mono overread, which disappears with FreeType.

Selection authorizes no production change; implementation is planned in [Define the FreeType replacement handoff](17-plan-font-migration.md).
