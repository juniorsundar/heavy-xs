# Prototype window, scale and IME on Wayland and X11

Parent: ../sdl3-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: pi-sdl3-window-prototype
Blocked by: 24, 26

## Question

Apply [the agreed acceptance criteria](26-define-platform-acceptance.md#resolution). Record unavailable checks as pending, distinguish XWayland from a native X11 session, and request explicit approval for any proposed difference. This standalone prototype provides feasibility evidence, not full-editor acceptance.

On the user's KDE Wayland desktop at fractional scale 1.25, and in an X11 session, does a minimal winit 0.30.13 + softbuffer 0.4.8 window give crisp CPU-drawn pixels, usable IME composition, and acceptable borderless moving and resizing?

A standalone throwaway binary, not the editor: log the scale factor, `inner_size` and scale changes; draw a 1 px grid sized from physical pixels; follow a caret with the IME cursor area and type CJK and dead keys; implement edge and corner zones with `drag_window`/`drag_resize_window`; try icon, cursors, file drop and touch where available; confirm opacity is absent. Then run the native-dependency audit (`cargo tree`, `ldd`, a binary scan for SDL) on the exact feature set. The user judges what they see.

Context: [Survey Rust-native platform crates](25-survey-rust-platform-crates.md) (experiments E1–E3, E6, X1–X3, X5).

## Comments

### Progress — runnable probe; human review pending

A standalone native probe, separate from the editor, was built for the user to run on native Wayland (borderless) and as an X11 client, distinguishing XWayland from a native X11 session.

The CPU-drawn window shows one-physical-pixel grid lines, scale and physical/logical dimensions, a preedit/committed-text field and candidate rectangle, plus click-triggered drag/resize regions. It logs keys/modifiers, focus, file-drop basenames and touch. Its small text helper reuses Skrifa and Zeno; this is not an editor widget or shaping implementation. Logs can contain the typed test text and filenames; use nonprivate samples.

Validation: debug build, format/shell syntax checks, five unit tests and decorated/borderless launcher smoke checks passed under a private Xvfb server at scale 1, 800×520. Missing-backend and unavailable-Wayland startup checks failed cleanly. Independent light review passed; a missing-glyph measurement/drawing discrepancy was fixed with a unit test. No user desktop window was launched and no runtime result is inferred for Wayland, native X11, fractional scaling or IME.

Dependency evidence: 112 deduplicated Linux normal/build dependency rows, no SDL package; no dynamic `SDL_` symbols or SDL loads in the Xvfb traces. `ldd` lists only libgcc_s, libm, libc and the Linux loader. `wayland-backend` brings the Rust `cc` build helper but its C logging shim's `log` feature is disabled. The existing development shell still includes unused SDL; final editor dependency removal is not proved. The prototype README records the exact lockfile/executable identities and limitations.

**Still pending:** human Wayland/native-X11 review, 1× and fractional display checks, real IME/dead-key composition and popup position, borderless move/resize, cursors/icon, real file drop and available touch hardware. Xvfb is only a startup/presentation check; unavailable hardware/session checks are not waived. The ticket stays claimed, no stack or feature loss is accepted, and no completed-decision entry is added to the map yet.

### Progress — first live run failed; experimental display workaround

The user reported a black window. Native Wayland captures reproduced a uniformly transparent-black surface despite successful presents and nonblack CPU buffer samples; the protocol trace showed XRGB8888 buffers attached and committed. An initial capture was of KRunner and was discarded, not used as evidence. A correct capture/check went red with one colour at 2543×2076, scale 1.20.

Changing only the unused XRGB high byte to `0xff` restored the visible grid, labels and text field. Disabling IME alone did not restore pixels, so the independent IME redraw storm was not the blank-window cause. The probe now applies opaque padding on Wayland only. **This differs from softbuffer's documented zero padding and is an experimental compatibility workaround, not a production-approved fix or proof of which lower-stack component is responsible.** The discrepancy must be explained or an explicitly accepted supported solution found before qualifying this stack.

Separately, repeated cursor-area updates caused empty-preedit notifications and repeated redraws. Caching the last area (including scale and reset on IME enable/disable), and suppressing redraws for unchanged preedit, reduced a six-second idle reproduction from 179 frames to 2. This is diagnostic evidence, not acceptance of calibrated performance.

After the fixes, seven unit tests, format/shell checks, both Xvfb smoke modes and independent light review pass. The new `check-visible.sh` captures the active probe only with the desktop idle, checks capture dimensions and green-strip RGB against the presented window, and rejects a blank frame or idle redraw storm. A final native Wayland capture at the actual scale 1.25 (1261×754) passed with two frames in six seconds; the image reader confirmed it shows the probe, not KRunner, and no blank region.

### User feedback — corrected Wayland probe

After reviewing the corrected window, the user reported: “Ok it works well.” On explicit follow-up, the user confirmed all four checks: grid/appearance, borderless dragging/resizing, ordinary typing without missing/duplicated characters, and real IME composition with its candidate popup following the caret. These checks pass the user's review of the corrected Wayland prototype at the tested fractional scale; they are not full-editor acceptance.

Native X11-session review, 1× display review, non-US dead keys, movement between differently scaled outputs, cursors/icon, real file drop and available touch hardware remain pending unless separately reported. The opaque-padding compatibility caveat is unchanged; this feedback does not approve a production workaround, hardware waiver or stack selection. The ticket remains claimed.

### XWayland observation — IME indicator differs on focus loss

The user observed that clicking outside the native Wayland window changes its `IME: true` label to false, whereas the X11 window under XWayland keeps the label true. Inspection of the pinned winit 0.30.13 source explains the distinction: the probe mirrors `Ime::Enabled`/`Disabled`, not `WindowEvent::Focused`; Wayland text-input `Leave` emits `Ime::Disabled`, while X11 focus loss calls `XUnsetICFocus` without emitting `Ime::Disabled`. The X11 context can remain enabled while unfocused. These events must not be interpreted as a cross-backend window-focus indicator.

This is a reported observation plus a static-source explanation, not a runtime trace or proof of functional XWayland IME. No code was changed to force the indicators to match. On follow-up about composition and candidate-popup placement after refocusing the XWayland window, the user answered yes and reported editing behaves the same on both paths. This records successful user-reviewed composition/refocus behaviour under XWayland, not a native X11 session.

The user also observed that Left Arrow does not move the caret on either backend, whereas Backspace deletes characters. Static inspection confirms the standalone field implements Backspace but no ArrowLeft/ArrowRight handling; clicking the field places its caret at the end. This is a limitation of the probe's minimal editing widget, not evidence that either backend loses arrow-key events. No claim is made that arrow-key delivery or full-editor cursor navigation has passed, and no editor behaviour is changed.

### Resolution

The user explicitly chose **“Close with blocking follow-ups”**: record successful bounded feasibility, while preserving unsupported presentation handling and incomplete desktop coverage as blockers for stack selection. This resolution supersedes the earlier progress notes saying the ticket stays claimed; it does not mark every original check passed.

- **User-reviewed success:** the corrected native Wayland probe at fractional scale 1.25 passed appearance/grid, borderless drag/resize, ordinary typing, and IME composition/candidate-popup placement. Under XWayland, the user confirmed composition, popup placement after refocusing and comparable editing behaviour. This is X11-client evidence under a Wayland compositor, not native X11-session acceptance.
- **Automated evidence:** seven unit tests, format/shell checks, decorated/borderless Xvfb smoke checks, the native visible-pixel/idle regression check and independent review passed. The six-second idle check showed two drawn frames, not a calibrated editor performance pass. Dependency-audit limits remain as recorded above.
- **Source:** a standalone probe outside the editor, with a visible-pixel check; no editor code changed.
- **Presentation compatibility remains open:** [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md) owns explaining/replacing the opaque-padding workaround with a supported, demonstrated route or an explicitly approved alternative disposition. Closing this ticket does not approve the workaround for production.
- **Desktop coverage remains open:** [Complete the outstanding window and input desktop checks](33-complete-platform-desktop-checks.md) owns native X11, real 1× and the remaining unreported checks and runtime audit. Missing checks are pending, not passed or waived; the minimal field's absent arrow navigation is not an editor regression or an arrow-event delivery result.

Both successors block [Select the Rust-native platform stack](27-select-platform-stack.md). The destination, agreed acceptance criteria and planning-only scope are unchanged; this resolution selects no stack, permits no feature loss and authorizes no production implementation.
