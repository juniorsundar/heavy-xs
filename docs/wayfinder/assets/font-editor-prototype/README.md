# PROTOTYPE: candidate font backend inside the editor (increment 1)

Throwaway. `LITE_FONT_BACKEND=candidate` swaps FreeType for **Skrifa 0.48.0** (parse, metrics,
autohint) + **Zeno 0.3.3** (grayscale/subpixel) + **tiny-skia 0.12.0** (non-AA mono) at font
load. Unset or `freetype` keeps the unchanged FreeType path. One binary serves both.

## What changed

- `src/font_candidate.rs` (new): glyph id, unhinted advance, face metrics, glyph image.
- `src/renderer.rs`: each FreeType touchpoint (glyph id, advance, bitmap, face metrics, load)
  branches to the candidate; atlas, glyph cache, fallback, tabs, blending, decorations and the
  Lua API are shared and unchanged.
- `src/syswindow.rs`: window titles are prefixed `[freetype]` / `[candidate]` when the variable is set.
- Styles: italic = the same 0.25 shear; bold and smoothing = fill plus a 0.5 px stroke
  (approximating FreeType's outline embolden). Mono keeps FreeType's one-pixel atlas offset.

## Checks

- All 16 contract suites pass on the FreeType path (no regression).
- Candidate path: `rencache` passes; `renderer` differs only in pixel hashes (80 rows, expected:
  different rasterizer). Load errors, size 0/negative and `set_size 0` match FreeType's wording
  and results after fixes. The 23 font-inventory assertions pass on both backends.
- Geometry probe (`geometry-probe.lua`): bundled JetBrains Mono / Fira Sans and the user's UI
  and code fonts, sizes 12/15/19, all nine AA/hint modes, four samples (code, UI, tabs, mixed
  scripts/emoji with fallback). 13,068 invariant checks pass per backend (draw end == measured
  width, per-codepoint sums, monotonic prefixes). Heights and every measured width are
  **identical** between backends (`geometry-*.txt`; user font rows by id only).
- The user's full config (plugins included) starts and draws under both backends with no
  logged warnings or errors.

## Live review

```sh
E=docs/wayfinder/assets/font-editor-prototype; R=$(mktemp -d)
nix develop --command sh -c "cargo build --release --target-dir $R/t && scripts/install-linux.sh $R/t/release/lite-xl $R/stage"
$E/review.sh "$R/stage/usr/local" [file] [user|bundled|mode=AA/HINT]
```

## Not yet covered

Performance and memory pairs, malformed fonts, `sdl_renderer`, BDF/Type 1 (candidate fails to
load them), normal-density review (no display available). Pixel appearance is not approved.
