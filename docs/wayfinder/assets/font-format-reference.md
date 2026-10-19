# Additional font-format reference coverage

Evidence for [Establish additional font-format reference coverage](../issues/20-establish-additional-font-format-coverage.md), using the [confirmed criteria](../issues/15-define-font-acceptance.md#resolution--confirmed-by-the-user). These are **current-renderer observations**, not candidate results, format exclusions or new acceptance thresholds.

## Findings

Opening a font is not evidence that its intended glyph renders:

- **Type 1/PFB:** the sampled Latin glyph loads, measures and renders as an outline. Losing this input format needs explicit disposition; OpenType/CFF support alone does not cover it.
- **BDF:** the sampled native-size glyph renders correctly with **antialiasing `none`**, matching FreeType's mono bitmap bit-for-bit. **The current grayscale/subpixel packed-mono copy path is unsafe**: it treats packed rows as byte-per-pixel rows. That mode was removed from further captures; no corrupted/OOB-derived pixel image is published. No production fix was made.
- **PCF / OTB / embedded bitmap strikes:** these samples open and measure at supported sizes, but mapped glyphs produce generic solid rectangles identical to the missing-glyph controls. They are not successful bitmap rendering.
- **Hybrid Terminus:** size 16 selects a bitmap strike and produces a rectangle; size 17 selects an outline and renders the glyph. Hinting `slight` at size 16 does not cure the rectangle.
- **CBDT/CBLC Noto Color Emoji:** size 16 fails with `failed to load font: invalid pixel size`. Its native 109-ppem strike opens, but the mapped emoji produces the same solid rectangle as a missing glyph.
- **COLRv1 and SVG-containing fonts:** the selected glyphs render their monochrome `glyf` outlines, not colour paints or SVG documents. All final pixel scans have zero chromatic pixels. A successful open of these files does not establish colour-rendering support.

## Build and containment

| Item | Identity |
| --- | --- |
| Source | `5bd90b47e8b078e959ce5c03f59c8811eccbecd0`; source/build inputs match the [memory investigation](font-retention-classification.md#identity) |
| Editor binary | Reused isolated release/test-hooks build; SHA-256 `e487b4c9fa019e9f3cf1eba2ef4edecaa13f677dd8c5a50890aea1061b85c974` |
| Stack | freetype-rs 0.38.0, freetype-sys 0.23.0, bundled FreeType 2.13.2; system SDL3 3.4.14 |
| Lock SHA-256 | `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272` |
| Capture probe | [`font-format-reference.lua`](font-format-reference.lua), SHA-256 `8910d049048a04a0b555b9ddf157831a5d58f624f747a960abcebbd119b62919` |
| Metadata helper | [`font-format-metadata.rs`](font-format-metadata.rs), SHA-256 `89d011f40d135c7f2132ad27dd5d84b0c548ff0586df8b5bd9e15fce274058b2` |
| Metadata executable | SHA-256 `547e3607f4f3cbe5f91ad398b95410f3dfb5fa9d44b67207abb615f8b4727407`; built against existing debug freetype-rs/FreeType artifacts, not used to render the reference images |
| Runtime | Linux x86_64, default surface renderer, offscreen, surface scale 1, `SDL_FRAMEBUFFER_ACCELERATION=0` |
| Per-case limits | 30 s wall, 20 s CPU, 512 MiB address space; one font/size/mode per process; isolated stage, home and user directories and minimal environment |

Acceleration is disabled to exclude the [offscreen NVIDIA framebuffer growth](font-retention-classification.md), not to claim real-window or GPU coverage. No production code, dependencies, existing probes or golden baselines were changed.

## Fixtures and provenance

[`fixtures.tsv`](font-format-reference/fixtures.tsv) records all 12 files' basenames, byte sizes, SHA-256, package/source versions, table tags and licences. No font binaries or local paths are published. Existing bundled and baseline fonts are controls, not a repeated full matrix.

| Fixture | Source/licence evidence |
| --- | --- |
| JetBrains Mono / TeX Gyre Heros / Noto Sans CJK collection | Existing [baseline provenance](freetype-baseline/font-provenance.tsv); OFL-1.1 / GUST Font License / OFL-1.1 |
| Static/variable COLRv1 test fonts | Local ttf-parser 0.25.1 fixtures; adjacent `colr_1_LICENSE` (Apache-2.0); [source directory](https://github.com/harfbuzz/ttf-parser/tree/v0.25.1/tests/fonts) |
| JuliaMono 0.055 | Public Ghostty source fixture, embedded OFL notice and `res/OFL.txt`; [JuliaMono](https://github.com/cormullion/juliamono) |
| Noto Color Emoji 2.051 | Public system package, embedded OFL notice; [upstream](https://github.com/googlefonts/noto-emoji) |
| Spleen 8×16 BDF/PCF/OTB | One design in three containers, from public Ghostty fixtures; [Frederic Cambus, copyright 2018–2024](https://github.com/fcambus/spleen), BSD-2-Clause |
| Terminus TTF 4.49.3 | Public Ghostty fixture, `res/README.md` and OFL text; [upstream](https://files.ax86.net/terminus-ttf/) |
| URW Palladio Type 1/PFB | Ghostscript font package: Valek Filippov's URW collection release 1.0.7pre22; package `COPYING` is GPL-2.0 and `README` supplies the document-embedding exception. [Ghostscript](https://www.ghostscript.com/) |

The [Ghostty fixture licence index](https://github.com/ghostty-org/ghostty/blob/main/src/font/res/README.md) attributes JuliaMono, Terminus and Spleen and links their licence texts. Individual file hashes, rather than the moving source checkout, identify the tested data. No new fonts were downloaded.

## Method and artifacts

1. Inspect the SFNT table directory/version fields (where applicable), rather than inferring formats from names. The COLRv1 fixture also contains one v0-compatible base record; a standalone COLRv0 file was not available. JuliaMono's SVG table has one document for glyph 11482, containing coloured paths; that same glyph also has a `glyf` outline.
2. The metadata helper uses the existing FreeType binding, **face 0/default instance only**. It records available strike ppem, set-size status, actual glyph index, metric-load result, glyph format, bitmap dimensions/mode/payload presence and a small already-loaded mono bitmap. It uses unhinted metrics-only loading and does not render a glyph or implement a font parser.
3. For every case, the target glyph index is nonzero. Both U+10FFFF and U+10FFFE controls have index 0. Do not assume that NUL is unmapped: the emoji font maps it.
4. The editor probe independently calls `renderer.font.load`, `get_width` and deferred `draw_text`/`end_frame`. For successful loads, it captures empty, target and both missing-glyph scenes on black. White text plus grayscale/mono AA prevents LCD fringes from masquerading as colour content.
5. A distinct priming frame forces full repaint before each capture; the probe asserts dirty coverage, returned pen versus measured advance, empty-scene cleanliness and agreement of the two glyph-0 controls. Capture scans cover the entire 256×180 surface; alpha is excluded.

| Artifact | Contents |
| --- | --- |
| [`cases.tsv`](font-format-reference/cases.tsv) | 20 font/size/hint/AA cases; explicitly records the safe mono BDF case |
| [`metadata.tsv`](font-format-reference/metadata.tsv) | 60 target/control records, proving mapping and distinguishing face-open from set-size/glyph results |
| [`results.tsv`](font-format-reference/results.tsv) | 68 editor records, with load/measure/draw status, height/advance/returned pen, RGB hash, lit/chromatic counts, ink bbox, solid-bbox flag and sanitized errors |
| [`outcomes.tsv`](font-format-reference/outcomes.tsv) | Process status, row and assertion counts; all 20 completed without external kills |
| [`outline-contact.png`](font-format-reference/outline-contact.png) | Controls, Type 1, COLRv1 and SVG outline fallback targets |
| [`bitmap-contact.png`](font-format-reference/bitmap-contact.png) | Safe BDF mono, PCF/OTB placeholders and the hybrid bitmap-versus-outline cases |
| [`emoji-contact.png`](font-format-reference/emoji-contact.png) | Native-strike target and missing control, which are pixel-identical rectangles |

The contacts are cropped **nearest-neighbour inspection magnifications** (4× for small glyphs, 2× for emoji), with captions added outside the capture. They are not HiDPI evidence. Hashes refer to the original full surfaces, not the labelled contacts.

## Current-renderer matrix

`grayscale` / hint `none` unless stated. These results qualify only the sampled files, glyphs, sizes and paths, not entire format families.

| Input / target | Size | Load and measure | Draw observation |
| --- | ---: | --- | --- |
| TTF / CFF / CFF2 collection controls | 16 | All succeed; target IDs are nonzero | Distinct mapped outlines, not glyph-0 controls |
| COLRv1 static, U+F0100 / U+F0200 | 16 | Advance 16 | Monochrome base-outline rectangles, 168 / 256 lit pixels; **not** COLR paint rendering |
| COLRv1 variable, default instance, U+F0200 | 16 | Advance 16 | Same 256-pixel base-outline rectangle as the static sample |
| JuliaMono SVG-bearing U+E800 (ID 11482) | 16 | Advance 9.59375 | 51 monochrome lit pixels from its outline fallback; no SVG-only fixture was tested |
| JuliaMono ordinary U+0041 | 16 | Advance 9.59375 | 60 lit pixels, ordinary outline control |
| Noto CBDT/CBLC U+1F600 (ID 883) | 16 | `invalid pixel size` | Not reached; **face-open succeeds separately**, but this requested size does not |
| Same emoji, native strike | 109 | Height 128, advance 136 | Solid 134×128 rectangle, 17,152 lit pixels; identical to both missing controls |
| Spleen BDF U+0041, **AA none** | 16 | Height 16, advance 8 | Correct mono bitmap: 44 lit pixels, 7×10 ink bbox; matches FreeType-provided bits |
| Same BDF at unsupported size | 18 | `invalid pixel size` | Not reached |
| Spleen PCF / OTB U+0041 | 16 | Height 16, advance 8 | Each is a solid 7×16 rectangle (112 pixels), identical to missing controls |
| Same PCF / OTB at unsupported size | 18 | `invalid pixel size` | Not reached |
| Terminus hybrid U+0041 | 16 | Height 17, advance 8; bitmap strike selected | Solid 7×17 rectangle (119 pixels), identical to missing controls; also at hint `slight` |
| Same hybrid, non-strike size | 17 | Height 18, advance 8.5; outline selected | 55 lit pixels, nonrectangular and distinct from missing controls |
| URW Palladio Type 1/PFB U+0041 | 16 | Height 20, advance 12.453125 | 72 lit pixels from its outline, distinct from missing controls |

A solid bbox alone is not proof of failure: the COLRv1 test glyphs genuinely have rectangular base outlines. The metadata and missing-control comparison distinguish them from the generic fallback rectangles in the bitmap cases.

### Why bitmap containers differ

[The bitmap loader](../../../src/renderer.rs#L585) passes `FT_LOAD_BITMAP_METRICS_ONLY` even on the raster path. Metadata confirms null bitmap payloads for PCF, OTB, Terminus's strike and CBDT. FreeType's PCF/SFNT drivers honour that flag; its BDF driver ignores the load flags and supplies a real packed bitmap. Rendering a slot already classified as `bits` does not turn a metrics-only slot into decoded pixels.

[The early rejection](../../../src/renderer.rs#L613) keeps only an advance for empty buffers or modes other than MONO/GRAY/LCD; BGRA is not accepted. [The draw fallback](../../../src/renderer.rs#L1071) turns that null surface into a solid text-colour rectangle. This explains why nonzero mapping and a successful `draw_text` call are insufficient evidence.

**Packed-mono safety caveat:** [the atlas copy](../../../src/renderer.rs#L643) unpacks bits only when AA is `none`; otherwise it copies `bmp_width` bytes per row. Here the BDF bitmap is 8 pixels wide with 1 byte per row. Treating that as 8 bytes per row corrupts the image and can read seven bytes beyond a 16-byte payload on the final row. The provisional grayscale result failed an independent bitmap-equality check. Further grayscale/subpixel BDF capture was censored; final published captures use AA `none` and match the supplied mono bits. This is a pre-existing reference defect, not a proposed compatibility target or an authorized fix.

The inspected FreeType 2.13.2 source points are `src/pcf/pcfdrivr.c:526`, `src/bdf/bdfdrivr.c:790`, `src/sfnt/ttsbit.c:1625–1651` and `src/base/ftobjs.c:4865–4868`, in freetype-sys 0.23.0's bundled tree. It is available from the pinned Cargo package; no local installation path is needed to identify it.

### Why colour-bearing files remain monochrome

The current load flags never enable `FT_LOAD_COLOR`; the tested COLRv1/SVG targets arrive as `outl` slots with nonzero outline points. [The renderer's flag selection](../../../src/renderer.rs#L272) and supported pixel-mode filter do not establish a colour/SVG pipeline. This task did not enable one. SVG-only glyphs and other colour versions remain separate unknowns.

## Untested, unavailable or censored

| Family/path | State |
| --- | --- |
| Standalone COLR/CPAL v0 file | No selected local fixture. The v1 fixture has a v0-compatible record, but that record was not separately exercised. |
| sbix | No local fixture found in the bounded public-font/fixture search. |
| SVG-only glyph without an outline fallback | Unavailable; JuliaMono's target has both representations. It cannot prove SVG rendering. |
| Bitmap GRAY2/GRAY4 and other bitmap depths | Not selected; mono BDF is not evidence for these. |
| BDF grayscale/subpixel | Censored after source inspection identified unsafe packed-mono copying. Not a successful reference mode. |
| Other Type 1 variants (PFA), other legacy/custom formats and WOFF/WOFF2 wrappers | Not investigated; PFB is not an exhaustive legacy-format audit. |
| Colour palettes/versions, variation-axis changes, face index >0 | No new selectors; tests use existing face-0/default-instance API only. |
| Optional `sdl_renderer`, real windows and density >1 | Not exercised here; still required by the comparison criteria. |
| Hostile fonts and resource exhaustion | Not certified by valid-fixture containment. The candidate comparison still owns bounded hostile-input checks. |

These are unknowns/limitations, not exclusions approved by the user.

## Candidate implications — survey only, no candidate execution

The [published backend survey](https://github.com/juniorsundar/lite-xl/blob/b8b9f40b3db851bdb26f3d135a94e23cc3616ef2/docs/wayfinder/assets/freetype-backend-survey.md) distinguishes file parsing from final output:

- Swash advertises bitmap/colour sources, but exact format/version coverage needs actual fixtures; its older Skrifa dependency cannot inherit every capability of the latest Skrifa by assumption.
- Direct Skrifa + Zeno is an outline/hint/raster composition, not a complete bitmap/colour/SVG renderer merely because tables can be read.
- Fontdue's inspected raster path is outline-based. ab_glyph's raw bitmap/SVG access is not an image decoder/compositor.
- The survey does **not establish standalone BDF, PCF or Type 1/PFB input support** for any proposed composition. "PostScript outlines" inside OpenType does not prove standalone Type 1 support.

The comparison must answer:

1. Are the working Type 1/PFB and safe mono-BDF input paths required, supported by a small existing-library composition, or explicitly excluded by the user? Neither is silently dropped because bundled fonts use SFNT.
2. For COLRv1/SVG-bearing files, does a candidate preserve usable monochrome fallback? Colour support is not newly required by successful parsing or these current monochrome results.
3. What happens at native bitmap strikes and at unsupported requested sizes? Do not count generic rectangles as correct glyphs, and do not copy the unsafe packed-mono path as compatibility.
4. What disposition does the user give each remaining relevant unsupported or untested case? Missing evidence cannot become a pass by closing this task.

## Validation

The [validation summary](font-format-reference/validation.txt) records the checks below.

- Two complete collections, each **20 processes / 68 rows / 420 probe assertions**, produced byte-identical final TSVs and 64 full-surface PPMs. All processes exited 0; four cases recorded clean load failures rather than claiming successful rendering.
- Metadata proves **20 nonzero target mappings and 40 zero control mappings**. Every face opens; set-size failures remain separate from parser failure.
- Independent PPM decoding reproduces all 64 RGB FNV-1a hashes, lit counts and chromatic counts. Empty scenes have no ink; both glyph-0 controls agree for every successful load.
- The BDF mono ink shape matches the FreeType-supplied 1-bit bitmap exactly (44 lit pixels, 7×10 bbox). The unsafe grayscale provisional case is not part of the final image set.
- Lua syntax, Rust formatting, metadata-helper mapped/unmapped cases and rejection of invalid helper input were checked. The final contact sheets are numerically achromatic; visual-agent colour impressions are not evidence of coloured source pixels.
- This task makes no live-window readability, comprehensive format support, hostile-font safety, latency or candidate-qualification claim.

## Reproduce

Use an isolated checkout of the source revision and its development shell. Keep `$E` pointing to this report's evidence directory in the current checkout: the new probes/tables do not exist in the historical source commit. Supply an external directory `$F` containing the basenames in `fixtures.tsv` (symlinks to licensed local files are sufficient), and verify each file against that table's SHA-256 before use. Do not substitute private fonts into published results.

```sh
# Set E to the evidence directory before entering the pinned checkout/dev shell.
# E="$(pwd)/docs/wayfinder/assets"
T=$(mktemp -d)
# Run in the matching build environment; all build/stage outputs stay in T.
cargo build --release --features test_hooks --target-dir "$T/target"
LITE_RELEASE=1 scripts/install-linux.sh "$T/target/release/lite-xl" "$T/stage"
cp "$E/font-format-reference.lua" "$T/stage/usr/local/share/lite-xl/"
mkdir -p "$T/out" "$T/home" "$T/userdir" "$T/tmp"

# Helper: same locked FreeType dependency, debug rlib avoids the release LTO link issue.
cargo build --target-dir "$T/target"
R=$(find "$T/target/debug/deps" -name 'libfreetype-*.rlib' -print -quit)
L=$(dirname "$(find "$T/target/debug/build" -name libfreetype2.a -print -quit)")
rustc -O --edition 2021 --crate-name font_format_metadata \
  --extern "freetype=$R" -L "dependency=$T/target/debug/deps" -L "native=$L" \
  -l static=freetype2 -l static=png -l z \
  -o "$T/metadata" "$E/font-format-metadata.rs"

D="$E/font-format-reference"
while IFS="$(printf '\t')" read -r id fixture size hint aa cp; do
  [ "$id" = id ] && continue
  name=$(awk -F '\t' -v k="$fixture" '$1==k {print $2}' "$D/fixtures.tsv")
  font="$F/$name"
  expected=$(awk -F '\t' -v k="$fixture" '$1==k {print $4}' "$D/fixtures.tsv")
  [ "$(sha256sum "$font" | cut -d ' ' -f 1)" = "$expected" ] || exit 1
  timeout -k 2 30 prlimit --as=536870912 --cpu=20 -- "$T/metadata" "$id-target" "$font" "$size" "${cp#U+}"
  timeout -k 2 30 prlimit --as=536870912 --cpu=20 -- "$T/metadata" "$id-missing" "$font" "$size" 10FFFF
  timeout -k 2 30 prlimit --as=536870912 --cpu=20 -- "$T/metadata" "$id-missing2" "$font" "$size" 10FFFE
  env -i PATH="$PATH" LANG=C.UTF-8 HOME="$T/home" TMPDIR="$T/tmp" \
    SDL_VIDEODRIVER=offscreen SDL_FRAMEBUFFER_ACCELERATION=0 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent SDL_FILE_DIALOG_DRIVER= \
    LITE_PREFIX="$T/stage/usr/local" LITE_XL_RUNTIME=font-format-reference \
    LITE_USERDIR="$T/userdir" LITE_OS_HOME="$T/home" \
    FORMAT_ID="$id" FORMAT_FONT="$font" FORMAT_SIZE="$size" FORMAT_CP="${cp#U+}" \
    FORMAT_AA="$aa" FORMAT_HINT="$hint" FORMAT_OUT="$T/out" \
    timeout -k 2 30 prlimit --as=536870912 --cpu=20 -- "$T/stage/usr/local/bin/lite-xl"
done < "$D/cases.tsv"
```

The `freetype2`/`png`/`z` link flags above match the recorded bundled build; check freetype-sys's build output if the environment differs. The helper is supporting metadata, not a replacement renderer. For inspection, use `magick capture.ppm -crop 64x48+0+0 +repage -sample 256x192! view.png` on small glyphs; retain the original surfaces for hash comparison. Do not add the censored packed-mono grayscale/subpixel mode to the run.
