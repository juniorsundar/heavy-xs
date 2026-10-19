# Editor font rendering

Terms used to distinguish text layout from raster appearance during font-backend comparisons.

## Language

**Text geometry**:
The advances, baselines and line dimensions that determine where text, cursors and selections are placed. Different glyph pixels do not necessarily imply different text geometry.
_Avoid_: Pixel parity when referring to layout

**Surface density**:
The number of physical surface pixels per logical drawing unit. Increasing a font's logical size is not an increase in surface density.
_Avoid_: HiDPI when referring only to enlarged text

**Synthetic bold**:
A bolder appearance produced by thickening the glyphs of a regular font when the bold style option is set. It is not a separate font.
_Avoid_: Bold face, when the font file itself is regular

**Bold face**:
A font file designed as the bold member of a family, loaded explicitly like any other font.
_Avoid_: Bold option, synthetic bold
