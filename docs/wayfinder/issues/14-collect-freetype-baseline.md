# Collect the FreeType rendering and cost baseline

Parent: ../freetype-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-freetype-baseline
Blocked by: 12

## Question

What reproducible FreeType reference artifacts and bounded measurements are needed to judge a replacement's visual output, text geometry, responsiveness and memory?

Use the completed font inventory to assemble a small representative corpus from existing fixtures: bundled fonts plus representative redistributable custom fonts, UI/code/icon text, fallbacks/missing glyphs, UTF-8, tabs, options, sizes, fractional positions and relevant scaling paths. Reuse the existing renderer tests before adding a new harness. Keep font licensing/provenance explicit; do not upload private fonts or user configuration.

Record the current source/build/FreeType identities, deterministic metric observations, inspectable rendering artifacts and an explanation of which real editor paths each covers. Measure fresh load/rasterization separately from warmed glyph/cache/drawing paths, cache growth and repeated-use cleanup. Document containment and any censored or unavailable cases. Headless pixel/metric evidence is not real-window readability or input-to-display latency.

Create bounded evidence assets/probes only; no production replacement, golden-baseline rewriting, invented pass thresholds or candidate verdict. Link assets and reproduction commands on resolution, including unmeasured paths. The acceptance ticket decides which comparisons and numeric gates are required.

Prerequisite: [Inventory the observable font contract](12-inventory-font-contract.md). The baseline unblocks [Define font compatibility and acceptance criteria](15-define-font-acceptance.md).

## Comments

### Baseline collection

Claimed by `pi-freetype-baseline` for bounded reference capture using the validated inventory and existing renderer fixtures. Builds/staging and runtime user directories will be isolated; production code, dependency configuration and golden baselines remain untouched. Rendering evidence, fresh/warm cost and retention observations will be separated from human appearance approval and future acceptance thresholds.

### Resolution

The FreeType reference baseline is recorded in [the baseline report](../assets/freetype-baseline.md), produced by [the bounded probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/freetype-baseline.lua) and saved in [the artifact directory](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/freetype-baseline). Its source/build identity is pinned to `d9b3db31` with bundled FreeType 2.13.2.

- **Corpus:** three bundled fonts plus four public custom fonts: Liberation Sans/Mono, TeX Gyre Heros CFF, and the Noto Sans CJK five-face variable collection. Provenance and licence evidence cover every file; no font files or local paths are published. The `icons.ttf` licence is still unverified.
- **Rendering:** eight byte-reproducible contact sheets. Seven are 3×3 antialiasing × hinting matrices, and one covers features. They exercise real ASCII icons, icon→Fira and Mono→CJK fallback, `.notdef`, tabs, fractional phases, styles, sizes and malformed text. The sheets come with 63 metric rows and 89 geometry rows, each asserting that the returned pen position equals the measured end.
- **Cost:** 315 wall-clock samples separate fresh load, first width, first draw, warm widths and forced full-replay warm draws. All values are machine-relative and are not thresholds.
- **Retention:** 19 lifecycle phases each for mono and CJK. Weak references prove collection. **Unclassified observation:** RSS rises by about 2 MiB per load/copy cycle and is not returned after collection.
- **Validation:** renderer and rencache contract suites and 1,239,848 hit-test points passed on the probe build. Production code, dependencies and golden baselines are unchanged.
- **Unmeasured:** HiDPI, `sdl_renderer`, real-window readability and latency, face index >0 and variation selection, colour/bitmap fonts, malformed fonts, the U+25A1 retry and cross-host variance.

[Define font compatibility and acceptance criteria](15-define-font-acceptance.md) decides which comparisons and gates apply. This ticket does not decide them.
