# Fonts

Heavy XS uses Skrifa for font parsing, metrics and hinting, Zeno for grayscale and
subpixel rendering, and tiny-skia for monochrome rendering. The backend choice
and accepted differences from FreeType are recorded in [ADR 0001](https://github.com/juniorsundar/heavy-xs-architecture/blob/main/adr/0001-rust-native-font-backend.md).

## Supported formats

TrueType/glyf, OpenType/CFF and CFF2 fonts are supported, including font collections.
Collections use face 0; variable fonts use their default instance. There is no
face-index or variation-axis selection API.

Type 1/PFB, BDF, PCF and EBDT/CBDT bitmap fonts are not supported. Loading them
fails with the `failed to load font:` error prefix.

## Styles and real bold faces

The `bold` option applies synthetic bold with a stroke width of `pixel_size / 20`,
so its strength scales with the font size. Italic applies a 0.25 shear; smoothing
adds a half-pixel stroke. Bold and smoothing are approximations, not FreeType's
outline-emboldening algorithm.

To use a font's designed bold glyphs instead, load its actual Bold face with
`renderer.font.load` and assign it to the relevant font setting. For example, in
`USERDIR/init.lua`:

```lua
local style = require "core.style"
style.syntax_fonts["keyword"] = renderer.font.load(
  "/path/to/YourFont-Bold.ttf", 15 * SCALE)
```

Do not add `{ bold = true }` unless you also want synthetic bold on that face.
`style.syntax_fonts` selects fonts by syntax token; `style.font` and
`style.code_font` can likewise be assigned loaded faces.

## Antialiasing and hinting

`antialiasing = "none"` uses tiny-skia's non-antialiased outline fill, rather than
thresholding a grayscale mask. Very thin strokes can disappear in unhinted mono
fonts. Use `hinting = "slight"` or `"full"`, or enable grayscale antialiasing, if
this affects your font.

Hinting `"none"` leaves outlines unhinted. With antialiasing enabled, `"slight"`
uses light autohinting and `"full"` uses normal autohinting; mono uses a dedicated
mono autohinting target for either hinted setting.

Subpixel rendering uses Zeno's RGB coverage, not FreeType's LCD filter. Subpixel
coverage and the smoothing stroke are approximations and can look different from
the previous backend. Font advances, line heights, fallback and tab rules are
unchanged.
