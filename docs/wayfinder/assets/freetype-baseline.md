# FreeType rendering and cost baseline

Evidence for [Collect the FreeType rendering and cost baseline](../issues/14-collect-freetype-baseline.md). It records how the current FreeType renderer behaves and leaves acceptance decisions to the later criteria ticket. It contains **no pass thresholds, candidate verdicts or approved differences**.

## Identity

| Item | Value |
| --- | --- |
| Source | `d9b3db31ab79339a60fe203ece364d17eb41786a` (Lite XL 2.1.7). The later crate move, `98ca8cd8`, leaves `renderer.rs`, `renderer_api.rs`, `ft.rs` and `test_hooks.rs` byte-identical. |
| Font stack | freetype-rs 0.38.0 / freetype-sys 0.23.0 (bundled FreeType 2.13.2); system SDL3 3.4.14 |
| `Cargo.lock` SHA-256 | `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272` |
| Probe binary | Release build with `--features test_hooks`. SHA-256: `8e8444bbde7dbf100f88046e8ae9fb310e1f19968b5531f729e9f511004f42e3` |
| Probe | [`freetype-baseline.lua`](freetype-baseline.lua), SHA-256 `4561a885b4b1e432ce4831afcd859da27c6d5a126e61c49a7b65654a5107d2ca` |
| Driver | `SDL_VIDEODRIVER=offscreen`, surface scale 1, Linux x86_64, single host |

Pixel output depends on the build. A FreeType 2.14.1 build has already changed six hinted mono hashes. Compare against these artifacts only with the same build identity, or treat the difference as a reviewed observation.

## Corpus and provenance

[`font-provenance.tsv`](freetype-baseline/font-provenance.tsv) records the file name, package, face-0 family/style, byte size, SHA-256, outline table, collection face count, default variation axes, licence evidence and upstream URLs. It contains no local paths.

| ID | Font | Format | Licence |
| --- | --- | --- | --- |
| `mono` | JetBrains Mono Regular (bundled) | glyf | OFL-1.1 ([licenses.md](../../../licenses/licenses.md)) |
| `fira` | Fira Sans Regular (bundled) | glyf | OFL-1.1 ([licenses.md](../../../licenses/licenses.md)) |
| `icons` | `icons.ttf` (bundled, fontello) | glyf | **Font-specific licence unverified.** The embedded notice credits only the "original authors @ fontello.com". |
| `custom_sans` / `custom_mono` | Liberation Sans/Mono 2.1.5 | glyf | OFL-1.1 |
| `custom_cff` | TeX Gyre Heros 2.501 | OTTO/CFF | GUST Font License |
| `custom_cjk` | Noto Sans CJK 2.004 variable collection | TTC, 5 faces, CFF2, `wght` default 100 | OFL-1.1 |

Custom fonts are supplied as public system paths through `FONT_BASELINE_SANS/MONO/CFF/CJK`; no font files were copied into the repository. An unset or missing path produces a `CENSORED` line and no data row. FreeType opens **face 0 only**, with no variation selector, so the CJK collection renders its default `wght=100` (Thin) instance. That is recorded behaviour, not a preference.

## Artifacts

| Artifact | Covers |
| --- | --- |
| `sheet-<id>.png` (7) | Each font across all antialiasing × hinting combinations (3×3) at 16 px, with a red marker at the pen position returned by `draw_text`. |
| [`sheet-features.png`](freetype-baseline/sheet-features.png) | Real ASCII-mapped icons, icon→Fira fallback, Mono→CJK fallback, an absent codepoint using the last font's `.notdef`, tabs with and without an offset, six fractional pen positions, eight style combinations, three sizes, malformed UTF-8, embedded NUL, a combining mark and U+3000. |
| [`image-sheets.tsv`](freetype-baseline/image-sheets.tsv) | Logical-RGB surface hash and lit-pixel count for every sheet. These are the same values as `lxl_test.surface_hash`. |
| [`font-metrics.tsv`](freetype-baseline/font-metrics.tsv) | 63 rows: size, height and widths for the sample, `A`, space, tab, tab offsets 0/3.5 and U+0301, by font × antialiasing × hinting. |
| [`feature-metrics.tsv`](freetype-baseline/feature-metrics.tsv) | 89 rows: x/y, measured width, returned x and height for every drawn line. The probe asserts `returned_x == x + width`. |
| [`font-timing.tsv`](freetype-baseline/font-timing.tsv) | 315 samples: 7 fonts × 9 options × 5 repetitions. |
| `font-retention-{mono,custom_cjk}.tsv` | RSS, HWM and Lua heap at 19 lifecycle phases. |

The PNG files are lossless conversions of probe-written P6 PPM files. Each PPM was read through `lxl_test.pixel`, so alpha is excluded. Two fresh capture processes produced byte-identical PPM and TSV outputs. Each sheet frame must repaint the whole surface, because same-size windows share the render-cache grid. An earlier run without this check left stale pixels in the CJK sheet; the final CJK and icon sheets match standalone single-font runs byte for byte.

### Editor paths covered

- **Code text:** `mono`, `custom_mono` and the fractional-phase row exercise `get_width`, `draw_text` and rencache replay, as used by `DocView` text, caret and hit-test geometry.
- **UI text:** `fira`, `custom_sans`, `custom_cff`, style rows and the size rows exercise status bar, tab, tooltip and label text.
- **Icons:** the `icons` sheet and the "ASCII-mapped icons" row exercise the real icon font through its actual ASCII mappings. Earlier fixtures used U+E000/U+F000, which exercise missing glyphs instead.
- **Fallback:** an icon→Fira group, a Mono→CJK group and a missing-glyph group cover user-configured font groups.
- **Cache:** rencache full replay, identical-frame skipping and deferred font rooting are asserted by `replay_checks` in capture and bench modes.

## Cost observations

Times use `system.get_time()` wall clock on one unloaded desktop host. They are machine-relative and are **not thresholds**. Each value below is the median of 45 samples per font (9 options × 5 repetitions), in milliseconds. Every timed frame was asserted to replay the entire 400×100 surface.

| Font | Fresh load | First width | First draw | 100 warm widths | 10 warm draws | Max warm draw |
| --- | --- | --- | --- | --- | --- | --- |
| mono | 0.029 | 0.071 | 0.393 | 0.029 | 0.715 | 0.093 |
| fira | 0.034 | 0.084 | 0.530 | 0.027 | 0.706 | 0.092 |
| icons | 0.028 | 0.040 | 0.222 | 0.017 | 0.684 | 0.091 |
| custom_sans | 0.032 | 0.059 | 0.348 | 0.019 | 0.659 | 0.089 |
| custom_mono | 0.031 | 0.070 | 0.396 | 0.028 | 0.696 | 0.092 |
| custom_cff | 0.056 | 0.075 | 0.385 | 0.019 | 0.640 | 0.098 |
| custom_cjk | 0.207 | 0.113 | 1.436 | 0.012 | 0.592 | 0.078 |

The phases are separated as follows:

- **Fresh load:** `renderer.font.load`
- **First width:** cold glyph metrics
- **First draw:** glyph rasterization and atlas allocation, plus the same surface fill and replay included in warm draws
- **Warm work:** cached widths and draws, with each frame forced into full replay

Loads are lazy: glyphs are not rasterized until they are drawn. A 32 MB collection therefore opens quickly, while its first draw costs more.

## Retention observations

Each retention run measures one font in its own process. The run collects garbage twice before each sample, then records `/proc/self/status` RSS/HWM and `collectgarbage("count")`. Phase sequence:

1. Baseline
2. Load the font.
3. Measure and render 16, 64 and 256 distinct glyphs.
4. Drop the font.
5. Run eight load → `set_size` → `copy` → draw cycles.
6. Hold eight fonts at once, then release them.

Weak references assert that every font becomes collectable after `end_frame` (44 checks per run).

Selected phases (all 19 are in the TSVs):

| Phase (KiB RSS) | mono | custom_cjk |
| --- | --- | --- |
| baseline | 60076 | 60184 |
| after 256-glyph raster | 62268 | 63168 |
| font dropped | 62268 | 62652 |
| after cycle 1 | 66444 | 66776 |
| after cycle 8 | 77588 | 78720 |
| 8 live fonts released | 78304 | 83548 |

**Unclassified observation:** RSS grows in steps across the eight load/copy cycles, about 1.9–2.0 MiB per cycle on average from `drop` to `cycle8`, and does not fall after Lua confirms that the fonts were collected. The Lua heap stays near 155 KiB. This does not establish whether the cause is glyph-atlas surfaces not freed on font `__gc`, FreeType/SDL allocator retention or the system allocator. A candidate backend should be measured with the same phases before this is treated as either a leak or acceptable behaviour.

## Validation

- Fresh isolated release build: `OK renderer` and `OK rencache` contract suites; `OK hit-test (1239848 grid points)`
- Capture: 114 checks. Bench: 3,469 checks. Retention: 44 checks each for mono and CJK.
- Second capture is byte-identical. The CJK and icon sheets also match standalone runs. Visual inspection confirms pictogram icons, real CJK fallback, a `.notdef` box, distinct styles, and pen-end markers at returned positions; the `all styles` row is dense by construction. An invalid mode exits non-zero with `invalid FONT_BASELINE_MODE`.
- Font table checks: embedded OFL notices for the two bundled OFL fonts, the expected CFF/CFF2 tables, the five-face collection and the CJK `wght` default.
- No changes to `src/`, `data/`, `tests/contract`, the golden baselines, `Cargo.toml` or `Cargo.lock`.

## Not measured

- HiDPI / surface scale >1: the offscreen driver reports scale 1, and `query_surface_scale` requires an integer ratio.
- `sdl_renderer` builds, real-window readability, colour/gamma on a physical display, and input-to-display latency.
- Face index >0, variation-axis selection, colour/emoji (COLR/CBDT/sbix/SVG), bitmap-only and Type 1/PCF fonts.
- Malformed-font robustness and resource exhaustion.
- The U+25A1 box retry after a metric-load failure: the bundled fixtures cannot trigger it.
- Shaping, kerning and ligatures; the current renderer does not apply them.
- Repeated-host or cross-machine timing variance, CPU frequency control, and per-allocation attribution of RSS.

## Reproduce

```sh
# Release build with test hooks, staged in an isolated directory.
cargo build --release --features test_hooks --target-dir "$T"
scripts/install-linux.sh "$T/release/lite-xl" "$T/stage"
cp docs/wayfinder/assets/freetype-baseline.lua "$T/stage/usr/local/share/lite-xl/"
export SDL_VIDEODRIVER=offscreen DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent SDL_FILE_DIALOG_DRIVER=
export LITE_PREFIX="$T/stage/usr/local" LITE_XL_RUNTIME=freetype-baseline LITE_USERDIR="$T/userdir"
export FONT_BASELINE_SANS=... FONT_BASELINE_MONO=... FONT_BASELINE_CFF=... FONT_BASELINE_CJK=...  # public font paths
for m in capture bench; do mkdir -p "$T/$m"; FONT_BASELINE_MODE=$m FONT_BASELINE_OUT="$T/$m" timeout -k 2 120 "$T/release/lite-xl"; done
mkdir -p "$T/retention"
for c in mono custom_cjk; do FONT_BASELINE_MODE=retention FONT_BASELINE_CASE=$c FONT_BASELINE_OUT="$T/retention" timeout -k 2 120 "$T/release/lite-xl"; done
```

Convert PPM files with `magick sheet.ppm -strip PNG24:sheet.png`. Each mode prints one summary line and exits 0. Any failed assertion prints `FREETYPE BASELINE FAILED` and exits 1.
