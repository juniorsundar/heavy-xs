# SDL expectations

Behaviour observed from SDL itself, recorded so that SDL-free replacements can be checked
against it after SDL is gone. These are reference observations, not goldens produced by the
editor. `scripts/rust-port/sdl-reference.sh probe` re-observes them and fails on any
difference; with `RECORD=1` it leaves the new observation in `target/sdl3-reference/evidence/record`
for review. Replacing a file here is a reviewed, manual step.

| File | What it pins | Observed with |
| --- | --- | --- |
| `draw.txt` | `ren_draw_rect`'s SDL operations: opaque fill and the 1x1 RGBA32 linear scaled blend (single and repeated), clipping of negative, empty and out-of-range rects, and channel packing of the formats in play (`fixtures/sdl_reference_probe.c draw`). | SDL 3.4.14 runtime, window surface format ARGB8888; identical on the offscreen driver and on the runner's private Xvfb (x11). |
| `clicks.txt` | SDL's multi-click count for real X button events: a 500 ms window (2 at 490 ms, 1 at 510 ms), a ±32 px box on each axis measured from the previous click, no cap on the count, separate state per button (`fixtures/sdl_reference_probe.c clicks`, XTest). | SDL 3.4.14 runtime, x11 driver on the private Xvfb; the default double-click time and radius (no XSETTINGS, no hints). |
| `hit-test-grid.txt` | Line count and sha256 of the original C `hit_test()` grid (`hit-test-check.sh`). | `fixtures/hit_test_reference.h` compiled against the SDL 3.4.14 headers. |

Recorded on 2026-10-21 from the reference build described in
`target/sdl3-reference/evidence/reference.txt` (source `c80459bc0f464df68b8d8cdff553cd8288519a67`).
