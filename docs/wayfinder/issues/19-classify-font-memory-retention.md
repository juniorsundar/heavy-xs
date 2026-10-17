# Classify font memory retention in the current renderer

Parent: ../freetype-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: open
Assignee: none
Blocked by: 14

## Question

The current renderer's resident memory grows during font load/copy cycles and is not returned after collection. Is that growth unbounded or does it plateau? Is it caused by FreeType-specific code a replacement would remove, or by backend-independent code a migration would keep?

The [FreeType baseline](../assets/freetype-baseline.md) recorded this as an **unclassified observation**. Each cycle loads a font, calls `set_size`, copies it, then draws 64 glyphs with both fonts. Across eight cycles, resident memory rose by about 2 MiB per cycle for both JetBrains Mono and the Noto Sans CJK collection. It did not fall after weak references proved that Lua had collected every font. The Lua heap stayed near 155 KiB. The raw rows are in `font-retention-{mono,custom_cjk}.tsv` in [the baseline artifacts](../assets/freetype-baseline/).

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
- Reuse [the baseline probe](../assets/freetype-baseline.lua) and its reproduction commands. Use an isolated target, stage and user directory, a test-hooks release build, the offscreen driver, and public fonts only. Record the source commit, `Cargo.lock` SHA-256 and the binary SHA-256.
- Do not use heap-profiler tooling unless the cheaper steps cannot distinguish the classes; record which tools were used.
- Do not set thresholds or acceptance criteria. Those belong to [Define font compatibility and acceptance criteria](15-define-font-acceptance.md).

## Deliverable

Add an evidence asset under `docs/wayfinder/assets/` with probe changes, raw TSVs, reproduction commands and limitations. Resolve this ticket with a comment and a gist line under the map's "Decisions so far".

## Comments

Created during the baseline resolution so it can run in parallel with the acceptance discussion. It deliberately does not block acceptance. Its result should inform the memory criterion when available.
