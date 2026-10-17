# Compare candidate font controls and rendering

Parent: ../freetype-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: open
Assignee: none
Blocked by: 14, 15, 19

## Question

Can an existing Rust-native library or small composition provide the agreed font controls and internally consistent rendering/measurement, and which measured differences does the user accept after seeing the comparison?

Use the prototype skill and agree a bounded experiment with the human before building a disposable harness. Apply [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) to the [Survey Rust-native font backends](13-survey-rust-font-backends.md), then test the viable candidate paths against [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md). Direct Skrifa + Zeno and stock Swash have different control/plumbing trade-offs; Fontdue's unhinted/subpixel path is a possible comparison only if the agreed criteria make it relevant. This ticket does not preselect a backend or require adopting all surveyed libraries.

Record the exact versions/features and demonstrate the real routing of hinting none/slight/full and AA mono/grayscale/subpixel. Stock Swash's fixed target/AutoFallback policy is not the editor's FORCE_AUTOHINT policy; Zeno Alpha is not native mono; Fontdue's RGB sampling is not evidence of the current LCD filter. Missing controls, thresholding or alternate algorithms must be explicit proposed differences, not silent aliases.

Use the baseline's chosen bundled/custom fonts and scenes to compare metrics and glyph placement together: three fractional phases, actual ASCII-mapped icons, genuine cross-font fallback versus .notdef, tabs, clipping, cursor alignment and the agreed formats. Show X-only bold, smoothing, italic and editor-owned decorations separately. Include bounded cold/warm, malformed-input and retention evidence using the agreed criteria; do not invent thresholds from the regex effort.

Present concrete images, measurements, reproduction instructions and limitations for the user's actual review. Automatic checks or an agent's opinion cannot substitute for the human's appearance/compatibility answers. Link the artifact and record accepted/rejected differences precisely; unresolved conflicts must block selection.

No production backend patch, dependency adoption, fork, custom parser/rasterizer, shaping/fallback redesign or hidden C fallback. If the necessary adapter exceeds a small existing-library composition, stop and ask rather than implementing an engine inside the prototype.

This evidence is a prerequisite for [Select the Rust-native font backend](16-select-rust-font-backend.md); it does not itself authorize production implementation.

## Comments

Created from the completed library survey: candidate controls, mono/LCD behavior and synthetic-style equivalence are now precise enough to test. The baseline and live acceptance discussion remain prerequisites, and no candidate prototype or user approval has occurred yet.

Now also blocked by [Classify font memory retention in the current renderer](19-classify-font-memory-retention.md). Candidate retention cannot be interpreted until it is known whether the baseline's RSS growth comes from FreeType-specific or shared renderer code.
