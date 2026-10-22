# SDL3 Step 4 live desktop review

Human review of the native cutover on the user's KDE Plasma Wayland session, 2026-10-22, as required by the [implementation handoff](sdl3-migration-handoff.md#real-desktop-and-cost-acceptance). Results are the user's own reports; nothing here is inferred from automated runs.

## Setup

- Outputs at 1.2×, 1× and 1.25× (existing configuration; no scale changed). Keyboard layout `us` only.
- Release builds installed into a scratch prefix with `scripts/install-linux.sh`, an isolated `LITE_USERDIR` profile setting `config.use_system_file_picker = true`, and two nonprivate text files. Launched from the Nix development shell.
- First pass: source at `da945168` with both backends compiled in; XWayland forced by unsetting `WAYLAND_DISPLAY`.
- Second pass: `8c79a8a0`, the X11-default build and a `--features wayland` build.

## Results

| Check | Native Wayland | XWayland |
| --- | --- | --- |
| Window open/close, minimise, maximise, resize | pass | pass |
| Scaling across the three outputs | pass | not separately reported |
| Keyboard editing, navigation, undo | pass | pass |
| File drop from Dolphin | **fail: known issue** (winit 0.30.13 has no Wayland drop) | pass |
| Window icon | **fail: known issue** (winit 0.30.13 Wayland `set_window_icon` is empty) | pass |
| Scroll after opening a file from the portal dialog | fail at `da945168`: wheel zoomed until another key action (Ctrl stayed held in Lua after the dialog took focus); pass after `ec25d7e2` | pass |
| Re-test of the Ctrl fix, default build on XWayland with `WAYLAND_DISPLAY` set, clipboard/primary with KWrite, multi-click and dragging a selection past the window edge, dialog cancel/error | pass ("works as expected", second pass) | pass (same report) |

The two Wayland failures led to reviewed difference 8 in the handoff: native Wayland became an opt-in `wayland` feature, with file drop and window icon recorded as known issues.

## Not covered by this review

- **IME:** no input method framework was found on the machine. Pending.
- **Non-US layouts and dead keys:** only `us` is configured. Pending.
- **Touch hardware, native X11 session, sandbox package, cold-cache startup:** dispositions unchanged from the handoff (unverified, accepted-unverified or conditional).
- **Clipboard and primary selection without a clipboard manager:** needs an isolated session. Pending.
- **Calibrated performance and memory gates:** not run. Pending.
- The editor was launched inside the development shell; testing it outside the shell is part of Step 5.
