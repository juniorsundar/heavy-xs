---
status: accepted
---

# Replace FreeType with Skrifa + Zeno + tiny-skia

The editor's font rasteriser moves from C FreeType (via `freetype-rs`) to three pinned pure-Rust crates: **Skrifa 0.48.0** parses fonts, measures and autohints; **Zeno 0.3.3** draws grayscale and subpixel glyphs; **tiny-skia 0.12.0** draws mono (antialiasing off) with its non-antialiased fill. Only the FreeType calls in `src/renderer.rs` change. The glyph cache, fallback, tabs, blending, decorations and Lua font API stay as they are, so text geometry (advances, heights, cursor and selection placement) is unchanged. The user reviewed the result live and accepted its appearance and performance. Selection is recorded in [Select the Rust-native font backend](../wayfinder/issues/16-select-rust-font-backend.md#resolution).

## Considered Options

- **Stock Swash:** rejected; its boolean hint switch and fixed fallback policy cannot route hinting none/slight/full and mono separately.
- **fontdue, ab_glyph:** rejected; no hinting.
- **Mono by thresholding Zeno coverage at 50%:** rejected; it erased thin strokes (Noto CJK 中: 6 lit pixels vs FreeType's 42).
- **Mono by keeping any coverage:** rejected; strokes too heavy.
- **Keep FreeType for mono only:** rejected; not Rust-native.
- **Real bold faces only (no synthetic bold):** rejected; it would break the `bold` font option that plugins and configs use.

## Consequences

Deliberate differences a reader might try to "fix":

- **Synthetic bold is size-scaled** (`pixel_size / 20`), not FreeType's fixed half pixel, which was invisible. Smoothing is a stroke approximation.
- **Unhinted mono can lose strokes in very thin fonts.** This is accepted; hinted mono keeps them.
- **Subpixel rendering is Zeno's**, not FreeType's five-tap LCD filter.
- **Type 1/PFB, BDF, PCF and EBDT/CBDT bitmap fonts are not supported.** They fail with `failed to load font:`. PCF and bitmap strikes already failed under FreeType; Type 1 and BDF did not.
- Some corrupted fonts are accepted or rejected differently than under FreeType, always without crashing.
- The normal-density visual review was waived by the user; only HiDPI appearance was reviewed.
