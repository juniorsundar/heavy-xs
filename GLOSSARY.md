# Editor terminology

Terms used when discussing editor rendering and Lua-facing GUI customisation.

## Language

### Lua-facing GUI

**Legacy Lite XL contract**:
The existing Lua-facing interfaces and observable behaviours carried forward from Lite XL. Its GUI portion includes the extension interactions identified as part of that contract, not just native functions.
_Avoid_: Native ABI when referring to Lua GUI compatibility

**GUI compatibility reference**:
The pinned pre-redesign editor snapshot and recorded plugin/configuration environments used to distinguish existing behaviour from redesign regressions.
_Avoid_: Latest upstream when referring to the fixed comparison reference

**GUI performance qualification**:
A claim that a GUI candidate satisfies the agreed startup and interaction criteria for explicitly identified workloads and environments, supported by the required evidence. It is distinct from permission to explore a design or a claim of legacy compatibility.
_Avoid_: Performance proven when required checks remain unverified

**Experimental GUI exposure**:
Newly exposed GUI internals offered without a stability promise, distinct from the existing Lua-facing exposure in the GUI compatibility reference.
_Avoid_: Legacy contract when referring to reversible experimental additions

**GUI customisation**:
Changes to the editor's appearance, arrangement and interaction, including adding views or replacing how views, layout and input behave. Depth of customisation and ease of making a change are separate properties.
_Avoid_: Theming when referring to behavioural changes

### Font rendering and presentation

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

**Coverage atlas**:
Cached glyph samples describing how much of each color channel a glyph covers. These are coverage values, not the finished text colors or the window image.
_Avoid_: Glyph image when implying finished colors

**Draw target**:
The editor's retained, composed window image, including pixels in regions not redrawn this frame. It is distinct from the image storage used to present a frame to the desktop.
_Avoid_: Surface when its role is ambiguous

**Presentation buffer**:
Image storage handed to the desktop to display a frame. It is distinct from the editor's retained draw target and may not contain the immediately preceding frame.
_Avoid_: Framebuffer when implying the retained draw target
