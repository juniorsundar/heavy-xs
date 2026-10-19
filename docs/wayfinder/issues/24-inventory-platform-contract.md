# Inventory the observable platform contract

Parent: ../sdl3-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-sdl3-contract
Blocked by: none

## Question

What exactly do the editor's Lua code, plugins and tests observe from the SDL-backed platform layer today, and which of it is exercised by tests?

List every `system.*` and `renwindow.*` function and every `system.poll_event` event with its arguments, the behaviour behind each (window modes, scale and size units, key names and modifiers, text input and IME, mouse and touch, drag and drop, clipboard and primary selection, dialogs and their `dialogfinished` results, cursors, hit-testing, opacity, focus, sandbox detection), its source location, and whether a contract suite or check script covers it. Record environment behaviour that shapes it: SDL hints, Wayland versus X11 differences visible in the code, the offscreen and dummy drivers the tests rely on, and threads that push events.

Return one source-backed inventory under `assets/` with coverage gaps called out. No candidate evaluation, no behaviour change.

Context: informs [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md).

## Comments

### Resolution

Inventory: [Observable SDL3 platform contract inventory](../assets/platform-contract-inventory.md), source-read at `f52aa273` with 197 resolved source links. It selects no replacement.

- **Surface:** 41 `system.*` functions, 4 `renwindow.*` functions plus `__gc`, 26 `poll_event` event names (25 reach Lua; `dirmonitor` only wakes the loop), 8 SDL hints. Each row gives arguments, units, errors, the SDL calls behind it and its test coverage.
- **Units are mixed:** mouse, motion and drop coordinates are surface pixels (SDL points times the window's float pixel/point ratio); touch is in points; wheel is in its own units. No scale value is exposed to Lua.
- **Quirks a replacement must keep or consciously change:** focus-gained is swallowed and flushes queued key presses; `set_window_opacity` always reports success; `set_clipboard`/`set_primary_selection` ignore failures; `absolute_path` returns nothing on failure; hit-test state is one global, so only one borderless window can have regions; `system.rmdir` deletes files too.
- **Tests:** contract suites run on SDL's offscreen driver and the startup check on the dummy driver; test hooks forge events into SDL's queue and read pixels through SDL. A run of `scripts/rust-port/verify.sh` passed 15 of 16 suites; `process` failed one timing assertion (`interrupt then dead`), outside the platform layer and not investigated.
- **Never observed:** any scale other than 1 (so fractional scaling and the pixel/point meaning of `get_size` are unverified), real IME preedit, live portal dialogs, real drag and drop or touch, clipboard keep-alive after exit, window-manager behaviour of modes, borderless regions and opacity.
