# Font memory retention classification

Evidence for [Classify font memory retention in the current renderer](../issues/19-classify-font-memory-retention.md). It explains the unclassified RSS growth in the [FreeType baseline](freetype-baseline.md). It sets **no thresholds or acceptance criteria** and changes no production code.

## Result

**Classification: none of the ticket's renderer classes.** The growth that was recorded comes from **memory mapped through the NVIDIA EGL driver (`/dev/nvidiactl`) while SDL's offscreen video driver uses its accelerated window framebuffer on this host**, not from fonts. Whether SDL or the driver holds that memory was not investigated. It is neither a font leak, in FreeType-specific or backend-independent code, nor glibc allocator retention.

- The growth is **file-backed RSS in the `/dev/nvidiactl` mapping**. Anonymous memory (the heap, where font, glyph and atlas allocations live) does not grow with it.
- It **grows per presented frame, not per font**. A control that loads no font and only presents frames grows the same way, and without bound: +188 MiB from baseline over 400 frames at 960×140.
- It **scales with window area**: about 0.10, 0.62 and 8.8 MiB per frame at 320×60, 960×140 and 1920×1080.
- With **`SDL_FRAMEBUFFER_ACCELERATION=0`**, or with the `dummy` driver, it disappears. Under that setting:
  - The ticket's full load → `set_size` → `copy` → draw cycle **plateaus after the first cycle**: mono stays flat from cycle 1 to cycle 1000, and CJK settles at +0.5–0.8 MiB over baseline.
  - `malloc_trim(0)` returns most of the remaining small rise, which makes it ordinary glibc free-list retention.
- The baseline probe's own 8-cycle run, unchanged except for the hint, grows +0.12 MiB (mono) and +0.7 MiB (CJK) from `drop` to `cycle8`. The baseline recorded about 15 MiB (mono) and 16 MiB (CJK).

**Why the 100-cycle offscreen runs plateaued at about +17 MiB:** the baseline cycle and the 100-cycle `full` variant alternate a background shade that repeats every second frame. The render cache therefore skips unchanged frames, and the offscreen growth stalls after a few distinct frames. The `frames` control forces a full repaint every cycle and never stops growing. That is why the baseline seemed to show steps of about 2 MiB "per font cycle".

**Font code (both classes, accelerated path excluded):**
- No unbounded growth was found. Every font, glyph map and atlas allocation goes away on `__gc`.
- Malloc in-use bytes rise by about 14 B per full cycle over 1000 cycles. Of that, 1.4 B per cycle is Lua heap growth, and the probe's own `/proc` and `io.popen` reads add some per sample. **The residue is below what this method can attribute** and does not scale with glyph count (see Attribution).

## Identity

| Item | Value |
| --- | --- |
| Source | `5bd90b47e8b078e959ce5c03f59c8811eccbecd0`. `src/`, `data/`, `Cargo.toml`, `Cargo.lock` and `build.rs` are identical to the crate-move commit `98ca8cd8`, and the renderer sources are those of the baseline. |
| `Cargo.lock` SHA-256 | `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272` (same as the baseline) |
| Probe binary | `cargo build --release --features test_hooks`, isolated target. SHA-256 `e487b4c9fa019e9f3cf1eba2ef4edecaa13f677dd8c5a50890aea1061b85c974` |
| Probe | [`font-retention-classify.lua`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention-classify.lua), SHA-256 `d8bd0d167c0876ab7b61134321e2cb38bb3cb2c41540fa835f2aade607d2b0c8` |
| Stats shim | [`mallinfo-shim.c`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention/mallinfo-shim.c), SHA-256 `3efb30246cc1ebeca2ab97d91a289d8dcad7b02b1f21293fad299e385a22958c` (gcc 15.3.0, glibc 2.42). It is `LD_PRELOAD`ed by the probe only and is not part of Lite XL. |
| Host | Linux x86_64, NVIDIA RTX A2000 Laptop GPU, driver 595.104.02, glvnd EGL; system SDL3 3.4.14 |
| Fonts | bundled JetBrains Mono Regular (`mono`); Noto Sans CJK 2.004 variable collection (`custom_cjk`, face 0), public system path |

## Method

Each run is a separate process. The probe reuses the baseline's helpers:
- `renderer.font.load` with grayscale antialiasing, size 16
- the alternating-shade `frame`
- `glyphs(64)` from U+0020 (mono) or U+4E00 (CJK)
- double `collectgarbage` before each sample
- weak-reference checks that every cycle's fonts are collected

Each sample records:
- `VmRSS`/`VmHWM`
- the Lua heap
- glibc `mallinfo2` (arena, mmap, in-use and free bytes)
- `/proc/self/smaps_rollup` anonymous and file-backed RSS
- the open file-descriptor count

The run ends with a sample after `malloc_trim(0)`. Optional full `smaps` snapshots compare each mapping before and after.

The variants change one condition at a time. Default is 100 cycles, sampled every 10:

| Variant | Each cycle |
| --- | --- |
| `full` | Load → `set_size(17..24)` → `copy` → draw 64 glyphs with both, then an empty frame. This is the baseline cycle; sizes repeat every 8 cycles. |
| `load` | Load only, then an empty frame |
| `width` | Load → `get_width(64 glyphs)`; no glyphs rasterized |
| `draw` | Load → draw 64 glyphs (`RETAIN_GLYPHS` changes the count) |
| `drawsame` | Load → draw the same 64 glyphs twice |
| `sameload` | One font kept alive, drawn every cycle; no new fonts |
| `hold8` | Eight fonts loaded at sizes 13–20 and drawn, then released together; the baseline's `live8` phase, repeated |
| `frames` | **No font**: one full-repaint frame per cycle (control) |

The tags in the file names give the driver and allocator conditions:

| Tag | Condition |
| --- | --- |
| `offscreen` | `SDL_VIDEODRIVER=offscreen`, as in the baseline |
| `noaccel` | offscreen with `SDL_FRAMEBUFFER_ACCELERATION=0` |
| `-trim` | adds `MALLOC_ARENA_MAX=1 MALLOC_TRIM_THRESHOLD_=0` |
| `dummy` | `SDL_VIDEODRIVER=dummy` |
| `sdl_render_driver_software` | offscreen with `SDL_RENDER_DRIVER=software` |
| `-long`, `-1000` | 400 or 1000 cycles |
| `-rN` | repeat number |
| `winWxH` | the window size |

## Findings

### 1. The growth is file-backed driver memory, not heap

Baseline probe, the unchanged 8-cycle retention mode ([`baseline-probe-*`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention)), RSS in KiB:

| Driver | Font | `drop` | `cycle8` | Growth |
| --- | --- | --- | --- | --- |
| offscreen (baseline) | mono | 62396 | 77716 | **+15320** |
| offscreen | custom_cjk | 62688 | 79220 | **+16532** |
| offscreen, no acceleration | mono | 14356 | 14480 | +124 |
| offscreen, no acceleration | custom_cjk | 14544 | 15268 | +724 |

The accelerated offscreen run reproduces the original baseline to within about 0.2 MiB. With acceleration off, the same probe starts about 46 MiB lower: the NVIDIA EGL libraries are never loaded.

[`smaps-delta-offscreen-accel.tsv`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention/smaps-delta-offscreen-accel.tsv) compares the mappings before and after 20 `full` cycles:

| Mapping | RSS change |
| --- | --- |
| `nvidiactl` | **+17280 KiB** (three more mappings, counted in the uncommitted full `smaps` dumps) |
| `[heap]` | +364 KiB |
| binary | +192 KiB |

Without acceleration ([`smaps-delta-offscreen-noaccel.tsv`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention/smaps-delta-offscreen-noaccel.tsv)), only `[heap]` (+436 KiB) and the binary's own pages (+128 KiB) change. In every accelerated run, the anonymous-RSS column stays within +0.25 MiB (mono) and +1.2 MiB (CJK) while the file-backed column grows by about 17.0 MiB (+17384 to +17448 KiB).

### 2. It grows per frame, by window area, without fonts

`frames` control, no font loaded, change in file-backed RSS:

| Run | Change |
| --- | --- |
| offscreen 960×140, 400 frames ([`retain-mono-frames-offscreen-long`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention/retain-mono-frames-offscreen-long.tsv)) | **+188 MiB** from baseline, linear, no plateau (240512 − 47104 KiB) |
| 320×60, frames 1→100 / 100→200 | +13.0 / +7.9 MiB |
| 960×140 | +62.9 / +60.0 MiB |
| 1920×1080 | +868 / +884 MiB |
| no acceleration, 400 frames | **0** |
| `SDL_RENDER_DRIVER=software`, 400 frames | +225 MiB from baseline (the hint does not affect the window framebuffer) |

Per frame from frames 1 to 200: 0.10, 0.62 and 8.8 MiB at the three sizes.

### 3. Font cycles plateau once the driver is excluded

`full` cycle, no acceleration ([`retain-*-full-noaccel*`](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-retention)), RSS in KiB:

| Font | baseline | cycle 1 | cycle 10 | cycle 100 | cycle 1000 | after `malloc_trim` |
| --- | --- | --- | --- | --- | --- | --- |
| mono | 14288 | 14560 | — | 14548 | 14544 | 14376 |
| custom_cjk | 14160 | 14808 | — | 14848 | 14848 | 14376 |
| mono r1/r2/r3 (100 cycles) | 14184 / 14252 / 14076 | — | +256 / +256 / +252 | flat after cycle 10 | — | −164 / −172 / −172 from cycle 100 |
| custom_cjk r1/r2/r3 | 14292 / 13956 / 14096 | — | +688 / +816 / +816 | +1836 / +816 / +816 (r1 stepped at cycle 40) | — | 14516 / 14292 / 14432 |

**One-time cost:** the first cycle of each font type costs about 0.25 MiB for mono and 0.6–0.8 MiB for CJK, held in glibc free lists. FreeType's per-face memory and the larger CJK charmap and glyph tables are freed into those lists and reused by later cycles.

**One later step:** a single CJK run (r1) took an extra +1.1 MiB step between cycles 30 and 50, then stayed flat to cycle 100. `malloc_trim` returned it, and r2/r3 did not repeat it. That is allocator retention.

**Under `MALLOC_ARENA_MAX=1 MALLOC_TRIM_THRESHOLD_=0`** (`-trim`), the shapes are the same, and glibc free bytes at the end drop from 336 to 309 KiB (mono) and from 1940 to 537 KiB (CJK). The 1000-cycle **offscreen** runs (`offscreen-1000`) stay flat between cycles 100 and 800 while frames repeat. They then step up about 3.5–4 MiB at cycle 900, which is again file-backed driver memory, as in finding 2.

### 4. Attribution by variant

No acceleration, 100 cycles. The **malloc in-use bytes** column is the net live-heap change from cycle 1 to cycle 100; the Lua-heap change is shown separately.

| Variant | mono in-use | CJK in-use | Lua heap |
| --- | --- | --- | --- |
| `frames`, no font | +2672 B | +2416 B | about +1.3–1.4 KiB |
| `load` | +2704 B | +2864 B | about +1.3 KiB |
| `draw` (64 glyphs) | +9584 B | +9680 B | about +1.3–1.4 KiB |
| `full` | +10528 B | +12080 B | about +1.3–1.4 KiB |

- **`frames` and `load` match:** loading and freeing a face, with its stream, charmap rows and metrics, leaves nothing measurable. The about 2.5 KiB over 100 samples there is the Lua heap plus the probe's own reads of `/proc` and `io.popen`.
- **Drawing adds about 70 B per cycle**, roughly a frame's worth of cached state. The glyph-count control rules out a per-glyph leak: over 200 cycles, drawing 16, 64 or 256 distinct glyphs adds +5440, +3344 and +1728 B in-use. That does not increase with glyph count. RSS moves by at most 16 KiB.
- **Over 1000 `full` cycles:** in-use rises +15.3 KiB (mono) and +14.6 KiB (CJK) between cycles 1 and 1000. After subtracting the Lua heap (+1.4 KiB), the residue is about **14 B per cycle**. It falls within the probe's own sampling and rencache noise, and no RSS growth follows from it.
- **`hold8` (eight fonts live, released together; 50 rounds):**
  - The first round costs +1.0 MiB (mono) or +2.4 MiB (CJK), and later rounds reuse it.
  - CJK keeps climbing for about 10 rounds, to about +6.3 MiB, then plateaus.
  - `malloc_trim` returns nearly all of it (mono 15080 → 14232 KiB, CJK 20400 → 14620 KiB). This explains the baseline's large `live8` step for CJK as allocator retention after eight CJK faces were alive at once.

### 5. Source reading of the release path

`f_font_gc` (`src/renderer_api.rs`) calls `ren_font_free` (`src/renderer.rs`):

| Resource | Allocated in | Freed by | Matched |
| --- | --- | --- | --- |
| Atlas SDL surfaces | `font_allocate_glyph_surface` (`SDL_CreateSurface`) | `font_clear_glyph_cache`: `SDL_DestroySurface` for every `AtlasSurface` of both formats | yes |
| `GlyphAtlas`/`AtlasSurface` vectors | `push` in the same function | `Vec::clear` and the drop of `RenFont` | yes |
| Glyph metric rows (`Box<[GlyphMetric; N]>`) | `font_load_glyph_metric` | set to `None` in `font_clear_glyph_cache`; any remainder drops with `RenFont` | yes |
| Charmap rows (`Box<[u32; N]>`) | `font_get_glyph_id` | drop of `RenFont.charmap` | yes |
| FreeType face | `FT_Open_Face` in `ren_font_load`; wrapped by `Face::from_raw` (takes an `FT_Reference_Library`) | `Face::drop`: `FT_Done_Face` + `FT_Done_Library` (balances the reference) | yes |
| Stream + `SDL_IOStream` | `font_stream_new` (`Box::into_raw`) | `font_file_close`, called by `FT_Done_Face`, or by `FT_Open_Face` on failure | yes (in every per-cycle release variant the fd count stays at 5 or 28; `sameload` holds one font, so +1) |
| Path `CString` | `ren_font_load` | drop of `RenFont` | yes |
| Render-cache roots | `draw_text` roots the font in a per-frame registry table | replaced by a new table in `f_end_frame`; the weak checks confirm collection after `end_frame` | yes |
| Font failure paths | `ren_font_load` error after `font_set_face_metrics` | the `RenFont` drop frees the face and stream | yes (not exercised here) |

`ren_font_group_set_size` also clears the glyph cache before resizing, so `set_size` does not pile up atlases. The rencache command buffer and its two cell grids are process-lifetime and do not grow per font.

## Consequences for the open tickets (facts only)

- **[Define font compatibility and acceptance criteria](../issues/15-define-font-acceptance.md):** a memory rule should be measured with `SDL_FRAMEBUFFER_ACCELERATION=0` (or the `dummy` driver) and in the `/proc` anonymous and malloc in-use columns, not raw RSS under the accelerated offscreen driver. On that basis FreeType plateaus after the first cycle per font type, apart from glibc free-list retention that `malloc_trim` returns. Choosing the rule stays with that ticket.
- **[Compare candidate font controls and rendering](../issues/18-compare-font-controls-and-rendering.md):** candidate retention can be read with this probe and the same conditions. The release path a replacement would keep is shared code: atlas surfaces, glyph and charmap rows, and frame roots. It showed no leak. A candidate changes only the face, stream and rasterizer parts.
- **Not a font finding:** a real NVIDIA window may show the same per-frame growth in driver memory if SDL's accelerated window framebuffer behaves as it does offscreen. This was **not tested** with a real window. It is a separate, non-font observation.

## Limitations

- Single host, single NVIDIA driver. The growth under the accelerated framebuffer may be specific to SDL 3.4.14's offscreen driver with this EGL stack. Mesa/Intel/AMD, Wayland and X11 windows were not measured. The cause inside SDL or the driver was not investigated, because it is outside this font effort.
- No heap profiler was used: no valgrind, heaptrack or perf. `mallinfo2`, `smaps` and controlled variants were enough to separate the classes. The roughly 14 B-per-cycle residue is therefore **undetermined**, not proved zero.
- The probe evolved while the matrix ran: variants and the window-size knob were added. Each variant's code was unchanged after its first run, and the committed probe is the final superset. Early runs used the default 960×140 window.
- Only grayscale antialiasing was cycled. Subpixel antialiasing (three bitmaps per glyph), other hinting modes, `sdl_renderer` builds, HiDPI and malformed fonts were not cycled.
- RSS counts shared libraries as they fault in. The anonymous and malloc columns are the font-relevant signals.

## Reproduce

```sh
T=/tmp/lxl-font-retention   # any isolated directory
cargo build --release --features test_hooks --target-dir "$T/target"
scripts/install-linux.sh "$T/target/release/lite-xl" "$T/target/stage"
cp docs/wayfinder/assets/font-retention-classify.lua "$T/target/stage/usr/local/share/lite-xl/"
gcc -O2 -shared -fPIC -o "$T/mallinfo-shim.so" docs/wayfinder/assets/font-retention/mallinfo-shim.c -ldl
export SDL_VIDEODRIVER=offscreen DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent SDL_FILE_DIALOG_DRIVER=
export LITE_PREFIX="$T/target/stage/usr/local" LITE_USERDIR="$T/userdir" LITE_OS_HOME="$T/home"
export FONT_BASELINE_CJK=...   # public Noto Sans CJK collection path
mkdir -p "$T/out" "$T/userdir" "$T/home"
run() { env "$@" LD_PRELOAD="$T/mallinfo-shim.so" LITE_XL_RUNTIME=font-retention-classify RETAIN_OUT="$T/out" \
          timeout -k 2 300 "$T/target/release/lite-xl"; }
for c in mono custom_cjk; do for v in full load width draw sameload frames; do
  run RETAIN_CASE=$c RETAIN_VARIANT=$v RETAIN_TAG=offscreen
  run RETAIN_CASE=$c RETAIN_VARIANT=$v RETAIN_TAG=noaccel SDL_FRAMEBUFFER_ACCELERATION=0
  run RETAIN_CASE=$c RETAIN_VARIANT=$v RETAIN_TAG=noaccel-trim SDL_FRAMEBUFFER_ACCELERATION=0 MALLOC_ARENA_MAX=1 MALLOC_TRIM_THRESHOLD_=0
done; done
run RETAIN_CASE=mono RETAIN_VARIANT=frames RETAIN_TAG=offscreen-long RETAIN_CYCLES=400 RETAIN_EVERY=50
# The unchanged baseline probe with the driver condition alone flipped:
SDL_FRAMEBUFFER_ACCELERATION=0 LITE_XL_RUNTIME=freetype-baseline FONT_BASELINE_MODE=retention FONT_BASELINE_CASE=mono FONT_BASELINE_OUT="$T/out" "$T/target/release/lite-xl"
```

Optional knobs:
- `RETAIN_CYCLES`, `RETAIN_EVERY`
- `RETAIN_GLYPHS` for `draw`
- `RETAIN_W`/`RETAIN_H` for the window size
- `RETAIN_SMAPS=1` to dump full `smaps` before and after (they contain local paths and are not committed)

Each run prints one `RETAIN …` line and exits 0. A failed weak-reference check prints `RETAIN FAILED` and exits 1.
