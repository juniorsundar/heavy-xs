# Classify font memory retention in the current renderer

Parent: ../freetype-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-font-retention
Blocked by: 14

## Question

The current renderer's resident memory grows during font load/copy cycles and is not returned after collection. Is that growth unbounded or does it plateau? Is it caused by FreeType-specific code a replacement would remove, or by backend-independent code a migration would keep?

The [FreeType baseline](../assets/freetype-baseline.md) recorded this as an **unclassified observation**. Each cycle loads a font, calls `set_size`, copies it, then draws 64 glyphs with both fonts. Across eight cycles, resident memory rose by about 2 MiB per cycle for both JetBrains Mono and the Noto Sans CJK collection. It did not fall after weak references proved that Lua had collected every font. The Lua heap stayed near 155 KiB. The raw rows are in `font-retention-{mono,custom_cjk}.tsv` in [the baseline artifacts](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/freetype-baseline).

`src/renderer.rs` combines two kinds of code:

- **FreeType-specific:** face, glyph loading and rasterization.
- **Backend-independent:** glyph and charmap caches, the atlas that packs glyph images onto SDL surfaces, and font release on `__gc`.

The answer decides how [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) can set a memory rule:

- "No worse than FreeType" could accept an equal leak in shared code.
- "Must plateau" might fail every backend, including FreeType.

It also decides whether [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md) can interpret candidate retention, and whether the migration plan must fix the shared code or keep the growth deliberately.

## Scope

1. **Shape:** extend the baseline probe's cycle with about 100 cycles, sampling every 10, for mono and CJK. Use separate processes and the same build identity. Report whether growth is linear or reaches a plateau.
2. **Allocator control:** repeat the run under glibc settings that return free memory, such as `MALLOC_ARENA_MAX=1` and `MALLOC_TRIM_THRESHOLD_=0`. Use an explicit `malloc_trim` only if it can be called without changing production code. This separates allocator retention from live allocations.
3. **Attribution:** source-read the font release path (`ren_font_free` and the `__gc` path in `src/renderer_api.rs`) for:
   - glyph atlas surfaces and their `AtlasSurface` bookkeeping
   - charmap and glyph map rows
   - the FreeType face and stream
   - render-cache roots

   Note any allocation without a matching free. If necessary, use a variant that changes one condition at a time: load-only, load+width, and load+draw.
4. **Result:** report a classification of *leak in FreeType-specific code*, *leak in backend-independent code*, *allocator retention* or *undetermined*, with the evidence. For a leak, estimate the cost per font and per glyph.

## Constraints

- Evidence only. Do not fix the renderer, change production code or dependencies, or rewrite golden baselines. If a leak is confirmed, report the cause and stop. Fixing it is a separate user decision.
- Reuse [the baseline probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/freetype-baseline.lua) and its reproduction commands. Use an isolated target, stage and user directory, a test-hooks release build, the offscreen driver, and public fonts only. Record the source commit, `Cargo.lock` SHA-256 and the binary SHA-256.
- Do not use heap-profiler tooling unless the cheaper steps cannot distinguish the classes; record which tools were used.
- Do not set thresholds or acceptance criteria. Those belong to [Define font compatibility and acceptance criteria](15-define-font-acceptance.md).

## Deliverable

Add an evidence asset under `docs/wayfinder/assets/` with probe changes, raw TSVs, reproduction commands and limitations. Resolve this ticket with a comment and a gist line under the map's "Decisions so far".

## Comments

Created during the baseline resolution so it can run in parallel with the acceptance discussion. It deliberately does not block acceptance. Its result should inform the memory criterion when available.

### Resolution

**Classification: not a font leak and not allocator retention in font code.** The baseline's RSS growth across font cycles is **file-backed memory in the NVIDIA EGL driver mapping (`/dev/nvidiactl`)**. It grows while SDL's offscreen video driver uses its accelerated window framebuffer on this host. Evidence: [Font memory retention classification](../assets/font-retention-classification.md).

- **Driver memory, not heap:**
  - Across the baseline's cycles, anonymous RSS and malloc in-use bytes stay flat.
  - The whole rise is file-backed: about +17 MiB in the `nvidiactl` mapping.
- **Grows per frame, without fonts:**
  - A control that loads no font and only presents frames grows linearly and without bound: +188 MiB over 400 frames at 960×140.
  - Growth scales with window area: about 0.10, 0.62 and 8.8 MiB per frame.
  - The baseline cycle repeats a two-shade frame, so the render cache skips repeated frames. That is why its growth looked like steps of about 2 MiB per font cycle.
- **Gone with `SDL_FRAMEBUFFER_ACCELERATION=0`** (or the `dummy` driver):
  - The unchanged baseline probe grows only +0.12 MiB (mono) and +0.7 MiB (CJK) from `drop` to `cycle8`, against +15.3 and +16.5 MiB with acceleration.
  - The full load → `set_size` → `copy` → draw cycle **plateaus after the first cycle per font type**: mono is flat from cycle 1 to cycle 1000; CJK settles at +0.5–0.8 MiB.
  - `malloc_trim(0)` returns most of the remaining rise, which is ordinary glibc free-list retention. The same goes for the eight-fonts-live step.
- **Font code, measured with acceleration off:**
  - Load and free leave nothing measurable beyond the font-free control.
  - Drawing adds about 70 B per cycle, which does not increase with glyph count.
  - Over 1000 full cycles, about 14 B per cycle remains after subtracting the Lua heap. That is **undetermined**: below the method's resolution, with no RSS effect.
- **Source reading of `__gc` → `ren_font_free`:** every allocation has a matching free:
  - atlas SDL surfaces and their bookkeeping
  - glyph and charmap rows
  - the FreeType face (library reference balanced) and its stream
  - the per-frame render-cache roots

  The fd count stays constant across the per-cycle release variants.

Per-cost estimate: no leak per font or per glyph was found to estimate.

Tools: the probe, `/proc` status/smaps, and an `LD_PRELOAD` `mallinfo2`/`malloc_trim` shim. No heap profiler was needed. No production code, dependency or golden baseline changed.

**For downstream tickets (facts, not criteria):**
- Font memory evidence should be measured with `SDL_FRAMEBUFFER_ACCELERATION=0` or the `dummy` driver, and read from the anonymous and malloc in-use columns, not from raw accelerated-offscreen RSS.
- The release path a replacement would keep (atlas, glyph and charmap rows, frame roots) is shared code and showed no leak. A candidate replaces only the face, stream and rasterizer parts.
- Whether a real NVIDIA window shows the same per-frame driver growth was not tested. It is a non-font observation outside this effort.

Limits: one host and one NVIDIA driver; grayscale antialiasing only in the cycles; `sdl_renderer`, HiDPI, subpixel and malformed fonts were not cycled.
