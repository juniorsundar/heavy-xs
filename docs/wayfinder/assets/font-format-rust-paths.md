# Lite XL Rust port: FreeType-format replacement research

Candidate stack: Skrifa 0.48.0 + read-fonts 0.45.1 + Zeno 0.3.3 + tiny-skia 0.12.0.
Primary evidence: local cargo-registry copies of skrifa-0.48.0 and read-fonts-0.45.1,
cross-checked against versioned docs.rs pages and upstream tags below.

## Summary

| Format | Verdict (released crates only) | Glue |
|---|---|---|
| Type 1 / PFB | drawable: `read_fonts::ps::type1::Type1Font` in released read-fonts 0.45.1 (PFA+PFB, charstring decode, advances, extents, encoding/charmap); **not** wired into skrifa outlines | medium |
| BDF | drawable: `bdf-parser` 0.1.0, `bdf2` 0.7.1, `bdf-reader` 0.1.3 (bitmap + advance + bbox each) | small |
| PCF (X11) | no maintained, released Rust-native parser found | large |
| EBDT/EBLC, CBDT/CBLC | drawable via skrifa 0.48.0 `bitmap::BitmapStrikes`/`BitmapGlyph` (BGRA direct, Mask unpack builtin, PNG bytes need decoder) | small (BGRA/Mask), medium (PNG) |

## 1. Type 1 / PFB

- Released API exists: [`Type1Font`](https://docs.rs/read-fonts/0.45.1/read_fonts/ps/type1/struct.Type1Font.html)
  in `read_fonts::ps::type1`, present in released
  [read-fonts-v0.45.1](https://github.com/googlefonts/fontations/releases/tag/read-fonts-v0.45.1)
  (tag 15 Oct 2026) and `pub mod ps` in the released crate.
- PFB handled: parser decodes PFB segment tags (`0x8001`/`0x8002`), then eexec-decrypts;
  PFA hex/binary eexec likewise. `Format::Type1` sniffing covers `%!PS-AdobeFont`/`%!FontType`
  at offset 0 and 6 (PFB header); see
  [`model::Format`](https://docs.rs/read-fonts/0.45.1/read_fonts/model/enum.Format.html).
- Drawable + metrics, no feature gate on the type itself: `Type1Font::new(&[u8])`,
  `draw(gid, ppem, &mut impl OutlinePen) -> Option<f32>` (advance),
  `advance()`, `extents()`/`evaluate_extents()`, `evaluate_width()`/`evaluate_charstring()`,
  `glyph_names()`, `Encoding::map(u8)->GlyphId` (+ predefined/custom). Only
  `unicode_charmap()` needs the non-default `agl` feature (empty map otherwise);
  `default = ["std"]`, Type 1 core ungated, no `experimental_*` gate.
- Not integrated with skrifa: `OutlineGlyphFormat` has only Glyf/Cff/Cff2/Varc
  ([enum](https://docs.rs/skrifa/0.48.0/skrifa/outline/enum.OutlineGlyphFormat.html));
  `Type1Font` is standalone (built from file bytes, not `FontRef`), so the
  `charmap -> OutlineGlyphCollection -> draw` path needs a parallel branch.
- Glue = medium: outline decode, transform, advance, bbox and byte-encoding map all
  exist; remaining work is pipeline wiring (size/orientation conventions, seacBehavior check,
  MM `weight_vector` blending unverified).

## 2. BDF and PCF

- [`bdf-parser` 0.1.0](https://docs.rs/crate/bdf-parser/0.1.0) (MIT OR Apache-2.0;
  deps bstr, nom 6, strum, thiserror — pure Rust, no C):
  [`BdfFont{metadata, glyphs, properties}`](https://docs.rs/bdf-parser/0.1.0/bdf_parser/struct.BdfFont.html),
  [`Glyph{name, encoding: Option<char>, device_width, bounding_box, bitmap: Vec<u8>, pixel()}`](https://docs.rs/bdf-parser/0.1.0/bdf_parser/struct.Glyph.html).
  Parser + metrics + drawable bitmap in one crate. Text BDF only.
- [`bdf2` 0.7.1](https://docs.rs/crate/bdf2/0.7.1) (licence WTFPL per released Cargo.toml;
  read + write): `Font` + [`Glyph`](https://docs.rs/bdf2/0.7.1/bdf2/struct.Glyph.html)
  (name, codepoint, device/alternate widths, bounds, `map: &[Bitmap]`, `pixels()`,
  `width()/height()/get()` via `Bitmap`). Upstream has
  [no GitHub releases](https://github.com/meh/rust-bdf/tags); maintenance signal weak.
- [`bdf-reader` 0.1.3](https://docs.rs/crate/bdf-reader/0.1.3) (EPL-2.0 weak copyleft —
  licence review advised): `Font::read(BufReader)` +
  [`Glyph{name, encoding: u32, swidth, dwidth, bounding_box, bitmap: Bitmap}`](https://docs.rs/bdf-reader/0.1.3/bdf_reader/struct.Glyph.html).
  Parser + metrics + drawable in one crate.
- X11 PCF (Portable Compiled Format): no maintained released Rust-native crate found.
  `simple-pcf` is the *other* PCF (kernel PC Screen Font), not X11
  ([docs](https://docs.rs/simple-pcf/latest/simple_pcf/)); `ppcf` is
  [C](https://github.com/ArcherPergande/ppcf) (excluded); Kaitai has only a
  [grammar](https://formats.kaitai.io/pcf_font/) (generator input, not a crate);
  fontpixel-rs claims BDF+PCF but has [no crates.io release](https://docs.rs/crate/fontpixel/latest).
  `.pcf.gz` likewise has no verified Rust handler (would be parser + flate2).
- Glue: BDF = small (all three yield blittable bitmap + advance + bbox; only
  codeset mapping and point-size scaling remain). X11 PCF = large (parser missing).

## 3. EBDT/EBLC and CBDT/CBLC (incl. OTB, embedded strikes, Noto Emoji)

- skrifa 0.48.0
  [`BitmapStrikes`](https://docs.rs/skrifa/0.48.0/skrifa/bitmap/struct.BitmapStrikes.html):
  `new` (prefers sbix, then CBDT, then EBDT), `with_format`, `get`, and
  `glyph_for_size` (exact, else nearest-larger, else nearest-smaller strike).
  `BitmapStrike::ppem()` + `get(glyph_id)`; `BitmapGlyph` carries bearings,
  inner bearings, `ppem_x/y`, `advance: Option<f32>`, `width/height`, and
  `placement_origin` (TopLeft for BDT, BottomLeft for sbix).
- [`BitmapData`](https://docs.rs/skrifa/0.48.0/skrifa/bitmap/enum.BitmapData.html):
  `Bgra(&[u8])` (premultiplied sRGB — directly drawable), `Png(&[u8])` (raw bytes —
  external decoder required), `Mask(MaskData{bpp 1/2/4/8, is_packed, data})` with
  builtin `decode`/`decode_to_slice` to 8bpp (mono/bit-packed unpacking done, no extra dep).
  bpp outside {1,2,4,8,32} yields `None`; sbix accepts only `png ` graphicType.
- PNG glue: [`png` 0.18.1](https://docs.rs/crate/png/0.18.1) is pure Rust
  (MIT OR Apache-2.0; deps bitflags, crc32fast, fdeflate, flate2, miniz_oxide — no C),
  but is not currently a direct dependency, so CBDT/sbix-PNG needs one new
  dependency plus premult/sRGB handling; fixed-ppem strikes need resampling for
  non-native sizes. Glue: small for Bgra/Mask, medium for Png.
- OTB/embedded-EBDT flow through the same `Ebdt` path (only `eblc`+`ebdt` tables
  required); CBDT/CBLC likewise. No PNG/mono unpacking code needs writing.

## Sources (kept)

- Local registry copies skrifa-0.48.0, read-fonts-0.45.1 (primary: bitmap.rs, outline/mod.rs, ps/type1.rs, model/font/format.rs, lib.rs features).
- docs.rs (exact versions): read-fonts 0.45.1 Type1Font; skrifa 0.48.0 bitmap/outline; bdf-parser 0.1.0 (+Cargo.toml); bdf2 0.7.1 (+Cargo.toml, Glyph, Font); bdf-reader 0.1.3 (+Glyph); simple-pcf; png 0.18.1 (+Cargo.toml).
- Upstream tags: [fontations tags](https://github.com/googlefonts/fontations/tags) (read-fonts-v0.45.1, 15 Oct 2026).
- Dropped: `ppcf` (C, violates no-C rule); Kaitai PCF page (grammar, not a crate);
  fontpixel-rs (toolkit claim, no crates.io release); lib.rs crate pages (403).

## Unknowns

- crates.io release dates/download counts for the BDF crates (API blocked them).
- Transitive deps of `bdf2`/`bdf-reader` (Cargo.toml dep sections not fully retrieved).
- BDF edge cases: `ENCODING -1`, non-Unicode charsets, vertical DWIDTH, `.pcf.gz`.
- Whether `FontRef::new` accepts OTB/CBDT-only fonts lacking `glyf` (expected yes, unverified).
- sbix non-`png ` graphic types; Type1 seac-correctness vs FreeType; Type1 MM blending.
