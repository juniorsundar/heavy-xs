# Establish additional font-format reference coverage

Parent: ../freetype-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-font-format-reference
Blocked by: 15

## Question

Beyond the mandatory TrueType/glyf, OpenType/CFF and default-instance CFF2 collection samples, which additional font formats actually load and draw through the current editor API, and which known or untested cases must the candidate comparison put before the user?

This task produces reference evidence for [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) and [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md). It does not decide format exclusions or approve a candidate. The user requires explicit disposition of relevant support gaps before selection; missing evidence is not compatibility and is not an approved exclusion.

## Scope

- Reuse [the font contract inventory](../assets/font-contract-inventory.md) and [the reference baseline](../assets/freetype-baseline.md). Keep the baseline's mandatory formats as controls rather than recreating that investigation.
- Build a bounded matrix of additional format families. Prioritize colour/bitmap/SVG cases (COLR/CPAL, CBDT/CBLC, sbix, SVG, bitmap-only) and record Type 1/PCF or other legacy/custom formats when encountered or identified as a relevant gap. Inventory remaining known families as untested if fixtures or execution are unavailable; do not claim exhaustive FreeType coverage.
- Use existing redistributable local fonts first. For each fixture, record source/licence evidence, font hash, relevant tables or format, face/default-instance identity and tested glyph mappings. Publish no private fonts, configuration or local paths. Do not commit new font binaries without separate approval.
- Exercise the actual `renderer.font.load` → `get_width` → `draw_text` path. Distinguish opening a face, obtaining metrics and rendering a mapped glyph. Record whether the current editor produces colour, a monochrome rendering, `.notdef`, blank output or an error; FreeType parser capability alone does not prove editor support.
- Stay within the existing API: face 0 and default variation instance. No new face/axis selector, shaping, font discovery service or new colour-rendering feature is authorized.
- Use isolated release/test-hooks builds, staging and user directories. Record source/build/FreeType identities, deterministic measurements, inspectable output where useful and bounded time/memory containment. External kills, unavailable fixtures and unsupported paths must be recorded distinctly.

## Deliverable

Save a compact evidence report and bounded probe/raw artifacts under `docs/wayfinder/assets/`. Include:

1. A tested/untested format matrix with reproduction commands and licensing/provenance.
2. Current editor load/measure/draw results, including whether the intended glyph really exists in the fixture.
3. Known candidate gaps from the existing survey, clearly separated from candidate execution, which belongs to the comparison ticket.
4. A list of format questions requiring the user's disposition during comparison: required support, approved exclusion or blocker. Record unknowns explicitly; do not silently drop them.

No production changes, backend selection, golden rewrites, custom parser/rasterizer or invented acceptance thresholds. The user did not require every FreeType format, but has not approved removing untested ones either.

## Comments

Created from the user's confirmed acceptance criteria. This task supplies missing reference coverage; the candidate comparison owns candidate measurements and the human's actual support-gap decisions. It can run independently of [Classify font memory retention in the current renderer](19-classify-font-memory-retention.md).

### Resolution

[The additional-format reference report](../assets/font-format-reference.md) records 12 provenance/licence-identified local public fixtures, 20 contained editor cases, 68 load/measure/draw records and 60 independent FreeType mapping/strike records. [Raw tables and contact sheets](../assets/font-format-reference/) and [the validation summary](../assets/font-format-reference/validation.txt) are linked from it; [the Lua capture probe](../assets/font-format-reference.lua) and [the existing-binding metadata helper](../assets/font-format-metadata.rs) reproduce the evidence without a custom font parser or new production hooks.

- Type 1/PFB renders the sampled outline; BDF renders its native-strike glyph with AA `none`, bit-for-bit equal to FreeType's supplied mono bitmap. These are concrete extra input paths whose removal would need the user's disposition, not implicit bundled-font-only scope.
- COLRv1 static/variable and JuliaMono's SVG-bearing glyph render monochrome base outlines, not colour paints/SVG. Parser acceptance alone is not colour support.
- PCF/OTB and Terminus's size-16 bitmap strike render generic rectangles, not their mapped glyphs; Terminus's size-17 outline works. The native CBDT emoji strike likewise renders a rectangle; ordinary size 16 fails cleanly. A nonzero mapping, successful metrics and successful `draw_text` do not prove the intended glyph is drawn.
- The current bitmap path requests `BITMAP_METRICS_ONLY` while trying to rasterize, leaving several bitmap payloads absent. The BDF grayscale/subpixel path also treats packed mono bytes as per-pixel bytes and can overread the buffer. Further unsafe captures were censored; no affected image or production fix is published. The final BDF capture uses safe mono AA.
- Standalone COLRv0, sbix, SVG-only rendering, other bitmap depths and remaining legacy/wrapper formats are explicitly untested or unavailable. No exclusion is approved.

Two complete final collections are byte-identical: 420 assertions per collection, 64 RGB surfaces independently hash/count-checked, 20 proven mapped targets and 40 proven glyph-0 controls. All cases finish within containment; four recorded load failures are unsupported-size errors, not parser failures. The source/binary identities, exact file hashes, source explanations and reproduction commands are in the report. Independent review found no blockers.

No candidate was executed or selected, no compatibility loss was approved, and production code, dependencies and existing goldens are untouched. [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md) now owns candidate results and the user's required-support / approved-exclusion / blocker decisions for these concrete cases.
