# Observable font contract inventory

Evidence for [Inventory the observable font contract](../issues/12-inventory-font-contract.md). This describes the current FreeType-backed layer; it selects no replacement, approves no compatibility loss and changes no production code.

## Identity and validation

Source: `d9b3db31ab79339a60fe203ece364d17eb41786a`, Lite XL 2.1.7. [Cargo.toml](../../../rust/Cargo.toml#L18) and [Cargo.lock](../../../rust/Cargo.lock#L91) select freetype-rs 0.38.0 / freetype-sys 0.23.0 with bundled FreeType; the bundled `freetype.h` defines **2.13.2**. System SDL3 is **3.4.14** by `pkg-config --modversion sdl3` (distinct from the SDL version in the bindings crate's name). Source anchors below refer to this revision; tracked `rust/`, `data/`, tests and golden baselines remained unchanged during validation.

The timed-out worker left a draft, not a completed result. The coordinating agent checked it against source, corrected its claims/citations and independently ran a fresh debug build with `test_hooks`, a separate Cargo target and stage, temporary home/user directories and SDL's offscreen driver:

- Existing **renderer and rencache contract suites pass**, byte-for-byte against their committed baselines. Their coverage combines snapshot observations with explicit assertions, including deferred-font GC lifetime; a pass does not certify every snapshot as desirable behavior.
- Existing **hit test passes 1,239,848 grid comparisons**. This compares the Rust hit algorithm with its reference fixture; it is not a real-editor font-to-cursor alignment study.
- [Bounded inventory checks](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-contract-check.lua) pass **23 assertions**; [raw output](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/font-contract-check.txt). These pin the group, path, fallback, advance and copy quirks described below, rather than inventing a new acceptance contract.
- Lua syntax, relative documentation links/source-line ranges and source/docs whitespace are checked. No golden baseline was rewritten.

Validation identities (SHA-256):

| Artifact | Hash |
|---|---|
| Fresh debug executable | `3c99a5b25aecede04d39bb33956d2a5782c0ed2a0604e0e43a0855052627a40c` |
| Cargo.lock | `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272` |
| Renderer baseline | `5b84bcf0356be30ecf579f61a9487e399030ef54004cab7331b9922324ec5200` |
| Rencache baseline | `23e89212c82979bbdc095599a41ec18139baa5e0ccfecb9e086da09ed6bd1443` |

**Evidence levels:** source descriptions below are source-read unless a fixture/probe is cited. The cited suites and 23-assertion probe were exercised now. No real-window review, HiDPI >1 study, candidate comparison, font-format campaign, or latency/cache-memory baseline was performed.

## Lua API and object shapes

[Native entry points and metatable](../../../rust/src/renderer_api.rs#L582): a Font is userdata holding one `RenFont *`; a font group is a plain Lua table with the same Font metatable. This distinction matters to Lua consumers. `renderer.font` exposes load/copy/group/set_tab_size/get_width/get_height/get_size/set_size/get_path and `__gc`; **there is no Lua `get_tab_size` method**. The similarly named helper in [renderer.rs](../../../rust/src/renderer.rs#L892) is internal. The retired plan's broader API list is not authoritative.

| Entry point | Current behavior | Source / exercised fixture |
|---|---|---|
| `renderer.font.load(path, size, opts?)` | Face index 0; size converted to `f32`; returns Font or raises. | [load](../../../rust/src/renderer_api.rs#L162), [fixture](../../../tests/contract/cases/renderer.lua#L34) |
| `renderer.font.group(table)` | Returns the same table, length 1–10. Constructor checks **userdata type only**, not the Font metatable. Nested tables/groups and non-userdata fail; foreign full userdata can be admitted. Readers subsequently validate each entry as Font and reject foreign userdata. | [constructor](../../../rust/src/renderer_api.rs#L259), [reader](../../../rust/src/renderer_api.rs#L185), probe |
| `Font:copy(size?, opts?)` | Reloads the stored path into independent fonts/caches. Omitted size defaults to the **first font's height**, not its requested size; explicit nil raises. Group input returns a new group table. | [copy](../../../rust/src/renderer_api.rs#L221), [reload](../../../rust/src/renderer.rs#L841), [fixture](../../../tests/contract/cases/renderer.lua#L124), probe |
| `Font:get_path()` | Stored, uncanonicalized C-string path; group returns a table of paths. An embedded NUL in the Lua input terminates the path, not an exact round-trip of the entire Lua string. | [path](../../../rust/src/renderer_api.rs#L286), [storage](../../../rust/src/renderer.rs#L824), probe |
| `Font:set_tab_size(n)` | Applies to each group member; number converts to `c_int`, then `u16`. Returns no values. | [setter](../../../rust/src/renderer_api.rs#L305), [storage](../../../rust/src/renderer.rs#L883), [fixture](../../../tests/contract/cases/renderer.lua#L56) |
| `Font:get_size()` / `get_height()` | Group values come from the **first font**, not the maximum. Size is the stored request; height is the derived integer metric returned as a Lua number. | [API](../../../rust/src/renderer_api.rs#L355), [metrics](../../../rust/src/renderer.rs#L899), probe |
| `Font:set_size(size)` | Changes all members, clears glyph caches and resets tabs to 2. The setter ignores a failed metric-refresh return; unlike load, it does not promise an error for every invalid size. | [API](../../../rust/src/renderer_api.rs#L371), [implementation](../../../rust/src/renderer.rs#L910), [fixture](../../../tests/contract/cases/renderer.lua#L69) |
| `Font:get_width(text, tab?)` | Sums per-codepoint advances with optional `{tab_offset=number}`; does not perform shaping or pair kerning. | [API](../../../rust/src/renderer_api.rs#L340), [loop](../../../rust/src/renderer.rs#L975), probe |
| `renderer.draw_text(font_or_group, text, x, y, color?, tab?)` | Queues a deferred draw, anchors the font/group, and returns `x + measured_width` even if the run is clipped. `y` converts to integer; x remains fractional. Missing color defaults to white. | [API](../../../rust/src/renderer_api.rs#L543), [cache](../../../rust/src/rencache.rs#L315), [return fixtures](../../../tests/contract/cases/renderer.lua#L322) |

Readers clamp a plain table longer than 10 to 10 entries; the group constructor instead rejects it. Empty/mutated groups are rejected by readers rather than dereferenced. These are observed/implemented quirks, not endorsements of keeping them forever.

## Options, hinting and rasterization

[Option parsing](../../../rust/src/renderer_api.rs#L80), [load flags](../../../rust/src/renderer.rs#L272), [render mode](../../../rust/src/renderer.rs#L288), [fixture matrix](../../../tests/contract/cases/renderer.lua#L191):

- Defaults: AA `subpixel`, hinting `slight`, no styles, tab size 2. Accepted AA values are `none`, `grayscale`, `subpixel`; hinting is `none`, `slight`, `full`. Unknown string values raise. A non-table options argument and unknown keys are ignored. Recognized non-string values follow `lua_isstring` behavior (numbers are string-coercible), rather than uniform strict validation.
- Styles are Lua-truthy fields `bold`, `italic`, `underline`, `smoothing`, `strikethrough`. Nonzero style combinations replace the style mask. Copy uses `-1` to inherit omitted options; a zero resulting style mask leaves that sentinel intact, so `copy(..., {bold=false})` does **not** necessarily clear inherited bold.
- Bitmap load target: AA none → `TARGET_MONO`; otherwise slight hinting → `TARGET_LIGHT`, else `TARGET_NORMAL`. Hinting none adds `NO_HINTING`; all hinted bitmap paths add **`FORCE_AUTOHINT`**. Reproducing FreeType's TrueType bytecode interpreter is therefore not itself an established requirement of this renderer's hinted path.
- Render modes: mono, LCD, or normal/light grayscale. LCD hinting-none disables the filter; hinted LCD sets custom five-tap weights `0x10,0x40,0x70,0x40,0x10`. Exact candidate filtering/gamma equivalence is untested. The fixture's mono/slight and mono/full pixel hashes are equal, as their bitmap options coincide.
- [Bitmap loader](../../../rust/src/renderer.rs#L564) accepts MONO, GRAY and LCD pixel modes only. Empty/unsupported bitmaps retain their advance without an atlas surface; the draw loop may use a filled rectangle. No color-glyph rendering capability is established merely because FreeType can open a color-font file.

[Synthetic styles](../../../rust/src/renderer.rs#L311) modify outlines: bold emboldens X by `1<<5`, smoothing emboldens both axes by `1<<5`, italic shears with `xy=1<<14`; they run after the phase translation and before rasterization. Underline/strike are [run rectangles](../../../rust/src/renderer.rs#L1180), not glyph transforms: enable bits come from the first font while rectangle geometry follows the previous run's font. These source behaviors need reviewed candidate comparisons, not assumptions that similarly named options match.

## Metrics, fallback, tabs and Unicode

[Face metrics](../../../rust/src/renderer.rs#L732) scale height/ascender from units-per-em for scalable fonts, with integer casts (through `i16` for height/baseline); underline thickness falls back to `ceil(height/14)` when zero. Non-scalable metrics use a cast-before-division path. Space advance is loaded unhinted. Group height/size/baseline policy is first-font based; glyph drawing uses the first font's baseline with each glyph's bitmap bearings.

Crucially, **glyph advances are also loaded unhinted**, explicitly removing `FORCE_AUTOHINT`, independently of bitmap rendering ([metric loader](../../../rust/src/renderer.rs#L522)). The probe confirms Fira's `AV` width is identical across all nine AA/hint combinations. Existing size-14 examples are mono width `A=8.40625`, height 18; Fira `A=8.015625`, `AV=15.796875`, space `3.703125`, height 16. The probe confirms `AV = A + V`, not pair kerning.

[Fallback selection](../../../rust/src/renderer.rs#L665) / [advance policy](../../../rust/src/renderer.rs#L943):

- First group member mapping the codepoint wins; whitespace always selects the first font. If none has it, the **last font's glyph 0 (`.notdef`)** is attempted. This is not an automatic U+25A1 substitution.
- U+25A1 retry occurs only when metric loading fails or produces no flags, and excludes whitespace and the box itself. A loaded `.notdef` normally prevents that retry. Missing bitmap output can instead cause the [draw-loop rectangle](../../../rust/src/renderer.rs#L1071). These are different paths; missing-codepoint snapshots alone do not prove the explicit box retry.
- All whitespace and any non-whitespace glyph with zero/unavailable advance receive the **first font's space advance** (tabs have their own rule). In particular, the probe observes a nonzero space width for Fira's U+0301 combining mark. This is current behavior, not correct complex-text layout or a decision to preserve a bug.
- The probe exercises a genuine second-font glyph using `{icons, Fira}` with `é`, last-font `.notdef` with `{Fira, icons}` and U+4E2D, and first-font whitespace using U+3000. Existing CJK/PUA examples mostly exercise missing glyphs rather than actual CJK/emoji coverage.

Tab width is `space_advance * tab_size`. Without offset it is a plain advance; with offset the next stop uses `(curr_x + tab_offset) % tab_width`, skipping a stop if less than one space remains. Offset absent/nil means NaN internally; wrong table/value types raise ([checktab](../../../rust/src/renderer_api.rs#L326)). Zero tab size plus an offset reaches modulo-zero/NaN; the existing fixture distinguishes the offset case from the no-offset zero advance. `DocView` [reapplies tabs](../../../data/core/docview.lua#L176) and [passes run-relative offsets](../../../data/core/docview.lua#L461).

Fractional raster x selects three cached phases: `(pen_x % 1 * 3)` with translations of `index*(64/3)` outline units; negative phase indices are adjusted. The cache's text geometry truncates sums/widths while [rectangle coordinates](../../../rust/src/renderer_api.rs#L506) use `+0.5` casts. Measurement and raster placement must be compared together.

The renderer has its own [lenient UTF-8 decoder](../../../rust/src/renderer.rs#L370): no continuation-byte validation, overlong/surrogate values accepted, and a truncated sequence can consume the NUL at the buffer's end. Values above U+10FFFF map to glyph 0. Existing fixtures cover malformed bytes/NUL/controls as widths and pixels; this is not decoding correctness or memory-safety certification. There is no bidi, shaping or grapheme engine in this per-codepoint loop.

## Loading, font scope and callers

[Loading](../../../rust/src/renderer.rs#L777) uses an SDL-backed [FreeType stream](../../../rust/src/ft.rs#L16), `FT_Open_Face` at index 0 and no filename/format filter. It attempts whatever this bundled build opens; neither arbitrary FreeType format coverage nor a face/variation-axis selector is exposed by this API. The bundled fixture set is three TTFs; OTF/CFF, collections, bitmap-only, variable, SVG/color-font behavior are **unverified**, not approved exclusions. File opening, available glyphs, outline support and rendered pixel formats are separate questions.

`fc-query --format '%{family}\n%{charset}\n' data/fonts/<file>` and `sha256sum` identify the actual repository fonts:

| File | Coverage observations | SHA-256 |
|---|---|---|
| FiraSans-Regular.ttf | Fira Sans; Latin/Greek/Cyrillic and punctuation; no U+25A1, CJK or emoji in the queried charset | `bc51f6412e9ecea0d158ea0fe6b757b6b4b49af4105fb4802752062dfc710b70` |
| JetBrainsMono-Regular.ttf | JetBrains Mono; contains U+25A1 and Powerline `E0A0–E0A2`, `E0B0–E0B3`; no tested CJK/emoji codepoints | `c95c87d1d160fb925f876d7da574a5ce38f7d4f69e215447a56b800fa18870b2` |
| icons.ttf | `icons`; ASCII icon mappings `21 2b 2d 35–39 3c 3e 42–44 4c–4d 50 53 57–58 5f 64 66–69 77`; no PUA | `082f7d2cd1412386fb99b1100a63116ab81e521bb1cb486f338e6f25e495f073` |

[Style defaults](../../../data/core/style.lua#L38) load UI Fira at `15*SCALE`, icons at `16*SCALE` with grayscale/full hinting, and code Mono at `15*SCALE`; big variants are copies. The [user-init template](../../../data/core/init.lua#L145) documents arbitrary font paths/options. It does **not** prohibit group overrides. [Per-token fonts](../../../data/core/style.lua#L48) can differ from the default code font.

Main consumers: [common.draw_text](../../../data/core/common.lua#L363) for alignment and dimensions; [DocView](../../../data/core/docview.lua#L134) for line height, column↔x, tabs, selection and drawing; command/context/status/title views for layout and icons; [scale plugin](../../../data/plugins/scale.lua#L57) for resizing; [config.stonks.font](../../../data/core/statusview.lua#L308) for a configurable status font. Existing UI icons use ASCII such as `f`/`g`; the fixture's [PUA icon example](../../../tests/contract/cases/renderer.lua#L280) uses glyphs absent from icons.ttf. A replacement corpus must render the actual mapped icons too.

## Lifetime, cache, scale and errors

Each [RenFont](../../../rust/src/renderer.rs#L120) owns a Face and path. [Free](../../../rust/src/renderer.rs#L873) clears metrics/atlases and drops the Face; group-table `__gc` is a no-op, with Lua references owning member lifetimes. The FreeType Library is [initialized/freed process-wide](../../../rust/src/renderer.rs#L1292). [draw_text](../../../rust/src/renderer_api.rs#L543) roots fonts/groups in a registry table until [end_frame](../../../rust/src/renderer_api.rs#L494) flushes deferred commands and clears the roots. The existing [GC assertions](../../../tests/contract/cases/renderer.lua#L329) pass.

Per-font caches: lazy codepoint→glyph mapping, glyph metrics, and grayscale INDEX8 / LCD RGB24 atlas surfaces; LCD has three phase slots. Resizing/free clears caches; copy reloads rather than sharing; tabs reset to 2. Allocator size accounting is internal, not a Lua resource limit. Checked SDL allocations use [check_alloc](../../../rust/src/renderer.rs#L155), which exits on null; this is not a universal claim that every FreeType allocation error is fatal. No per-font/cache memory budget or hostile-font bound was established by this inventory.

Lua [SCALE](../../../data/core/start.lua#L8) multiplies requested sizes. Separately, the optional SDL-renderer feature uses a surface pixel/point scale: [retrieval](../../../rust/src/renderer_api.rs#L185) refreshes fonts when needed; [refresh](../../../rust/src/renderer.rs#L1329) sets the group to the first font's size, resets tabs and clears caches. Widths are divided by scale for callers, pixels/positions are scaled for drawing. The default build's surface scale is 1. The optional feature and scale >1 were source-read, **not exercised now**.

Load and copy failures raise `failed to load/copy font: ...`; errors can originate from SDL I/O or the [FreeType error mapping](../../../rust/src/renderer.rs#L164). [Existing error cases](../../../tests/contract/cases/renderer.lua#L92) include missing/empty/directory/non-font paths, wrong argument types and invalid options/sizes. Error acceptance is not uniform across load and set_size. Other group/table/type errors come from native Lua checks. The Rust layer deliberately fixes the old C unknown-hinting diagnostic and rejects malformed groups rather than null-dereferencing ([module notes](../../../rust/src/renderer_api.rs#L8)).

## Coverage gaps and next evidence

The [existing renderer suite](../../../tests/contract/cases/renderer.lua#L178) covers the AA/hinting matrix, sizes, fractional phases, styles, blending/clipping, tabs, mixed font groups, whitespace, invalid bytes, long/many draws, copying/resizing and deferred GC. Its [surface hash](../../../rust/src/test_hooks.rs#L121) hashes logical RGB and counts non-black pixels, excluding alpha; it is build-specific evidence, not a visual quality verdict. Rencache fixtures exercise command/dirty-cell behavior; the pure hit test does not establish font-layout correctness end to end.

The [reference-baseline task](../issues/14-collect-freetype-baseline.md) should address:

1. Real mapped ASCII icons, actual cross-font glyphs, absent codepoints, metric-failure box retry and empty/unsupported bitmap paths as distinct cases. The new probe adds a real second-font **width** assertion, not a new pixel rendering corpus.
2. Representative redistributable custom fonts/paths with provenance: at least proportional, monospace and actual CJK fallback coverage; OTF/CFF and a collection if they are to be evaluated. Separate default-instance loading from any axis/face selection, which is not currently exposed. Publish no private fonts/configuration.
3. Surface scale >1 and optional SDL-renderer coverage, real-window readability and cursor/drawing/measurement alignment. These have not been certified here.
4. Fresh load/first glyph vs warm cache, repeated resizing/copying and many-font memory/retention; calibrate criteria afterward. No timing/memory thresholds are selected here.
5. Malformed font tables and text, extreme sizes/tabs and resource exhaustion. Existing fixtures are bounded observations, not font-parser fuzzing or safety proofs.

Color, bitmap, variable and other format behavior remains unclassified until inventory/evidence supports a live compatibility decision; it is not silently ruled out. No candidate, fork, feature removal or new shaping/fallback architecture is approved by this report.

## Reproduce

From repo root (fresh isolated target/stage; original baselines only compared):

```sh
run=$(mktemp -d /tmp/lxl-font-validation.XXXXXX)
mkdir -p "$run/userdir" "$run/home"
export TARGET_DIR="$run/target" LITE_USERDIR="$run/userdir" LITE_OS_HOME="$run/home"
nix develop --command scripts/rust-port/verify.sh renderer,rencache
nix develop --command scripts/rust-port/hit-test-check.sh
cp docs/wayfinder/assets/font-contract-check.lua "$TARGET_DIR/stage/usr/local/share/lite-xl/"
DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/lxl-font-inventory-bus SDL_FILE_DIALOG_DRIVER= \
SDL_VIDEODRIVER=offscreen LITE_PREFIX="$TARGET_DIR/stage/usr/local" \
LITE_XL_RUNTIME=font-contract-check timeout -k 2 60 "$TARGET_DIR/debug/lite-xl"
```

Observed local validation artifacts: `/tmp/lxl-font-validation.7QbCld` (temporary, not a portable dependency). Portable evidence is the scripts, committed reference baselines, raw probe output and identities above. Release/optional-renderer variants and full application regression suites were not rerun for this documentation-only inventory.
