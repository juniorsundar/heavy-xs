# Mono rendering alternatives (Rust-native, no new rasterizer)

Status: research only. No implementation, dependency, build, or capture.
Context (historical measurements, not approved rendering): screen pins Skrifa 0.48.0 +
Zeno 0.3.3; seven fonts, size 16, 9 modes, 3 phases; unhinted Thin CFF2
default-instance keeps 42/77 lit pixels in FT native mono but 6/9 with
alpha>=128. The >=128 (50%) conversion stands rejected. Hint `none` stays
genuinely unhinted; no synthetic embolden, no forced hinting, no format
exclusions.

## 1. Zeno 0.3.3 has no mono mask-rendering mode

- `Format` (0.3.3 docs) offers only `Alpha`, `Subpixel`,
  `CustomSubpixel([f32; 3])`; no binary/mask-bit/non-AA variant.
  Source: [Format 0.3.3](https://docs.rs/zeno/0.3.3/zeno/enum.Format.html).
  (GitHub `v0.3.3` tag did not resolve, 404; the 0.3.3 release itself is
  proven by docs.rs 0.3.3 pages resolving.)
- `HitTest` supplies point-testing controls, not an additional `Format`
  mode. Building a mask algorithm around per-point tests would be new
  custom rendering code, not use of a shipped mono mask mode; none is
  proposed here.
  Source: [HitTest 0.3.3](https://docs.rs/zeno/0.3.3/zeno/struct.HitTest.html).
- Declared deps (zeno): `libm` optional only; everything else is
  dev-dependencies. Licence `Apache-2.0 OR MIT`. Full transitive closure
  NOT audited; statement covers declared deps only.
  Source: [Cargo.toml 0.3.3](https://docs.rs/crate/zeno/0.3.3/source/Cargo.toml).

## 2. Candidate A: tiny-skia 0.12.0 non-AA fill (`anti_alias = false`)

- Release exists: [Paint docs 0.12.0](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Paint.html)
  resolve (docs.rs builds from the crates.io publish), and the
  [v0.12.0 tag](https://raw.githubusercontent.com/linebender/tiny-skia/v0.12.0/Cargo.toml)
  resolves. Version pinned 0.12.0 everywhere below.
- Explicitly set `Paint.anti_alias = false`; do not depend on a default.
  `Pixmap(PixmapMut)::fill_path` branches on it: `scan::path_aa::fill_path`
  vs `scan::path::fill_path`; same split for rects. Verified in tag source,
  including degenerate-path early-outs and tiled paths.
  Sources: [painter.rs v0.12.0](https://raw.githubusercontent.com/linebender/tiny-skia/v0.12.0/src/painter.rs),
  [scan/mod.rs v0.12.0](https://raw.githubusercontent.com/linebender/tiny-skia/v0.12.0/src/scan/mod.rs),
  [Pixmap 0.12.0](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Pixmap.html).
- The non-AA path (`scan/path.rs`) is an edge-based scanline fill with
  conservative bounds rounding (`conservative_round_to_int`,
  `BasicEdgeBuilder`); files carry Skia/Android-port headers. It is a
  genuinely different sampling rule from thresholding Zeno's 256-level
  coverage. No claim is made about its sub-pixel sampling rule (e.g.
  center-based): not verified from source here.
  Source: [scan/path.rs v0.12.0](https://raw.githubusercontent.com/linebender/tiny-skia/v0.12.0/src/scan/path.rs).
- Limits (honest): thin-stroke retention at 16px UNTESTED (no images);
  no dropout/stem-correction control was found in the examined entry
  points (`Paint`, `fill_path`, `FillRule`); full-surface audit NOT done.
  Analysis, not measured: a thin span falling between sampling positions
  may vanish, while any-coverage filling may fatten it; neither direction
  is proven for either engine without images.
- Declared deps (0.12.0): arrayref, arrayvec, bytemuck, cfg-if, log,
  tiny-skia-path 0.12.0, png (optional). No C/C++ native dep declared;
  full closure NOT audited. Licence `BSD-3-Clause`.
  Source: [Cargo.toml v0.12.0](https://raw.githubusercontent.com/linebender/tiny-skia/v0.12.0/Cargo.toml).
- Adapter: `PathBuilder` 0.12.0 documents `move_to`/`line_to`/`quad_to`/
  `cubic_to`/`close` with Skrifa-compatible verb/point arities, matching
  the prototype's `OutlinePen` verbs (`curve_to` maps to `cubic_to`).
  Mapping verified against docs, NOT compiled or run.
  Sources: [PathBuilder 0.12.0](https://docs.rs/tiny-skia-path/0.12.0/tiny_skia_path/struct.PathBuilder.html),
  [skrifa outline 0.48.0](https://docs.rs/skrifa/0.48.0/skrifa/outline/index.html).

## 3. Candidate B: any-coverage postprocessing of STORED hint-none data

Project-specific code-path check, from the font controls prototype
([Compare candidate font controls and rendering](../issues/18-compare-font-controls-and-rendering.md)).
Its runtime data is not a public library guarantee:

- `hint == none` yields `target = None` regardless of `aa`, so BOTH the
  Mono and Grayscale hint-none cases call
  `DrawSettings::unhinted(size, location)` (`route_plan` + `run_case`).
- Both use `blend_format = Format::Alpha` (`Mono | Alpha => Format::Alpha`)
  and the identical `Mask::new(&commands).format(Alpha).size(w,h)
  .render_into(&mut mask_buffer, None)` call. Only `compose` differs:
  Mono thresholds (`alpha >= 128`), Alpha replicates raw bytes to RGB.
- Therefore stored GRAYSCALE/HINT-NONE PPM pixels ARE the raw coverage
  bytes (same value on R=G=B), and re-thresholding them with `> 0` is
  exactly equivalent to re-rendering stored masks under `> 0`. Reuse is
  legitimate for hint-none. No re-render needed for the experiment.
- Hinted Mono is different: `Target::Mono` vs `Target::Smooth(Light /
  Normal)` feed different autohinted outlines, so gray-hinted PPMs can
  NOT substitute for a hinted-Mono any-coverage test.
- Precision fixes: nonzero quantized alpha retains positive 8-bit samples,
  NOT every geometrically touched pixel (quantization can round a touch to
  0). Any-coverage fills a pixel from a single touched sub-column, i.e. it
  can fatten strokes relative to another binary rasterization rule.
  Label it **any-coverage mono conversion**, not FreeType-native mono.
  Binary output alone does not establish native-mono equivalence or
  acceptable appearance. New images still require HITL agreement.

## 4. Why FT native mono can retain strokes the >=128 cut loses

- Different engine, not a different threshold. In tag `VER-2-13-2`,
  the smooth-renderer implementation rejects mono rendering; the
  mono path uses the separate B/W rasterizer
  (`src/raster/ftraster.c`, `ft_standard_raster`).
  Sources: [ftsmooth.c VER-2-13-2](https://gitlab.freedesktop.org/freetype/freetype/-/raw/VER-2-13-2/src/smooth/ftsmooth.c),
  [ftraster.c VER-2-13-2](https://gitlab.freedesktop.org/freetype/freetype/-/raw/VER-2-13-2/src/raster/ftraster.c).
- The B/W converter detects drop-outs and corrects the bitmap by default;
  `FT_OUTLINE_IGNORE_DROPOUTS` / `SMART_DROPOUTS` / `INCLUDE_STUBS` tune it
  and "are ignored by the smooth rasterizer" (SCANTYPE semantics).
  Source: [ftimage.h VER-2-13-2](https://gitlab.freedesktop.org/freetype/freetype/-/raw/VER-2-13-2/include/freetype/ftimage.h).
  (The freetype.org reference currently renders 2.14.3; the versioned
  normative source for the 2.13.2 claim is the tag above.)
- Scope guard: this establishes the capability gap only. Attributing the
  observed 42/77 retention to a specific dropout rule is NOT proven:
  the outline flags and CFF2-specific raster behavior of that capture were
  not instrumented. The historical build is pinned to FreeType 2.13.2;
  the generic source establishes a capability, not this case's cause.

## 5. Recommendation (bounded, for HITL decision)

1. Smallest experiment first: re-threshold STORED grayscale hint-none PPMs
   under `alpha > 0`, labelled any-coverage, against FT native mono. No
   code beyond the threshold loop; the stored files already contain the
   raw coverage. (New postprocessing artifact still requires HITL sign-off.)
2. If rejected, propose a separate additional-library trial with tiny-skia 0.12.0
   `fill_path` with `anti_alias = false` on the same Thin CFF2 pair at 16.
3. Conclusion is scoped: no dedicated dropout-handling mono API was found
   in the examined candidates (Zeno 0.3.3, tiny-skia 0.12.0 entry points).
   NOT a claim that no Rust crate anywhere can do it, and NOT proof no
   Rust-native path reproduces the B/W rasterizer. If both fail, that is
   evidence only that the scoped routes fail the observed case. Return
   that gap to the human for further research or an explicit decision;
   do not implement an engine or implicitly relax any requirement.

## Gaps / untested

- tiny-skia non-AA thin-stroke behavior at 16px: no images exist.
- Skrifa-verb -> `PathBuilder` mapping: documented, not compiled or run.
- Prototype FT outline flags / CFF2-specific dropout behavior: not instrumented.
- No performance, licence-conflict, MSRV, or full dependency-closure
  review was done.
