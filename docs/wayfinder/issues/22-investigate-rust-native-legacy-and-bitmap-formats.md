# Investigate Rust-native paths for legacy and bitmap font formats

Parent: ../freetype-map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-font-format-research
Blocked by: 18

## Question

Which existing Rust-native APIs can load, measure and draw the formats that the [format reference](../assets/font-format-reference.md) shows working under FreeType today but which have no candidate path, and how much glue does each need?

- **BDF** (works today, mono only) and **PCF**: is there a maintained Rust-native parser producing glyph bitmaps and metrics?
- **Type 1 / PFB** (works today): read-fonts 0.45.1 contains `ps::type1` and `model::font::Format::Type1`, but Skrifa 0.48.0 `OutlineGlyphFormat` has only Glyf/Cff/Cff2/Varc. Is a Type 1 outline/metrics path exposed by released fontations crates, and through which API?
- **EBDT/EBLC and CBDT/CBLC strikes** (placeholders today): Skrifa `bitmap::BitmapStrikes` reads them. What does it return (mono/gray/PNG data, metrics), and what decode/scale steps would remain?

Use versioned primary sources (released crate docs/source, tags). Record crate versions, licences, declared native dependencies and maintenance state. Distinguish "parser exists" from "metrics plus drawable glyph image available".

No code, prototype, dependency adoption or format disposition. The user decides required support / approved exclusion / blocker in [Select the Rust-native font backend](16-select-rust-font-backend.md) using this evidence. Exclude C/C++ bindings, forks and custom parsers.

## Comments

## Resolution

Research only. [Report](../assets/font-format-rust-paths.md), published on branch `research/font-format-rust-paths` at commit `f0d8b13f` (no PR, no merge). Type 1 and bitmap-strike API claims were spot-checked against the released crate sources.

| Format | Existing Rust-native path | Glue |
|---|---|---|
| Type 1 / PFB | released read-fonts 0.45.1 `ps::type1::Type1Font`: PFA/PFB, `draw(gid, ppem, OutlinePen)`, `advance`, `extents`, byte `encoding`; Unicode charmap needs the `agl` feature. Standalone, not a Skrifa `OutlineGlyphFormat`, so a parallel load path is needed. | medium |
| BDF | released parsers with bitmap + advance + bbox: `bdf-parser` 0.1.0 (MIT/Apache-2.0), `bdf-reader` 0.1.3 (EPL-2.0), `bdf2` 0.7.1 (WTFPL, weak maintenance signal) | small |
| X11 PCF | no maintained released Rust-native parser found | large |
| EBDT/EBLC, CBDT/CBLC | Skrifa 0.48.0 `bitmap::BitmapStrikes`: `Mask` (1/2/4/8 bpp, built-in unpack) and `Bgra` drawable directly; `Png` needs a decoder (`png` 0.18.1, pure Rust) and fixed strikes need scaling | small (mask/BGRA), medium (PNG) |

Unknowns: BDF crates' maintenance and transitive dependencies, BDF encoding edge cases, `.pcf.gz`, `FontRef` on outline-less fonts, Type 1 seac/multiple-master correctness versus FreeType.

No format disposition, dependency adoption or code is approved. The user decides required support / approved exclusion / blocker in [Select the Rust-native font backend](16-select-rust-font-backend.md). PCF and the bitmap strikes are already broken under FreeType today, so dropping them would not regress current behavior; BDF and Type 1 would.
