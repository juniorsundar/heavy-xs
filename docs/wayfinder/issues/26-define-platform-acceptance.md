# Define platform compatibility and acceptance criteria

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-sdl3-acceptance
Blocked by: 24, 25

## Question

With the contract inventory and the crate survey available, which platform behaviours must stay identical, which differences may be reviewed, which gaps would be acceptable to list, and what checks must a replacement pass on Wayland and X11?

Discuss concrete scenarios with the user: fractional and integer scale, IME composition, key names on non-US layouts, borderless dragging and resizing, clipboard and primary selection across apps, file dialogs inside and outside sandboxes, drag and drop, focus and minimise/maximise events, and startup and idle cost. Decide how the test suites keep proving behaviour once SDL's test drivers are gone.

Context: [Inventory the observable platform contract](24-inventory-platform-contract.md), [Survey Rust-native platform crates](25-survey-rust-platform-crates.md).

## Comments

### Resolution

The user answered **“yes to all”** to the five acceptance recommendations below. These are requirements, not evidence that a candidate passes them. No stack, compatibility loss or production change is approved by this resolution.

1. **Preserve the observable contract, including quirks.** Preserve the Lua API, return values, errors, event arguments and ordering, and coordinate conversions. In particular, preserve focus-gained discarding queued key presses, opacity reporting success, clipboard setters hiding failures, and touch using different coordinate units from mouse input. Any correction is a separately listed change requiring user approval; replacing SDL must not silently fix behaviour.
2. **Drawing stays unchanged.** Require identical rendered pixels for deterministic scenes at the same surface density, unchanged text geometry, and correct mouse-to-cursor alignment. Replacing pixel storage and presentation is not a rasterizer change. Explain every mismatch and obtain approval; regenerating expected images is not acceptance.
3. **Real desktop checks are required on Wayland and X11.** Check resize, maximise, minimise, restore, fullscreen, borderless dragging/resizing, keyboard shortcuts, a non-US layout/dead keys, IME composition and candidate-popup placement near the caret, clipboard and primary selection between applications, file-manager drops, and open/save/directory dialogs including cancellation. Require 1× and genuine fractional scaling such as 1.25×, including pointer alignment and movement between differently scaled outputs where available. Unavailable checks stay explicitly pending: XWayland demonstrates X11-client behaviour under that compositor, not a native X11-session test. Touch hardware and sandbox-specific checks remain pending unless the user explicitly accepts a narrower gate. No unavailable check is implicitly waived.
4. **Keep automated and live integration evidence separate.** Require deterministic checks of event translation, ordering, coordinate conversions, worker wake-ups, drawing and Lua results, plus live checks of the actual desktop path. A replacement injection queue alone cannot prove real OS events are translated correctly. Retain all 16 contract suites and hit-test/startup checks. Baseline changes need an explained, approved difference. Rerun and classify the existing `process` timing failure before claiming an all-passing run; do not assume it is flaky or unrelated without investigation.
5. **Measure costs against SDL under matched conditions.** Collect paired measurements of cold startup, scrolling/typing frame work, idle CPU and wake-ups, and memory while opening/resizing/closing windows. Report presentation time separately from active work. Require no unexplained material regression, accumulating memory or idle busy-spin. Measure SDL's baseline and agree numerical tolerances with the user before judging candidate results; no numerical budget or measured performance result is approved here.

The [window, scale and IME prototype](29-prototype-window-scale-ime.md) and [loop, clipboard and dialogs prototype](30-prototype-loop-and-dialogs.md) supply bounded feasibility evidence, not proof that the whole editor passes these gates. The [platform cost calibration](31-calibrate-platform-cost-gates.md) supplies the still-unset numerical tolerances. The [implementation handoff](28-plan-sdl3-migration.md) must carry every gate and pending check into the final editor validation plan.
