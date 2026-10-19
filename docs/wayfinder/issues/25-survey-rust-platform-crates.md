# Survey Rust-native platform crates

Parent: ../sdl3-map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-sdl3-research
Blocked by: none

## Question

Which maintained Rust crates, alone or composed, can replace each SDL area the editor uses on Linux (window and events, keyboard and IME, CPU frame presentation, clipboard and primary selection, file dialogs), and where do their verified capabilities or dependencies fall short?

Investigate primary sources (docs.rs for the released version, the release tag's source). Consider candidates such as winit, softbuffer, smithay-client-toolkit, x11rb, arboard, smithay-clipboard, rfd and ashpd without treating them as a shortlist. For each area cover: Wayland and X11 support; fractional scaling and the units sizes come in; IME composition and the text-input area; borderless windows with custom hit-testing (move/resize regions); opacity; touch; Wayland primary selection; whether the event loop can be polled or waited with a timeout from the editor's own loop and woken from other threads; the CPU buffer format presentation expects; and portal-based dialogs.

Audit native dependencies end to end: which system libraries are linked or dlopened, whether any C/C++ is compiled, and no SDL anywhere in the closure. Do not select a stack or infer equivalence from API presence.

Capture one Markdown report on a throwaway `research/sdl3-platform-crates` branch, push it, open no PR, never merge, and link it from this ticket.

Context: informs [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md).

## Comments

### Resolution

Report: [Rust-native platform crates to replace SDL3](https://github.com/juniorsundar/heavy-xs/blob/21f81b2b/docs/wayfinder/assets/sdl3-platform-crates.md), on the throwaway branch `research/sdl3-platform-crates` (commit `21f81b2b`, pushed, no PR, never merged). Research only: no stack selected.

- **Window, events, keyboard, IME:** winit 0.30.13 covers Wayland and X11, layout-aware and physical keys, IME preedit/commit with a cursor area, touch, file drop, cursors and icons. It can be pumped with a timeout from the editor's own loop and woken from other threads (`pump_app_events`, `EventLoopProxy`), though its docs warn the pump API is not portable and discouraged.
- **Missing in winit:** no window opacity setter (only transparent/blur hints) and no per-point hit-test callback; borderless move/resize is only `drag_window`/`drag_resize_window` started by the app.
- **Presentation:** softbuffer 0.4.8 takes a 0RGB `u32` buffer in physical pixels with damage rectangles; the app must size it from the scale factor itself. Master has an unreleased breaking pixel-format change, so pin 0.4.8.
- **Clipboard:** arboard 3.7.0 covers clipboard and primary selection; on Wayland it needs the `wayland-data-control` feature and compositor support (secondary selection is X11 only). smithay-clipboard is Wayland-only text.
- **Dialogs:** rfd (portal by default, sync and async, filters, folders, save) and ashpd's file chooser are both viable; rfd parent modality and ashpd folder picking are unverified.
- **Native dependencies:** winit and softbuffer dlopen system libraries by default; no SDL in any audited closure; pixels pulls wgpu (rules it out under CPU-only). No `cargo tree` or binary scan was run, so closure claims are from manifests only.
- **Unverified at runtime:** fractional 1.25 scale end to end, IME on Wayland (text-input-v3) and X11 (XIM), clipboard keep-alive on KDE, borderless dragging on KDE, async dialog wake-up of the loop, sandbox detection (`ashpd::is_sandboxed` is an unconfirmed lead).

A light independent review passed five load-bearing claims against release tarballs and docs.rs and corrected three details before publication. Other crates' claims were not reviewed.
