# Replace SDL3 with Rust-native platform crates

Labels: wayfinder:map
Status: resolved

## Destination

Choose Rust-native replacements for the editor's SDL3 uses, whole or piecemeal, and produce a concrete incremental migration and validation plan, with the Lua-visible platform behaviour preserved and every difference or remaining SDL use explicitly reviewed by the user. Implementation is a separate handoff.

## Notes

- Planning only: inventory, research and bounded prototypes support decisions; no production change is authorized by this map.
- "Replaced" means SDL3 is gone from the build and the binary. Loading the desktop's own system libraries (libwayland-client, libxkbcommon, libX11 and the like, linked or dlopened) is allowed; writing a Wayland or X11 client from scratch is not.
- Partial replacement is a valid outcome, provided every SDL area that stays is listed with a reason.
- Support both Wayland and X11, as SDL does today. Dropping X11 needs its own explicit decision.
- Present on the CPU only: the editor keeps drawing every pixel itself and hands a finished frame to the window. GPU rendering (e.g. wgpu) is a separate effort.
- The `sdl_renderer` build feature is retired rather than ported; its fractional-scale startup crash goes with it.
- Freeze the Lua-visible behaviour: every `system.*` and `renwindow.*` function and every event name and argument from `system.poll_event`. Acceptance is the contract suites plus a real-window review on Wayland and X11, including fractional scaling and IME, subject to the user-approved validation dispositions in [Complete the outstanding window and input desktop checks](issues/33-complete-platform-desktop-checks.md#resolution). Test-only hooks may be adapted; changing an expected image requires an explained difference approved under the acceptance criteria.
- No feature is dropped by default. Any remaining crate capability gap becomes an explicit user decision, not a silent reduction. The [selected stack](issues/27-select-platform-stack.md#resolution) and subsequent resolutions record the approved exceptions; do not reopen whole-window opacity in this effort.
- Migration is incremental: separately shippable steps with SDL living alongside until the last. Which areas must move together (window, events and presentation probably do) is for this effort to confirm.
- Decided at charting, not ticketed: plugin loading moves to `libloading`; timing and sleep to `std`; directory listing and removal to `std::fs`, preserving `system.rmdir` also deleting files; SDL's allocator to Rust allocations; the font loader stops opening files through SDL only to reuse its error wording, keeping the same message text.
- SDL is also the editor's pixel store, not only its window: glyph atlases and draw targets are SDL surfaces, and test hooks push into SDL's event queue. Both are in scope.
- The native-plugin interface exposes no SDL, so it is unaffected; Lua and the native-plugin interface stay. Replacing libc bindings is not part of this effort.
- Use grilling and domain-modeling for live decisions; research for external facts; prototype for behaviour the user must judge; codebase-design when establishing the seam. Consult the [local tracker](README.md) for claims, dependencies and frontier queries.
- Reviewers use `opencode-go/deepseek-v4.1-flash` until the user changes that preference.

## Decisions so far

- [Inventory the observable platform contract](issues/24-inventory-platform-contract.md): 41 `system` and 4 `renwindow` functions and 26 events are recorded with units, quirks and coverage; nothing at scale other than 1, real IME, live dialogs or drag and drop has ever been observed.
- [Survey Rust-native platform crates](issues/25-survey-rust-platform-crates.md): winit + softbuffer + arboard + rfd/ashpd cover most SDL areas on paper; winit has no opacity setter or per-point hit-test, and fractional scale, IME, clipboard keep-alive and dialog wake-up are unverified at runtime. The survey itself selected no stack.
- [Define platform compatibility and acceptance criteria](issues/26-define-platform-acceptance.md#resolution): the user approved the contract, drawing, desktop-validation, test and cost gates; unavailable checks remain pending and numerical cost tolerances require calibration.
- [Prototype window, scale and IME on Wayland and X11](issues/29-prototype-window-scale-ime.md#resolution): the user accepted the reviewed Wayland behaviour and XWayland composition/refocus evidence as successful feasibility; presentation and remaining desktop coverage were delegated to follow-ups, with no gate waived.
- [Prototype the editor loop, clipboard and dialogs](issues/30-prototype-loop-and-dialogs.md#resolution): an own poll/wait loop on winit, arboard clipboard/primary and both rfd and ashpd dialogs work on Wayland and XWayland; rfd needs runtime libdbus and ashpd a process-lifetime async runtime; native X11, sandbox and no-clipboard-manager checks moved to the desktop-checks task.
- [Calibrate platform performance and memory gates](issues/31-calibrate-platform-cost-gates.md#resolution): candidates are judged paired against SDL with one-sided limits set at 2–3× the measured Wayland noise, plus absolute limits for unfocused idle and memory accumulation; SDL itself fails the accumulation limit through its fullscreen surface path.
- [Complete the outstanding window and input desktop checks](issues/33-complete-platform-desktop-checks.md#resolution): Wayland desktop and runtime evidence is sufficient for planning; remaining editor/hardware checks move to the handoff, sandbox checks are conditional, and native X11 remains accepted-unverified; presentation has its own qualification.

- [Qualify Wayland CPU presentation without unsupported pixel padding](issues/32-qualify-wayland-presentation.md#resolution): the matched ARGB route on an explicit unreleased upstream pin is accepted for planning; production adoption and remaining acceptance gates are not waived.

- [Select the Rust-native platform stack](issues/27-select-platform-stack.md#resolution): winit, pinned upstream softbuffer, arboard and ashpd are selected with the poll/wait pump model; whole-window opacity becomes a no-op and portal-controlled Cancel text is accepted.

- [Choose SDL-free pixel storage and presentation conversion](issues/34-choose-sdl-free-pixel-storage.md#resolution): compact Rust-owned coverage atlases and a persistent opaque draw target feed full stride-aware presentation copies; parity, lifecycle and copy/memory costs remain gated.

- [Define the pump-loop and dialog-runtime boundary](issues/35-define-pump-and-dialog-runtime-boundary.md#resolution): main-thread event/clipboard ownership, a process-lived dialog runtime and generation-fenced completions are selected; parent retention and UTF-8-only clipboard text are explicitly accepted.

- [Design SDL-free platform contract testing](issues/36-design-sdl-free-contract-testing.md#resolution): private-Xvfb contract runs share production translation/drawing/lifetime logic with narrow test seams; historical goldens stay strict and live evidence stays separate.

- [Preserve fatal-error reporting without SDL](issues/38-choose-sdl-free-fatal-error-reporting.md#resolution): lazy native initialization preserves bootstrap diagnostics; direct bounded D-Bus notification replaces the blocking fatal-error box without a GUI helper.

- [Choose process-signal quit and wake integration](issues/37-choose-process-signal-quit-integration.md#resolution): eligible SIGINT/SIGTERM reuse the shared Tokio runtime from startup; process-owned pending quits survive Lua restart and wake native waits without handler-side Lua work.

- [Define the SDL3 replacement handoff](issues/28-plan-sdl3-migration.md#resolution): five separately checked steps cover reference preparation, independent ports, renderer retirement, the coupled native cutover and final SDL-free artifact proof; implementation remains separate.

## Not yet specified

None.

## Out of scope

- Implementing or adopting the replacement during this planning effort.
- Replacing Lua or the native-plugin interface; replacing libc bindings; restoring non-Linux platforms.
- GPU rendering, a rendering redesign, or new windowing features merely because a candidate supports them.
- Restoring functional whole-window opacity: the user explicitly chose a compatibility no-op and stopped the opacity investigation; possible recovery is a separate future effort ([Select the Rust-native platform stack](issues/27-select-platform-stack.md#resolution)).
- Writing a Wayland/X11 client, maintaining a fork, or keeping a hidden SDL fallback without a new explicit user decision.
- Fixing the editor's idle-loop spin (plugin polling threads keep `system.wait_event` polling about 1,000–1,600 times a second while focused): a Lua behaviour change, recorded in [Calibrate platform performance and memory gates](issues/31-calibrate-platform-cost-gates.md); a replacement must reproduce it, not fix it.
