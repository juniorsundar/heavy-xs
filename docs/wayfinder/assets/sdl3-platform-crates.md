# SDL3 replacement on Linux: Rust-crate capability research (research only, no selection)

Date of research: 2026-10-19. Scope: Linux only (Wayland + X11). All claims cite the latest
RELEASED version at research time, via version-pinned docs.rs URLs and release-branch GitHub
sources. Nothing here was benchmarked and no stack is selected or recommended.

## Pinned versions examined

| Crate | Version | Release-date signal | MSRV signal |
|---|---|---|---|
| winit | 0.30.13 | 2026-03-02 ([crates.io API](https://crates.io/api/v1/crates/winit/0.30.13)); latest prerelease 0.31.0-beta.3 | 1.70.0 (crates.io `rust_version` for 0.30.13); master (0.31-beta) pins 1.86 |
| softbuffer | 0.4.8 | 2025-12-13 (crates.io listing) | 1.71.0 (Cargo.toml) |
| smithay-client-toolkit (sctk) | 0.21.1 | 2026-07-23 (crates.io listing) | 1.86 (Cargo.toml) |
| x11rb | 0.14.0 | 2026-09-27 (docs.rs header) | 1.68+ (docs.rs badge) |
| arboard | 3.7.0 | ~2026-10-16 (GitHub release "3 days ago" at research time; approximate) | 1.71.0 (Cargo.toml) |
| smithay-clipboard | 0.7.3 | exact date unknown | 1.85.0 (Cargo.toml) |
| rfd | 0.17.2 | exact date unknown | 1.92 (Cargo.toml) |
| ashpd | 0.13.13 | ≤2026-07 (distro packaging of 0.13.13 dated 2026-07-19; approximate) | 1.92 (workspace rust-version; ashpd is a workspace member) |
| tao | 0.37.1 | exact date unknown; 0.37.x series current (changelog cites 2026-era PRs) | unknown |
| pixels | 0.17.2 | exact date unknown | unknown (policy: see MSRV.md on GitHub; not read) |
| minifb | 0.29.0 | ~2026-10-02 (crates.io listing; approximate) | unknown |
| xkbcommon (xkbcommon-rs) | 0.9.0 | exact date unknown | unknown |
| raw-window-handle | 0.6.2 | exact date unknown | 1.64.0 |
| wayland-sys | 0.31.11 | exact date unknown | unknown |

Version URLs: <https://docs.rs/crate/winit/0.30.13>, <https://docs.rs/crate/softbuffer/0.4.8>,
<https://docs.rs/crate/smithay-client-toolkit/0.21.1>, <https://docs.rs/crate/x11rb/0.14.0>,
<https://docs.rs/crate/arboard/3.7.0>, <https://docs.rs/crate/smithay-clipboard/0.7.3>,
<https://docs.rs/crate/rfd/0.17.2>, <https://docs.rs/crate/ashpd/0.13.13>,
<https://docs.rs/crate/tao/0.37.1>, <https://docs.rs/crate/pixels/0.17.2>,
<https://docs.rs/crate/minifb/0.29.0>, <https://docs.rs/crate/xkbcommon/latest>,
<https://docs.rs/crate/raw-window-handle/0.6.2>, <https://docs.rs/crate/wayland-sys/latest>.

Legend for the table: S = supported, P = partial, N = no, ? = unknown. Each cell carries its
citation; fuller evidence is in the per-area notes.

## Summary table (area × candidate)

| Area | winit 0.30.13 | softbuffer 0.4.8 | sctk 0.21.1 | x11rb 0.14.0 | arboard 3.7.0 | smithay-clipboard 0.7.3 | rfd 0.17.2 | ashpd 0.13.13 | tao 0.37.1 | pixels 0.17.2 | minifb 0.29.0 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1. Wayland+X11 backends | S [W](https://docs.rs/winit/0.30.13/winit/platform/wayland/index.html) [X](https://docs.rs/winit/0.30.13/winit/platform/x11/index.html) [F](https://docs.rs/crate/winit/0.30.13/features) | S [W+X](https://docs.rs/softbuffer/0.4.8/softbuffer/#platform-support) | Wayland S, X11 N [D](https://docs.rs/crate/smithay-client-toolkit/0.21.1) | X11 S, Wayland N [D](https://docs.rs/x11rb/0.14.0/x11rb/) | S (both, feature-gated) [D](https://docs.rs/crate/arboard/3.7.0) | Wayland S, X11 N [D](https://docs.rs/crate/smithay-clipboard/0.7.3) | S (both via portal; GTK opt) [D](https://docs.rs/rfd/0.17.2/rfd/#linux--bsd-backends) | S (portal = both) [D](https://docs.rs/ashpd/0.13.13/ashpd/) | S (both; winit fork lineage) [D](https://docs.rs/crate/tao/0.37.1) | ? (delegates to windowing; GPU-gated) [D](https://docs.rs/crate/pixels/0.17.2) | S (both incl. Wayland) [R](https://github.com/emoon/rust_minifb) |
| 2. Fractional scale | S (compositor-suggested, fr. protocol; f64 factor) [W](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs) | P (buffer sized in physical px; no scale logic itself) [B](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html) | S (dpi-aware surfaces; wayland-protocols incl. staging) [C](https://raw.githubusercontent.com/Smithay/client-toolkit/master/Cargo.toml) | N/A (X11 has no fractional-scale protocol; DPI via Xft.dpi/XRandR heuristics) [W](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs) | N/A | N/A | N/A | N/A | ? (not verified) | ? (not verified) | ? (not verified) |
| 3. IME | S (preedit/commit/enable + cursor-area + purpose via new request API) [I](https://docs.rs/winit/0.30.13/winit/event/enum.Ime.html) [T](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs) | N/A | P (input-method-unstable-v2 + v3 modules exist; text-input-v3 framing not verified) [S](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/seat/index.html) | N (no XIM evidence found) | N/A | N/A | N/A | N/A | ? (not verified) | N/A | ? (not verified) |
| 4. Borderless custom hit-test | P (whole-window drag/resize + bool hittest only; no per-point callback) [T](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs) [C](https://docs.rs/winit/0.30.13/winit/changelog/index.html) | N/A | ? (not verified) | S-low-level (raw X11 can implement anything; no helper) [D](https://docs.rs/x11rb/0.14.0/x11rb/) | N/A | N/A | N/A | N/A | ? (not verified) | N/A | N (prototyping-oriented) [D](https://docs.rs/crate/minifb/0.29.0) |
| 5. Opacity / touch / file-drop / cursor / icon | opacity N; touch S; drop S; cursor S; icon S [T](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs) [E](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html) [Ci](https://docs.rs/winit/0.30.13/winit/window/enum.CursorIcon.html) [Ic](https://docs.rs/winit/0.30.13/winit/window/struct.Icon.html) | N/A | ? | ? | N/A | N/A | N/A | N/A | ? | N/A | P (buffer+input only) [D](https://docs.rs/crate/minifb/0.29.0) |
| 6. Keyboard (layout name + scancode + mods) | S (Key/logical + KeyCode/physical + text + location + modifiers) [K](https://docs.rs/winit/0.30.13/winit/event/struct.KeyEvent.html) [M](https://docs.rs/winit/0.30.13/winit/keyboard/index.html) | N/A | P (wl_keyboard + libxkbcommon wrapper) [D](https://docs.rs/crate/smithay-client-toolkit/0.21.1) | P (raw X11 events; app maps via xkbcommon crate) [D](https://docs.rs/x11rb/0.14.0/x11rb/) [Xk](https://docs.rs/crate/xkbcommon/latest) | N/A | N/A | N/A | N/A | ? | N/A | P (own Key enum; layout mapping not verified) |
| 7. App-owned pump/wait loop + wakeup | S, with caveats: the pump API is documented as "not a portable API" that apps "almost certainly shouldn't use" (pump_events with timeout; run_on_demand; EventLoopProxy; WaitUntil) [P](https://docs.rs/winit/0.30.13/winit/platform/pump_events/trait.EventLoopExtPumpEvents.html) [R](https://docs.rs/winit/0.30.13/winit/platform/run_on_demand/trait.EventLoopExtRunOnDemand.html) [Px](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html) [C](https://docs.rs/winit/0.30.13/winit/event_loop/enum.ControlFlow.html) | N/A | ? (calloop-based; ownership model not verified) | S (direct fd polling; fully app-owned) [D](https://docs.rs/x11rb/0.14.0/x11rb/) | N/A | N/A | N/A (spawns own dialog; async API exists) [D](https://docs.rs/rfd/0.17.2/rfd/) | N/A (async D-Bus; needs executor) [D](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/) | ? (run_return-style API historically; not verified for 0.37.1) | N/A | S (app-owned `is_open` loop) [D](https://docs.rs/crate/minifb/0.29.0) |
| 8. CPU presentation (format/damage/scale) | N/A (delegates to softbuffer-style lib) | S (0RGB u32; damage; age) [B](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html) [Sf](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Surface.html) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N (GPU/wgpu path) [D](https://docs.rs/pixels/latest/pixels/) | S (own 32-bit buffer path) [D](https://docs.rs/crate/minifb/0.29.0) |
| 9. Clipboard + primary selection | N/A | N/A | S-low-level (data_device + primary_selection modules) [M](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/) | S-low-level (raw X11 selections) | S (clipboard+primary; secondary X11 only; Wayland needs feature and compositor support) [L](https://docs.rs/arboard/latest/arboard/enum.LinuxClipboardKind.html) [Cb](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html) | P (Wayland text only; own thread) [Cl](https://docs.rs/smithay-clipboard/latest/smithay_clipboard/struct.Clipboard.html) [Cd](https://docs.rs/crate/smithay-clipboard/0.7.3) | N/A | N/A ( Clipboard portal exists but not examined) | N/A | N/A | ? |
| 10. File dialogs (portal/async/filter/dir) | N/A | N/A | N/A | N/A | N/A | N/A | S (portal default; async; filters; pick_folder; save) [D](https://docs.rs/rfd/0.17.2/rfd/) [A](https://docs.rs/rfd/latest/rfd/struct.AsyncFileDialog.html) | S (open/save/multi-save; filters; async; folder request type not verified) [F](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/) | N/A | N/A | N/A |
| 11. No SDL in closure; native deps | S (no SDL; dlopen by default) [F](https://docs.rs/crate/winit/0.30.13/features) [Wp](https://docs.rs/winit/0.30.13/winit/platform/wayland/index.html) [Ws](https://docs.rs/crate/wayland-sys/latest) | S (no SDL; dlopen by default) [Ct](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/Cargo.toml) | S (no SDL; links libwayland/libxkbcommon unless dlopen) [D](https://docs.rs/crate/smithay-client-toolkit/0.21.1) | S (pure-Rust conn; links or dlopens libxcb) [Rc](https://docs.rs/x11rb/latest/x11rb/rust_connection/) [Ft](https://docs.rs/crate/x11rb/0.14.0/features) | S (no SDL; x11rb + opt wl-clipboard-rs) [Ct](https://raw.githubusercontent.com/1Password/arboard/master/Cargo.toml) | S (no SDL; dlopen feature default) [Ct](https://raw.githubusercontent.com/Smithay/smithay-clipboard/master/Cargo.toml) | P (portal path: no GTK build; GTK path needs C libs; zenity/libdbus runtime) [D](https://docs.rs/rfd/0.17.2/rfd/#linux--bsd-backends) [Ct](https://raw.githubusercontent.com/PolyMeilex/rfd/master/Cargo.toml) | S (pure-Rust zbus; no C portal lib) [D](https://docs.rs/ashpd/0.13.13/ashpd/) | ? (not audited) | N (pulls wgpu/GPU stack; no SDL but violates CPU-only goal) [D](https://docs.rs/pixels/latest/pixels/) | ? (bundles C; not audited) |
| 12. Maintained | S (2026-09-20) | P (2025-12-13; breaking rewrite unreleased on master) [Ch](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/CHANGELOG.md) | S (2026-07-23) | S (2026-09-27) | S (~2026-10-16, approx) | ? (version verified; date unknown) | ? (version verified; date unknown) | P (0.13.13 ≤2026-07; cadence unclear) | S (0.37.x active in 2026) [Ch](https://raw.githubusercontent.com/tauri-apps/tao/dev/CHANGELOG.md) | ? (version verified; date unknown) | P (~2026-10-02 approx; prototyping scope) |

## Per-area notes

### 1. Wayland and X11 support

1. winit 0.30.13 has both backends behind default features `wayland` and `x11`
 (17 default features incl. `wayland-dlopen`, `wayland-csd-adwaita`, `x11`) —
 [features](https://docs.rs/crate/winit/0.30.13/features). Dedicated platform modules exist for
 [Wayland](https://docs.rs/winit/0.30.13/winit/platform/wayland/index.html) and
 [X11](https://docs.rs/winit/0.30.13/winit/platform/x11/index.html); scope statement confirms
 Unix-via-X11 and Unix-via-Wayland are both supported targets —
 [FEATURES.md](https://raw.githubusercontent.com/rust-windowing/winit/master/FEATURES.md).
2. softbuffer 0.4.8 supports Wayland, XCB and Xlib presentation paths (all marked present) —
 [platform support](https://docs.rs/softbuffer/0.4.8/softbuffer/#platform-support).
3. smithay-client-toolkit 0.21.1 is Wayland-only (built on `wayland-client`; crate description is
 "a toolkit for writing wayland clients") —
 [docs](https://docs.rs/crate/smithay-client-toolkit/0.21.1). It cannot cover X11.
4. x11rb 0.14.0 is X11-only ("X11 rust bindings"; connection to an X11 server) —
 [docs](https://docs.rs/x11rb/0.14.0/x11rb/). It cannot cover Wayland.
5. arboard 3.7.0 covers X11 by default and Wayland via the opt-in `wayland-data-control` feature
 (prioritized over X11 when enabled) —
 [docs](https://docs.rs/crate/arboard/3.7.0). Wayland support is off by default because it needs
 compositor support for data-control protocols.
6. smithay-clipboard 0.7.3 is Wayland-only ("provides access to the Wayland clipboard ...
 for applications already using some sort of GUI toolkit or a windowing library, like winit") —
 [docs](https://docs.rs/crate/smithay-clipboard/0.7.3).
7. rfd 0.17.2 Linux backends: GTK3 or XDG Desktop Portal (portal is the default feature
 `xdg-portal`; `wayland` feature pulls wayland-client/protocols for the portal path) —
 [backends](https://docs.rs/rfd/0.17.2/rfd/#linux--bsd-backends),
 [features](https://docs.rs/crate/rfd/0.17.2/features). The portal path is backend-agnostic
 (works under both Wayland and X11 sessions, subject to a portal backend being installed).
8. ashpd 0.13.13 is a pure-Rust zbus wrapper of the XDG portal D-Bus interfaces, an alternative
 to the C libportal —
 [docs](https://docs.rs/ashpd/0.13.13/ashpd/). Portal-based, so session-type agnostic.
9. tao 0.37.1 is a winit-derived cross-platform windowing library covering Linux —
 [docs](https://docs.rs/crate/tao/0.37.1). Per-backend verification was not done (unknown).
10. pixels 0.17.2 delegates windowing to any raw-window-handle provider (winit suggested) —
 [docs](https://docs.rs/pixels/latest/pixels/). Its own backend coverage was not verified (unknown).
11. minifb 0.29.0 documents Linux X11 and Wayland support (repo build notes list Windows, X11,
 Wayland, macOS, ...) —
 [repo](https://github.com/emoon/rust_minifb) — but it is explicitly "primarily designed for
 prototyping" —
 [docs](https://docs.rs/crate/minifb/0.29.0).

### 2. Fractional scale

1. winit reports scale via `Window::scale_factor() -> f64`; on Wayland "the scale factor is
 suggested by the compositor for each window individually by using the wp-fractional-scale
 protocol if available" with integer fallback; on X11 it uses `WINIT_X11_SCALE_FACTOR` env,
 `Xft.dpi`, then XRandR mm-dimension heuristics —
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 Sizes/events use logical vs physical types from the `dpi` crate —
 [dpi](https://docs.rs/winit/0.30.13/winit/dpi/index.html). How the app must size a CPU buffer
 against a fractional factor (surface size × scale, viewport semantics) is NOT documented in
 winit's public docs as far as verified — gap; the protocol rule itself (buffer = surface × scale
 via wp_viewport) is documented by the protocol references, e.g.
 [fractional-scale-v1](https://wayland.app/protocols/fractional-scale-v1). Whether winit 0.30.13
 exposes the exact fractional value vs a rounded one, and what `inner_size` returns under 1.25×
 on KDE, needs a runtime probe (see experiments).
2. softbuffer has no scale logic: `Surface::resize(NonZeroU32, NonZeroU32)` + `Buffer` of
 `width × height` pixels in physical pixels; the app must pass the right size —
 [Surface](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Surface.html),
 [Buffer](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html). Correctness of the
 app's scale→buffer-size mapping is therefore the app's job (unknown until prototyped).
3. sctk 0.21.1 advertises "utilities for creating dpi aware surfaces" and depends on
 `wayland-protocols` 0.32.9 with `staging` + `unstable` protocol features (the channel through
 which fractional-scale support would arrive) —
 [docs](https://docs.rs/crate/smithay-client-toolkit/0.21.1),
 [Cargo](https://raw.githubusercontent.com/Smithay/client-toolkit/master/Cargo.toml). Direct
 wp_fractional_scale_v1 API use was not verified — unknown.
4. X11 has no fractional-scale protocol; per-point fractional values like 1.25 do not exist
 there — DPI is approximated (see winit X11 heuristic above). Any X11-side scaling behavior must
 be treated as unknown until measured.

### 3. IME

1. winit 0.30.13 models IME as `WindowEvent::Ime(Ime)` with `Enabled / Preedit(String,
 Option<(usize, usize)>) / Commit(String) / Disabled` —
 [Ime](https://docs.rs/winit/0.30.13/winit/event/enum.Ime.html) — gated on explicitly enabling
 IME, and positioned via cursor-area APIs —
 [WindowEvent::Ime](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html). In the
 current (master/0.31-line, and deprecated-shimmed in 0.30.x) API the calls are
 `set_ime_allowed` / `set_ime_cursor_area` / `set_ime_purpose` (deprecated) superseded by
 `request_ime_update(ImeRequest)` + `ime_capabilities()` —
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 Backend mapping to Wayland text-input-v3 vs X11 XIM was NOT verified from primary sources —
 gap: confirm at runtime on KDE Wayland and on X11 (see experiments). Changelog shows ongoing
 IME fixes (e.g. 0.30.13 macOS Pinyin/Windows fixes) —
 [changelog](https://docs.rs/winit/0.30.13/winit/changelog/index.html).
2. sctk 0.21.1 ships `seat::input_method` (input-method-unstable-v2) and `seat::input_method_v3`
 (experimental xx-input-method-v2, hoped to become v3) plus keyboard-filter modules —
 [seat](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/seat/index.html).
 These are the compositor-side/IME-authoring protocols, not the client text-input-v3 interface an
 editor needs; client-side text-input-v3 framing was not found — gap/unknown.
3. x11rb: no XIM helper found; the crate exposes raw protocol (43 feature flags, incl. `xkb`
 for key description) —
 [features](https://docs.rs/crate/x11rb/0.14.0/features). XIM support: unknown, likely absent
 (no claim made).
4. tao/minifb IME: not verified — unknown.

### 4. Borderless windows with app-defined move/resize regions

1. winit offers `Window::drag_window()` (move with LMB until release) and
 `Window::drag_resize_window(ResizeDirection)` (direction chosen by the app, e.g. from cursor
 position near edges — the bundled example computes the direction from cursor position and
 `scale_factor`) plus `set_cursor_hittest(bool)` (whole-window boolean, not per-point) —
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 There is NO per-point hit-test callback equivalent to SDL_SetWindowHitTest in the verified API:
 `set_cursor_hittest` only toggles hit-testing for the whole window, and platform notes say
 Wayland drag needs a pressed button/touch inside the window while `show_window_menu` is
 unsupported on Wayland/X11. X11 `set_hittest` fixes appear in the changelog (0.30.13) —
 [changelog](https://docs.rs/winit/0.30.13/winit/changelog/index.html) — but that is the same
 boolean API, not a region callback. Verdict: partial; custom per-region drag/resize must be
 hand-built from cursor position + the two drag calls (unknown how well this behaves on KDE
 Wayland — see experiments).
2. sctk/tao hit-test helpers: not verified — unknown.
3. x11rb: raw X11 allows implementing any hit-testing manually (no helper) —
 [docs](https://docs.rs/x11rb/0.14.0/x11rb/). Effort, not capability, is the question.

### 5. Opacity, touch, file drop, cursor shapes, window icon

1. Opacity: NO `set_opacity` exists in the verified winit Window API (master trait has
 `set_transparent`/`set_blur` only; blur on Wayland needs `org_kde_kwin_blur_manager` or
 `ext_background_effect_manager_v1`) —
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 Verdict: no (for SDL_WINDOW-opacity semantics). Transparency is only a boolean hint.
2. Touch: `WindowEvent::Touch(Touch)` with id/phase/location/force exists —
 [Touch](https://docs.rs/winit/0.30.13/winit/event/struct.Touch.html),
 [WindowEvent](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html). Linux-backend
 delivery (Wayland/X11) not verified per backend — partial.
3. File drop: `DroppedFile / HoveredFile / HoveredFileCancelled` events exist —
 [WindowEvent](https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html). Wayland
 data-device delivery vs X11 XDND delivery not verified per backend — partial.
4. Cursor shapes: full `CursorIcon` enum (36 variants, CSS-derived) + `Window::set_cursor`,
 custom cursors, grab modes, visibility —
 [CursorIcon](https://docs.rs/winit/0.30.13/winit/window/enum.CursorIcon.html),
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 Verdict: supported (API presence verified; per-backend fidelity on KDE/Wayland vs X11 unknown).
5. Window icon: `Icon::from_rgba` + `set_window_icon`; on Wayland the compositor must implement
 `xdg_toplevel_icon` —
 [Icon](https://docs.rs/winit/0.30.13/winit/window/struct.Icon.html),
 [Window trait](https://raw.githubusercontent.com/rust-windowing/winit/master/winit-core/src/window.rs).
 Verdict: supported with a Wayland-compositor dependency (KDE support unverified — see experiments).

### 6. Keyboard

1. winit `KeyEvent { physical_key: PhysicalKey (KeyCode positions ≈ scancode), logical_key: Key
 (layout-aware meaning), text, location, state, repeat }` + `Modifiers`/`ModifiersState`;
 `physical_key` docs explicitly describe AZERTY-style position-vs-label semantics —
 [KeyEvent](https://docs.rs/winit/0.30.13/winit/event/struct.KeyEvent.html),
 [keyboard](https://docs.rs/winit/0.30.13/winit/keyboard/index.html). Verdict: supported at API
 level (scancode-equivalence caveats are documented in the type itself).
2. sctk wraps `wl_keyboard` with automatic keymap interpretation via libxkbcommon —
 [docs](https://docs.rs/crate/smithay-client-toolkit/0.21.1). Verdict: partial (building block,
 not an editor-ready key pipeline).
3. x11rb gives raw X11 key events; layout interpretation needs the `xkbcommon` crate
 (0.9.0, Wayland/X11 features) —
 [xkbcommon](https://docs.rs/crate/xkbcommon/latest). Verdict: partial (manual composition).
4. minifb exposes its own `Key` enum + `is_key_down` polling —
 [docs](https://docs.rs/crate/minifb/0.29.0). Layout-aware naming/scancode parity: unknown.

### 7. Event loop

1. winit 0.30.13 supports app-owned pumping: `EventLoopExtPumpEvents::pump_events(timeout,
 handler)` / `pump_app_events` on android/macos/wayland/windows/x11 —
 [pump_events](https://docs.rs/winit/0.30.13/winit/platform/pump_events/trait.EventLoopExtPumpEvents.html)
 (bundled `pump_events.rs` example pumps with `Duration::ZERO`); `EventLoopExtRunOnDemand`
 (`run_on_demand`/`run_app_on_demand`) returns control to the caller —
 [run_on_demand](https://docs.rs/winit/0.30.13/winit/platform/run_on_demand/trait.EventLoopExtRunOnDemand.html).
 Cross-thread wakeup via `EventLoopProxy::send_event` (emits `UserEvent`) —
 [proxy](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html);
 `EventLoop` itself is `!Send + !Sync` (lives on one thread; `Window` is `Send + Sync`) —
 [EventLoop](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoop.html).
 Timeout-wait exists at the callback level via `ControlFlow::WaitUntil(Instant)` —
 [ControlFlow](https://docs.rs/winit/0.30.13/winit/event_loop/enum.ControlFlow.html). Caveat:
 changelog 0.30.11 fixed "`pump_events` driven loop deadlocking when loop was not drained before
 exit" on Wayland —
 [changelog](https://docs.rs/winit/0.30.13/winit/changelog/index.html) — so exact
 drain-before-exit discipline matters (see experiments). Main-thread ownership restrictions for
 Linux were not found (the documented main-thread limits concern macOS/Web/iOS) — Linux
 main-thread requirement: unknown, verify by probe.
2. x11rb: `RustConnection` is pure Rust over a generic stream that "can also be polled" —
 [rust_connection](https://docs.rs/x11rb/latest/x11rb/rust_connection/) — fully app-owned
 polling; no loop ownership at all. Verdict: supported.
3. sctk: calloop integration is a default feature —
 [Cargo](https://raw.githubusercontent.com/Smithay/client-toolkit/master/Cargo.toml) — but
 whether the dispatch loop can be externally pumped with a timeout was not verified — unknown.
4. rfd dialogs are sync or `async` (futures); the portal backend suspends on D-Bus rather than
 integrating with the app loop; waking the editor loop on dialog completion needs an executor
 bridge — [rfd](https://docs.rs/rfd/0.17.2/rfd/). ashpd is `async` throughout (`send().await?
 .response()?`) with tokio/async-io features —
 [file_chooser](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/). Neither was verified
 against a hand-rolled poll/wait loop — gap (see experiments).
5. minifb owns a simple `is_open()` loop —
 [docs](https://docs.rs/crate/minifb/0.29.0) — app-driven but feature-thin.

### 8. CPU presentation

1. softbuffer 0.4.8: `Buffer` derefs to `[u32]`, one u32/px row-major, format `0x00RRGGBB`
 (top byte must be 0) —
 [Buffer](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html); `present()` commits
 (on Wayland: attach + damage + commit), `present_with_damage(&[Rect])` on Wayland/XShm/Win32/Web,
 `age()` for partial-update tracking —
 [Buffer](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html). No-copy paths:
 Wayland, X with XShm, Win32 —
 [Buffer](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html). Sizing is physical
 pixels via `Surface::resize` —
 [Surface](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Surface.html); mapping to
 fractional scale is the app's responsibility (see §2). Verdict: supported.
 CAUTION: softbuffer master (unreleased) breaks this API (`Pixel` struct, target-dependent
 `PixelFormat`, `buffer_mut`→`next_buffer`, DRM/KMS off by default) —
 [changelog](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/CHANGELOG.md).
 Pin to 0.4.8 for the documented behavior.
2. pixels 0.17.2 is GPU-backed ("rendered entirely on the GPU" via wgpu) —
 [docs](https://docs.rs/pixels/latest/pixels/). Verdict: no for a CPU-only pipeline.
3. minifb offers its own 32-bit buffer path —
 [docs](https://docs.rs/crate/minifb/0.29.0). Verdict: supported-ish, but its window management
 is judged inadequate by softbuffer's authors (segfaults, missing icon support) —
 [softbuffer](https://docs.rs/crate/softbuffer/0.4.8).

### 9. Clipboard + primary selection

1. arboard 3.7.0: `Clipboard` plus `LinuxClipboardKind::{Clipboard, Primary, Secondary}` with
 `GetExtLinux`/`SetExtLinux` selectors; on Wayland, Primary needs compositor v2+ or ops error —
 [kind](https://docs.rs/arboard/latest/arboard/enum.LinuxClipboardKind.html),
 [Clipboard](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html). Linux clipboard
 content is hosted by the last owner app (keep-alive semantics); parallel ops can fail with
 `ClipboardOccupied`; `Clipboard`s should be dropped before exit (winit-style takeover noted) —
 [Clipboard](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html). Unix deps: x11rb +
 optional wl-clipboard-rs, parking_lot, percent-encoding (no C clipboard libs) —
 [Cargo](https://raw.githubusercontent.com/1Password/arboard/master/Cargo.toml). Verdict:
 supported (both selections), Wayland gated on `wayland-data-control` + compositor support.
 Thread model beyond the above: unknown (verify keep-alive thread needs on KDE).
2. smithay-clipboard 0.7.3: Wayland text clipboard for GUI apps with a surface; `Clipboard::new`
 takes a raw `*mut wl_display` (unsafe, must outlive) and runs its own thread + event queue;
 `load`/`store` strings —
 [Clipboard](https://docs.rs/smithay-clipboard/latest/smithay_clipboard/struct.Clipboard.html),
 [crate](https://docs.rs/crate/smithay-clipboard/0.7.3). No primary-selection or image API
 verified — partial.
3. sctk exposes `data_device_manager` + `primary_selection` modules for hand-rolled support —
 [modules](https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/) — building
 blocks only.

### 10. File dialogs

1. rfd 0.17.2: sync (`FileDialog`) + async (`AsyncFileDialog`) with filters, starting directory,
 single/multiple file, folder pick, save — all marked ✔ on Linux —
 [rfd](https://docs.rs/rfd/0.17.2/rfd/). Portal backend is default (`xdg-portal` + `wayland`
 features); GTK3 backend is opt-in and needs gtk3 dev headers; portal runtime needs a portal
 backend (GTK/GNOME/KDE) + libdbus (else zenity fallback; zenity also needed for message dialogs)
 —
 [backends](https://docs.rs/rfd/0.17.2/rfd/#linux--bsd-backends),
 [features](https://docs.rs/crate/rfd/0.17.2/features). Async dialog details incl. parent-window
 modal behavior exist but were only skimmed (DeepWiki-level source, not primary) — parent-window
 attachment API unverified (unknown). xdg-portal-wlr explicitly does NOT implement the needed
 D-Bus API. Verdict: supported, portal-flavored, with runtime deps.
2. ashpd 0.13.13 `file_chooser` (feature-gated): `SelectedFiles::open_file/save_file/save_files`
 builders with title/accept-label/modal/multiple/choice/filter (`FileFilter::mimetype/glob`),
 `current_name`/`current_folder`, all `send().await?.response()?` async over
 `org.freedesktop.portal.FileChooser` —
 [module](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/),
 [SelectedFiles](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/struct.SelectedFiles.html).
 A dedicated open-*folder* request type was not confirmed in the verified surface (only
 open/save/save-files + current_folder) — unknown/gap. Sandbox detection helper exists:
 `ashpd::is_sandboxed()` checks `/.flatpak-info`, snap, or `GTK_USE_PORTAL=1` (cached) — secondhand
 citation (not docs.rs-verified; treat as unverified lead).
3. Neither rfd-portal nor ashpd draws its own UI: both need a running portal backend at runtime
 (KDE: `xdg-desktop-portal-kde` — presence/version on the target desktop unverified).

### 11. Native-dependency audit (Linux, needed features)

All of the following are SDL-free by construction (none depends on SDL; verified via the Cargo
manifests and feature tables below). "Link" = linked at build time; "dlopen" = loaded at runtime
with graceful fallback.

- winit 0.30.13 (default): features `wayland, wayland-dlopen, wayland-csd-adwaita, x11`
 pull `winit-wayland/winit-x11`, `x11rb`, `x11-dl`, wayland-client/backend/protocols(-plasma),
 sctk + sctk-adwaita (CSD), memmap2, xkb-related crates —
 [features](https://docs.rs/crate/winit/0.30.13/features). Wayland system libs are dlopened by
 default (disable via `wayland-dlopen`) —
 [wayland module](https://docs.rs/winit/0.30.13/winit/platform/wayland/index.html); the dlopen
 mechanism itself is `wayland-sys`'s `dlopen` cargo feature ("instead of directly linking ...
 open them at runtime ... gracefully handle non-Wayland environments") —
 [wayland-sys](https://docs.rs/crate/wayland-sys/latest). X11 side uses `x11-dl` (dlopen) +
 x11rb. CSD-adwaita pulls font/shaping Rust crates (no GTK). No `cc`/`cmake` C/C++ build scripts
 were found in the verified metadata — but absence was NOT exhaustively proven (unknown; verify
 with `cargo tree`/`cargo build -v` in a probe). No SDL anywhere in this closure.
- softbuffer 0.4.8 (default = kms, x11, x11-dlopen, wayland, wayland-dlopen): optional Rust deps
 `x11rb` (+`dl-libxcb`), `tiny-xlib` (+dlopen), `wayland-sys` (+dlopen), drm/rustix/memmap2/\
 fastrand/bytemuck; MSRV 1.71 —
 [Cargo](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/Cargo.toml)
 (master manifest; default-feature set matches the released crate's documented backends —
 [docs](https://docs.rs/softbuffer/0.4.8/softbuffer/#platform-support)). Default = dlopen
 both sides. No SDL. KMS backend pulls `drm` (Rust) — irrelevant on desktop, can be disabled.
- sctk 0.21.1 (default = calloop + xkbcommon): `wayland-client/cursor/protocols*`,
 `xkbcommon` 0.8 (links libxkbcommon.so), calloop; `dlopen` NOT default (wayland-backend
 `client_system` links libwayland-client; "version 1.15 of the wayland system libraries if using
 the system backend") —
 [Cargo](https://raw.githubusercontent.com/Smithay/client-toolkit/master/Cargo.toml),
 [docs](https://docs.rs/crate/smithay-client-toolkit/0.21.1). Note sctk's own readme text still
 describes an older architecture; the Cargo manifest is authoritative. No SDL.
- x11rb 0.14.0 (no default features): pure-Rust `RustConnection` over a generic pollable stream
 (no C needed) —
 [rust_connection](https://docs.rs/x11rb/latest/x11rb/rust_connection/); `allow-unsafe-code`
 enables the libxcb-based `XCBConnection`; `dl-libxcb` "will prevent libxcb being linked ...
 instead dynamically loaded at runtime" (+ `xcb_ffi::load_libxcb`) —
 [module docs](https://docs.rs/x11rb/0.14.0/x11rb/) (via search excerpt),
 [features](https://docs.rs/crate/x11rb/0.14.0/features). Extension coverage is feature-gated
 (43 flags). No SDL.
- arboard 3.7.0 (unix): x11rb, parking_lot, percent-encoding, log; optional wl-clipboard-rs —
 [Cargo](https://raw.githubusercontent.com/1Password/arboard/master/Cargo.toml). No C clipboard
 libs. No SDL.
- smithay-clipboard 0.7.3: default `dlopen` (= wayland-backend/dlopen); deps libc, sctk
 (calloop, no xkbcommon), wayland-backend `client_system` —
 [Cargo](https://raw.githubusercontent.com/Smithay/smithay-clipboard/master/Cargo.toml). No SDL.
- rfd 0.17.2 default (`xdg-portal` + `wayland`): pollster + wayland-client/backend/protocols;
 GTK sys crates (`gtk-sys/glib-sys/gobject-sys`, gtk4) are optional and off by default —
 [Cargo](https://raw.githubusercontent.com/PolyMeilex/rfd/master/Cargo.toml),
 [features](https://docs.rs/crate/rfd/0.17.2/features). Runtime (not build): portal backend,
 libdbus-or-zenity, zenity for message dialogs —
 [backends](https://docs.rs/rfd/0.17.2/rfd/#linux--bsd-backends). No SDL. (The `ashpd` crate is
 NOT used by rfd's portal path — rfd speaks D-Bus itself; do not conflate the two.)
- ashpd 0.13.13: pure-Rust zbus-based portal wrapper ("alternative to the C library libportal");
 async runtimes via `tokio` (default) / `async-io` features; `backend`/`frontend`/`gtk4`/
 `gdk4wayland`/`gdk4x11` features exist for the demo/backend crates, not the core file-chooser
 path —
 [docs](https://docs.rs/ashpd/0.13.13/ashpd/),
 [features](https://docs.rs/crate/ashpd/0.13.13/features). No SDL, no GTK needed for
 `file_chooser`.
- raw-window-handle 0.6.2: pure interface crate (no native deps) —
 [docs](https://docs.rs/crate/raw-window-handle/latest). This is the glue that lets softbuffer
 attach to winit/tao windows.
- pixels 0.17.2: pulls the wgpu GPU stack (Vulkan preferred on Linux) —
 [docs](https://docs.rs/pixels/latest/pixels/). No SDL, but incompatible with the CPU-only
 requirement.
- tao 0.37.1 / minifb 0.29.0 / xkbcommon 0.9.0 native closures: NOT audited — unknown
 (tao inherits winit-like deps incl. WebKitGTK-adjacent? unverified — do not assume).

### 12. Maintenance signal

- winit 0.30.13 (2026-09-20) + active 0.31-beta workspace: maintained —
 [changelog](https://docs.rs/winit/0.30.13/winit/changelog/index.html),
 [workspace](https://raw.githubusercontent.com/rust-windowing/winit/master/Cargo.toml).
- softbuffer 0.4.8 (2025-12-13): released version stable-ish, but master carries an unreleased
 breaking rewrite (pixel-format change!) — pinning + migration risk —
 [changelog](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/CHANGELOG.md).
- sctk 0.21.1 (2026-07-23), x11rb 0.14.0 (2026-09-27): maintained.
- arboard 3.7.0 (~2026-10-16, approx): maintained (release days before research).
- ashpd 0.13.13 (≤2026-07; GitHub "0.13.0 latest 8 months ago" conflicts — cadence unclear):
 partially verified; treat cadence as unknown.
- tao 0.37.1: changelog shows 2026-era fixes (PRs #1341/#1342) —
 [changelog](https://raw.githubusercontent.com/tauri-apps/tao/dev/CHANGELOG.md): maintained,
 exact release date unknown.
- rfd 0.17.2, smithay-clipboard 0.7.3, pixels 0.17.2, minifb 0.29.0 (~2026-10-02 approx),
 xkbcommon 0.9.0, rwh 0.6.2, wayland-sys 0.31.11: versions verified; exact release dates unknown
 unless noted.

## Review

A light independent review checked five load-bearing claims against the release tarballs, crates.io metadata and version-pinned docs.rs (winit 0.30.13 has no opacity setter and no per-point hit-test callback; pump_app_events(timeout) and EventLoopProxy exist on Linux; softbuffer 0.4.8 is 0RGB u32 with present_with_damage; arboard Primary plus the wayland-data-control feature). All five passed. It corrected winit's release date and MSRV, arboard's Secondary selection (X11 only), and the pump API's own caveats, which are applied above. Other crates' claims were not independently reviewed.

## Concrete gaps (what "replaced" still needs to prove)

1. Per-point hit-test callback: no verified equivalent of SDL_SetWindowHitTest. winit gives only
 whole-window drag/resize + boolean hittest. Any custom title-bar behavior must be prototyped.
2. Window opacity: no verified setter in winit's API at all (only transparent/blur hints).
3. Fractional-scale end-to-end: winit reports the factor, softbuffer takes physical pixels, but
 the exact buffer-size rule at 1.25× on KDE (and what `inner_size` yields) is unverified.
4. IME backend mapping: preedit/commit/cursor-area APIs verified, but Wayland text-input-v3 vs
 X11 XIM wiring and dead-key behavior unverified on both sessions.
5. Clipboard keep-alive/threads on KDE Wayland (data-control availability) + primary-selection
 compositor-version behavior: unverified at runtime.
6. Dialog parent-window modality (rfd) and ashpd open-folder coverage: unverified.
7. Icon on KDE Wayland (`xdg_toplevel_icon` support), touch + file-drop delivery per backend,
 cursor-shape fidelity: unverified.
8. Loop integration: pump-with-timeout + proxy-wakeup verified at API level (with a known
 Wayland drain-before-exit deadlock fix); async dialog completion → loop wakeup bridge unverified.
9. Sandbox (flatpak/snap) detection: only an unverified lead (`ashpd::is_sandboxed`); needs docs.rs
 confirmation.
10. Build-closure proof: no `cargo tree`/link audit performed; C-build-script absence,
 exact .so linkage, and SDL-absence in the final binary are all unverified claims so far.
11. tao / minifb / pixels / xkbcommon details and all exact release dates/MSRVs marked unknown
 above were not chased further (diminishing returns for a non-shortlist).

## Proposed narrow prototype experiments

KDE Wayland desktop (fractional scale 1.25, IME) — each is a ≤1-day spike, no stack commitment:

- E1 (scale): winit 0.30.13 + softbuffer 0.4.8 window logging `scale_factor`,
 `inner_size`, and `ScaleFactorChanged` across 1.0→1.25→2.0; render a 1px grid with buffer =
 inner_size vs inner_size×scale; record which is crisp. Answers gap 3.
- E2 (IME): same harness with IME enabled, `set_ime_cursor_area` following the caret; type
 CJK + dead-keys (e.g. `/ ` E) under KDE; log Preedit/Commit ordering and candidate-box
 placement. Answers gap 4.
- E3 (hit-test): borderless window; implement edge/corner zones from cursor position +
 `drag_window`/`drag_resize_window`; test press-then-drag from each zone on KDE Wayland
 (button-pressed requirement) — record failures. Answers gap 1.
- E4 (clipboard): arboard plain + `wayland-data-control` on KDE: copy/paste text + primary
 selection + exit-with-content (keep-alive); smithay-clipboard load/store in the same session
 for comparison. Answers gap 5.
- E5 (dialogs+loop): app loop using `pump_app_events(Some(timeout))` + `EventLoopProxy` woken
 from a worker thread; fire rfd `AsyncFileDialog` (portal) and ashpd open/save from the
 callback; verify the loop wakes, no Wayland drain deadlock, and dialog modality. Answers gap 8
 and part of 6.
- E6 (misc window): icon (`from_rgba`), cursor shapes, file drop onto window, touch events,
 transparent/blur hints on KDE; opacity attempt to confirm absence. Answers gaps 2 and 7.

X11 session — same rig under X11 (or XWayland off):

- X1 (DPI): log `scale_factor` with/without `WINIT_X11_SCALE_FACTOR` and `Xft.dpi`; check
 `inner_size` units for the CPU buffer. Extends E1.
- X2 (IME): XIM probe with dead-keys + CJK; compare Preedit/Commit stream against Wayland. Extends E2.
- X3 (hit-test/decorations): borderless + drag/resize zones under a WM; confirm `set_hittest`
 boolean behavior; record WM differences. Extends E3.
- X4 (clipboard): primary-selection middle-click paste + `ClipboardOccupied` under parallel
 access; keep-alive after exit. Extends E4.
- X5 (closure audit): `cargo tree -e normal`, `ldd`/runtime `dlopen` strace, and binary scan for
 SDL symbols for the exact feature sets used — turns gap 10 into evidence.
