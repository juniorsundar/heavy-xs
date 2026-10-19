# Prototype the candidate backend inside the editor

Parent: ../freetype-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: pi-font-editor-prototype
Blocked by: 18

## Question

Does the per-glyph candidate composition from [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md#resolution) hold up behind the real renderer, and which live differences does the user accept?

Build a throwaway integration on an isolated prototype branch: Skrifa 0.48.0 parsing/metrics/hinting, Zeno 0.3.3 grayscale/subpixel, tiny-skia 0.12.0 non-AA mono, behind the existing renderer/cache/Lua API seam, with FreeType selectable for paired comparison. Styles: italic as a shear transform, bold and smoothing as a Zeno stroke-and-fill approximation, disclosed as a different algorithm from FreeType's outline embolden. Agree the workload manifest with the human before building.

Produce the evidence that [Define font compatibility and acceptance criteria](15-define-font-acceptance.md#resolution--confirmed-by-the-user) requires and the human can judge in a real window:

- Live normal-density review of real editor text in all AA/hint modes and styles; genuine HiDPI review where hardware exists (otherwise recorded as pending).
- Geometry: advances, line height, cursor, selection and hit-test consistency; tabs, fallback/.notdef, icons, clipping and deferred-draw lifetimes.
- Paired full-viewport editing/scrolling performance and cold/first-glyph cost; matched memory with framebuffer acceleration off; corrupt/truncated-font and malformed-text checks.
- Default surface and `sdl_renderer` build/runtime paths.

No production adoption, removal of FreeType, custom rasterizer, shaping or fallback redesign. Escalate rather than write engine code if the adapter grows beyond a small composition. Format dispositions belong to [Select the Rust-native font backend](16-select-rust-font-backend.md), informed by [Investigate Rust-native paths for legacy and bitmap font formats](22-investigate-rust-native-legacy-and-bitmap-formats.md).

## Comments

### Agreed workload — first increment

The user chose:

- **Review method:** one prototype binary; `LITE_FONT_BACKEND=freetype|candidate` selects the backend at startup. Compare two windows side by side on the same file. The same binary serves later paired measurements.
- **First increment:** a live-review build. The candidate sits behind the existing renderer seam with all AA/hint modes, styles, fallback, tabs and icons, plus automated geometry/cursor/contract checks against FreeType. Performance, memory, malformed fonts and `sdl_renderer` follow as later increments on the same branch.
- **Fonts:** the user's own configuration (UI and code fonts at grayscale/full) for daily-use judgement, plus bundled fonts at the defaults (subpixel/slight). The user's font files and paths stay private; only results are recorded.
- **Density:** the laptop panel (2560x1600, about 220 dpi, KDE scale 1.25) covers the genuine HiDPI review. No normal-density display is available, so the **normal-density review is pending** and remains a selection blocker unless the user explicitly revises that gate.

### Increment 1 — live-review build, awaiting the human's review

Local throwaway branch `prototype/font-editor-candidate`, commit `07b93ca9` (from base `5bd90b47`; no push or merge). Report: `docs/wayfinder/assets/font-editor-prototype/README.md` on that commit.

`LITE_FONT_BACKEND=candidate` swaps FreeType for Skrifa 0.48.0 + Zeno 0.3.3 + tiny-skia 0.12.0 at each FreeType touchpoint in the renderer (glyph id, advance, bitmap, face metrics, load). The atlas, glyph cache, fallback, tabs, blending, decorations and Lua API are shared. Styles: italic as the same shear; bold and smoothing as fill plus a 0.5 px stroke.

Automated evidence:

- FreeType path: all 16 contract suites pass, so the seam does not regress the reference.
- Candidate path: `rencache` passes; `renderer` differs only in its 80 pixel-hash rows. Load-error wording and size-0/negative behavior were brought into line with FreeType. The 23 font-inventory assertions pass on both backends.
- Geometry: bundled JetBrains Mono and Fira Sans plus the user's UI and code fonts, sizes 12/15/19, all nine AA/hint modes, code/UI/tab/mixed-script samples with fallback. 13,068 invariant checks pass per backend (draw end equals measured width, per-codepoint sums, monotonic prefixes). Heights and every measured width are **identical** between backends, so no geometry difference needs approval for these fonts.
- The user's full configuration, plugins included, starts under both backends with no logged warnings or errors.

Pending: the user's side-by-side live review on the HiDPI panel (`review.sh`, configs `user`, `bundled`, `mode=AA/HINT`). Later increments: paired performance and memory, malformed fonts, `sdl_renderer`. Normal-density review is pending (no display). No appearance is approved.

### Human decision — visible, size-scaled synthetic bold

Live review: the user could not see synthetic bold on either backend, even zoomed in. Measured cause: FreeType thickens by a fixed 0.5 px regardless of size (Lilex at 15 px: 188 → 203 lit pixels for `test`).

The user chose **stronger, size-scaled bold** for the candidate: thicken by `pixel_size / 20` (0.75 px at 15 px, 1.2 px at 24 px). Commit `b7b87900` on `prototype/font-editor-candidate`. For Lilex, the ink then lands close to the real Lilex Bold face (size 15: 122k vs 118k; size 24: 319k vs 281k), against FreeType's 94k. Advances are unchanged, so layout, wrapping and cursor placement do not move; the geometry probe output is byte-identical. This is an approved, recorded difference from FreeType's synthetic bold. Smoothing keeps FreeType's 0.5 px.

The user also asked about using a font's real Bold file. Automatically finding a sibling Bold file is font discovery, which this map rules out of scope. It is already possible through configuration (`style.syntax_fonts[...]` pointing at a Bold file), and works the same on both backends.

For a like-for-like comparison, commit `bbe1b6d3` adds `LITE_FT_SCALED_BOLD=1`, which gives the FreeType path the same size-scaled strength (on both axes, as the candidate's stroke does). Measured ink then agrees within about 1–8% across sizes and the user's two fonts. Unset, FreeType output is unchanged and the contract suites still pass. The opt-in is a comparison aid only, not a proposed FreeType change.

### Human review — increment 1 live appearance

After side-by-side review on the HiDPI laptop panel (including the size-scaled bold comparison), the user reported: **"Candidate looks good, I don't see any problems."** Recorded as acceptance of the candidate's live appearance, with the size-scaled bold, at genuine HiDPI density, for the configurations the user looked at. The user confirmed which configurations they reviewed: their own config (grayscale/full), the bundled defaults (subpixel/slight), mono (`mode=none/...`) and bold/italic text. All are accepted as viewed. Pixel differences from FreeType in these modes need no further approval.

Still open on this ticket: paired performance and memory, malformed fonts, `sdl_renderer` build/runtime. Normal-density review stays pending (no display). This ticket stays claimed and unresolved; backend selection stays blocked.

### Increment 2a — paired real-window frame timing (display mismatch)

Commit `c9c40f76` on `prototype/font-editor-candidate` adds `frame-probe.lua` (per-frame active work and presentation, timed in Rust around `ren_update_rects`) and `frame-run.sh` (FreeType/candidate alternating over 3 rounds, the user's config and bundled fonts, plain-text copy of `src/renderer.rs`, editor fps 60). Raw TSVs are in `frames/`.

The maximized windows opened at 2543x2033 on an **external 60 Hz monitor**, not the 165 Hz laptop panel, so the agreed 6.06 ms budget was not tested on its own display. One run (bundled candidate, round 1) was disturbed: 627 instead of about 1,080 scroll frames, with 29 ms presents. It is excluded below and needs a rerun.

Per-run p95 active work, ms (excluding the disturbed run):

| Workload | FreeType | Candidate |
|---|---|---|
| User config, scroll | 8.49 / 8.69 / 9.24 | 8.04 / 7.51 / 7.22 |
| User config, typing | 5.79 / 10.83 / 5.53 | 4.09 / 4.27 / 4.11 |
| Bundled, scroll | 7.47 / 5.60 / 7.72 | 4.95 / 6.71 |
| Bundled, typing | 4.14 / 3.33 / 4.08 | 8.76 / 2.96 |

The candidate is not slower than FreeType on ordinary work, and both fit a 60 Hz interval (16.7 ms). **Neither backend** meets 6.06 ms on scrolling with the user's config, so the 165 Hz gate fails for the reference too; most of the frame is shared editor work. Pending: a rerun on the laptop panel, and memory.

### Human decision — performance evidence accepted

The user accepted the paired run as trustworthy: apart from the one disturbed run (excluded), all runs had the same conditions. Their conclusion: **no performance degradation**. Recorded:

- The tested display is the 60 Hz external monitor the windows actually opened on. Under the agreed gate (p95 active work within one interval of the *tested* display), both backends pass: worst p95 is 10.83 ms for FreeType and 8.76 ms for the candidate, against 16.7 ms.
- The earlier 165 Hz / 6.06 ms target is dropped for this comparison. Neither backend meets it while scrolling with the user's config, and most of that frame is shared editor work rather than font work.
- Presentation timings were comparable between backends, apart from the excluded run.

Cold-start/first-glyph cost and a large-collection stress case were not separately timed; the frame run includes first-glyph frames only in its unscored warm-up.

### Increment 2b — paired memory, awaiting the human's review

Commit `b44b7914` on `prototype/font-editor-candidate`. The existing retention probe (`font-retention-classify.lua`, unchanged) ran for both backends: JetBrains Mono and the 32 MB Noto Sans CJK collection; variants `full` (load, set_size, copy, draw), `hold8` (eight fonts live at once), `draw` and `width`; 200 cycles each, two rounds, offscreen with framebuffer acceleration off and the glibc stats shim. Raw TSVs are in `memory/`.

**Defect found and fixed:** the candidate copied each font file onto the heap (32 MB per Noto CJK load; 326 MB peak with eight fonts live). It now maps the file read-only, as FreeType streams it. Load-error wording is unchanged.

After the fix (MiB; ranges over the two rounds):

| Case | Steady RSS, FreeType → candidate | Peak RSS, FreeType → candidate | After trim |
|---|---|---|---|
| Mono, all variants | 14.1–15.2 → 14.4–15.5 | 14.1–15.2 → 14.4–16.0 | equal ±0.4 |
| CJK draw / width / full | 14.4–14.8 → 14.5–15.0 | 14.5–15.4 → **15.2–23.8** | equal ±0.3 |
| CJK eight live (`hold8`) | 20.2–20.4 → **15.6–15.8** | 20.2–20.4 → **52.1** | equal ±0.3 |

- No candidate-owned leak: fonts are released every cycle (the probe asserts this), the heap in use stays at about 0.35 MB, and RSS is flat after cycle 20 apart from small allocator steps that `malloc_trim` returns.
- **Steady state is equal or lower** for the candidate (8 live CJK fonts: 15.7 vs 20.3 MiB).
- **Peak is higher** with the large CJK collection: up to +8 MiB with one font and +32 MiB with eight live. Each candidate font builds a per-font hinting instance (`HintingInstance::new`, autohinter) at load and on every `set_size`, which costs transient memory that is freed afterwards. FreeType's hinter state is per glyph slot.

Under the agreed policy, the repeatable peak increase requires explicit human review even though it is released.

### Human decision — peak-memory spike deferred

The user deferred the CJK peak-memory increase ("this can be touched later"). Recorded as **accepted for now, not waived**. Steady memory and the no-leak result are accepted. The peak (up to +8 MiB with one large CJK font, +32 MiB with eight live, released afterwards) goes to the migration plan as a follow-up: reduce per-font hinting-instance cost (for example, build it lazily or reuse it across `set_size`), then re-measure. It does not block backend selection unless the user revisits it.

### Increment 3 — malformed fonts

Commit `9723a24b` on `prototype/font-editor-candidate` adds `malformed-run.mjs` and `malformed-probe.lua`. The corpus has 114 deterministic mutations: empty/zero/random/text files; truncation at 0.1–99%; 16/256/4096 random byte flips; and table-directory length/offset damage and 0xFF-filled `head`/`hhea`/`maxp`/`cmap`/`loca`/`glyf`/`hmtx`/`CFF `/`CFF2`/`OS/2`/`post`. The sources are bundled JetBrains Mono and Fira Sans, public TeX Gyre Heros CFF and the Noto Sans CJK CFF2 collection. Each case runs in its own process with 20 s wall, 30 s CPU and 2 GiB address-space limits. A successful load is then exercised with set_size 1–300, malformed UTF-8, fallback codepoints, 200-glyph lines and copies in all AA modes with bold+italic.

Result (`malformed/results.tsv`): **no crash, signal, timeout or Lua error on either backend**. Every case either failed to load with the existing `failed to load font:` prefix (FreeType 48, candidate 50) or loaded and drew safely. Eighteen cases differ only in which side of that line they land: for example, FreeType rejects a corrupt `maxp` while the candidate loads and draws, and the candidate rejects a corrupt `glyf`/`hmtx` length while FreeType loads. Both outcomes are acceptable under the failure policy, which preserves the error prefix, not FreeType-specific diagnostics.

Remaining on this ticket: the `sdl_renderer` build/runtime path.

### Increment 4 — `sdl_renderer` build path

Built `--features sdl_renderer` from the same prototype commit. Headless (dummy video, surface scale 1):

- FreeType path: all 16 contract suites pass. Candidate path: 15 pass; `renderer` differs only in its 80 pixel-hash rows, as on the default build.
- The geometry probe (13,068 invariant checks per backend) passes on both, with output **identical** between backends and identical to the default build.

**Live real-window run is blocked by a pre-existing defect, not a font issue.** On this KDE Wayland desktop with fractional scaling (1.2/1.25), the `sdl_renderer` build aborts at startup on **both backends**: `src/renwindow.rs:261` asserts that the pixel/point ratio is an integer. The assertion is inherited verbatim from the upstream C (`query_surface_scale`), and the default surface build never calls it. Font scaling under `sdl_renderer` at a surface scale above 1 is therefore unverified live. It needs an explicit migration disposition: either fix the inherited fractional-scale assertion separately, or record `sdl_renderer` live coverage as blocked by it.

### Human decision — `sdl_renderer` crash is a separate defect

The user chose to record the fractional-scale abort as an **inherited defect with its own follow-up**, outside the font comparison. It crashes identically on FreeType. For the font decision, the headless `sdl_renderer` evidence (contract suites, identical geometry) is accepted, and the crash does not block backend selection. The migration plan must carry the fix (support or round non-integer pixel/point ratios in `query_surface_scale`).

## Resolution

The candidate composition — **Skrifa 0.48.0 + Zeno 0.3.3 + tiny-skia 0.12.0** — works behind the existing renderer seam on throwaway branch `prototype/font-editor-candidate` (local only; commits `07b93ca9` through `9723a24b`, base `5bd90b47`). Atlas, glyph cache, fallback, tabs, blending, decorations and the Lua API are shared and unchanged. Evidence and the user's decisions:

| Gate | Result | Decision |
|---|---|---|
| Live appearance (HiDPI panel) | Side-by-side review of the user's config (grayscale/full), bundled defaults (subpixel/slight), mono and bold/italic | **Accepted** by the user: "Candidate looks good" |
| Synthetic bold | FreeType's fixed 0.5 px bold was invisible; candidate uses `pixel_size / 20` | **Approved difference** (size-scaled bold) |
| Text geometry | Heights and every measured width identical to FreeType for bundled and user fonts, 3 sizes × 9 modes; 13,068 invariant checks pass per backend, also under `sdl_renderer` | No geometry difference to approve |
| API/contracts | FreeType path: 16/16 suites. Candidate: all suites except pixel hashes; 23 inventory assertions; load-error wording preserved | Passes |
| Performance | Paired, alternating real-window runs on the 60 Hz display: candidate p95 equal or lower than FreeType in every ordinary workload, both within 16.7 ms | **Accepted**: no degradation |
| Memory | No leak; steady RSS equal or lower. Fixed a whole-file heap copy found during testing. CJK peak up to +8 / +32 MiB (one / eight live) | Steady accepted; **peak deferred** to migration follow-up |
| Malformed fonts | 114 corrupt/truncated/mutated fonts: no crash, signal, timeout or Lua error on either backend | Passes |
| `sdl_renderer` | Headless suites and geometry pass; live run blocked by inherited fractional-scale abort (both backends) | **Separate defect**, not blocking |

Carried forward to [Select the Rust-native font backend](16-select-rust-font-backend.md) and the migration plan: the deferred CJK peak, the `sdl_renderer` scale defect, unexercised format paths (BDF/Type 1/bitmap strikes; see [the format research](22-investigate-rust-native-legacy-and-bitmap-formats.md#resolution)), and the normal-density review, which is still pending with no display (the user may revise that gate at selection). Cold-start/first-glyph cost and a large-collection stress case were not separately timed. This ticket approves evidence and the listed differences only; it selects no backend and authorizes no production change.
