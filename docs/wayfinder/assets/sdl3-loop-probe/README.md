# Throwaway editor-loop, clipboard and dialog probe

Standalone feasibility evidence for **Prototype the editor loop, clipboard and dialogs**, not the editor, a selected stack or production implementation. Derived from the captured window probe at `f8dacae0`; that branch remains unchanged. This branch moves/reuses its drawing, fonts, IME helper and borderless regions rather than making another widget toolkit.

## Run

From this worktree:

```bash
bash docs/wayfinder/assets/sdl3-loop-probe/run.sh --backend wayland
# X11 client under the current compositor; XWayland is NOT a native X11 session:
bash docs/wayfinder/assets/sdl3-loop-probe/run.sh --backend x11
```

The launcher enters the existing Nix development shell and supplies dlopen library paths. It removes `DISPLAY` for a forced Wayland child and `WAYLAND_DISPLAY` for a forced X11 child, so arboard cannot silently succeed using the other transport. It does not change the parent environment. Missing/unavailable backends fail explicitly.

**Safety:** launch does not read/write the clipboard or contact a portal. Those services are accessed only by the explicit keys below. F2 and F4 overwrite their selections with public test markers; clipboard history may retain those markers. Read results show byte counts and marker-match only, never clipboard text. Dialog results show counts/outcomes only, never paths or URIs. Backend diagnostic messages are redacted. Ordinary keyboard/IME logs inherited from the window probe still contain typed text: use throwaway text and nonprivate files. No automated screenshots are taken. Save chooses a name; the probe itself never opens, reads or writes a selected file.

## Controls

| Key | Action |
| --- | --- |
| F1 | Worker waits 150 ms, then wakes the pump via `EventLoopProxy` |
| F2 | **Overwrite clipboard** with `lite-xl clipboard probe` |
| F3 | Read clipboard: report byte count and whether it matches that marker |
| F4 | **Overwrite primary selection** with `lite-xl primary probe` |
| F5 | Read primary selection: report byte count and marker match |
| F6 | Switch dialog API: rfd ↔ ashpd; opens nothing |
| F8 | Open-file dialog, using the displayed API |
| F9 | Save-file dialog, using the displayed API; no file is written by the probe |
| F10 | Directory dialog, using the displayed API |
| F11 | Report `ashpd::is_sandboxed()` heuristic; not proof of sandbox compatibility |
| F7 | Inherited cursor cycle |
| Escape | Exit when not composing; close any active dialog first, since exiting the probe does not explicitly cancel its portal request |

Use Fn if the keyboard maps F-keys to media controls. One dialog can be outstanding. The displayed API and last result are below the text field; the terminal logs transitions. Each dialog launch also schedules one harmless worker wake: leave the dialog open briefly and look for a worker result with `dialog_pending=true`. This demonstrates continued event processing while the dialog is outstanding. There is no recurring timer/heartbeat busy-spin.

## Live review — keep each backend/session result separate

1. **Loop:** let it settle, press F1, check the status updates. While an actual dialog is open, confirm the pending-dialog worker wake is processed, then that accept/cancel posts a result promptly. A parented/modal dialog should not let typing accidentally edit the parent; after closure the parent must remain usable.
2. **Clipboard both directions:** F2, then paste into another application; it should receive the public clipboard marker. In that application copy a nonprivate sample, then use F3 here (`marker-match=false` is expected for a different sample, not failure).
3. **Primary both directions:** F4, then middle-click into a suitable other application's text field. Select nonprivate text in the other application and use F5 here. Primary availability is a real compositor/protocol check, not inferred from clipboard success.
4. **Both dialog APIs:** for each displayed API, test F8/F9/F10 with accept and cancel, inspect Rust/text/all-file filters, parenting/modality and loop responsiveness. Do not use a real document for the save-name test. If an API fails or opens a second unexpected fallback dialog, report it rather than counting a pass.
5. **After exit:** set F2 and F4 markers, quit the probe, and check whether normal paste/middle-click still work in another application. The probe intentionally runs no helper after exit. A desktop clipboard manager may preserve one selection but not the other; record what happens, never infer persistence from a successful setter.
6. **Sandbox:** F11 reports only the library's cached heuristic. Real sandbox launch, file-access grants and dialog behaviour remain separate pending tests. Setting an environment variable alone is not a sandbox test.

Repeat available checks on forced X11 (label XWayland if that is the actual session). A native X11 session and unavailable checks stay pending unless the user explicitly revises the acceptance gate.

## What is actually being tested

- Main-thread `pump_app_events(Some(Duration::ZERO))` drains a poll iteration, followed by `pump_app_events(Some(5 seconds))` waiting. Native callbacks are handled synchronously, not buffered for later. This tests Linux feasibility of winit's documented non-portable pump API, not full Lua event translation or a production-loop decision.
- Typed proxy events wake the pump from worker threads. Startup schedules one harmless worker wake. `--self-check` additionally schedules a **simulated async-result wake**, not a real file-dialog result. It requires both to interrupt a measurably waiting pump before its timeout, additional frames, and a bounded pump count. Those thresholds are diagnostic assertions, not calibrated performance acceptance.
- A single lazy clipboard worker serializes operations and retains its arboard `Clipboard` while the process lives; blocking clipboard calls do not block the window thread. No clipboard instance is created until an explicit action.
- rfd and ashpd dialog futures run on worker threads with a small Tokio current-thread runtime, their respective parent-handle mechanisms and an `Arc<Window>` retained through the response. This scaffolding selects no production async runtime.
- ashpd classifies cancellation separately from errors. **rfd returns `Option`, so `no-selection(cancel-or-error)` cannot prove clean cancellation.** rfd may fall back to Zenity when its portal path fails; the probe detects its pinned fallback diagnostic and reports `fallback-zenity=true`. Such a result is **not portal evidence**, even if the fallback dialog works. Raw library error text is not emitted because it can contain paths.

## Known limitations inherited from the window probe

The tested Wayland desktop showed transparent-black pixels with documented zero XRGB padding. This probe retains the experimental Wayland-only high-byte `0xff` workaround, preserving RGB channels. It remains unapproved for production and is owned by **Qualify Wayland CPU presentation without unsupported pixel padding**. Successful service tests do not resolve that presentation gap. X11 uses zero padding.

The minimal field supports ordinary typing/IME/Backspace but not left/right caret navigation, selection or a complete editor layout. Its `IME` label mirrors enabled/disabled events, not window focus; the X11 input context can remain enabled while unfocused.

## Automated checks and evidence limits

```bash
bash docs/wayfinder/assets/sdl3-loop-probe/check-loop.sh
```

This runs unit tests and decorated/borderless self-checks on a **private Xvfb display**, with an invalid private D-Bus address. It never exercises the user's desktop clipboard, opens a real dialog or claims native X11/Wayland acceptance. Unit tests cover error/cancel categories, redaction/marker summaries, fallback reporting and the seven inherited geometry/text checks. Private loop checks exercise real proxy dispatch and drawing; they do not prove OS clipboard, actual async portal completion, modality or persistence.

`check-visible.sh` is inherited for an optional **explicit desktop-idle** active-window pixel check (Spectacle/ImageMagick required); do not run it without arranging desktop access. It is not part of the unattended suite.

Live Wayland/X11 clipboard, primary, real dialogs, after-exit ownership, actual sandbox behaviour and runtime dependency traces remain pending until separately recorded. The reused development shell still contains SDL, so it is not evidence of final editor dependency removal. No stack, feature drop, numerical performance tolerance or gate waiver is approved.

## Live finding: rfd could not load libdbus

The first live Wayland F8 attempt with rfd failed in 35 ms: no dialog appeared, rfd fell back to Zenity, and that fallback also failed. This was a failure, not a cancellation. The cause was isolated without opening dialogs: neither of rfd's dlopen names (`libdbus-1.so.3`, `libdbus-1.so`) resolved in the launcher environment, because the repository dev shell does not include dbus. `check-loop.sh` now runs that loader test explicitly; it failed before the change and passes after it. It opens no bus connection.

`env.sh` enters the repository's pinned dev shell **plus `dbus`** (probe-only `--impure` override; the repository flake is unchanged) and adds its library directory. The F-key logs now name the failure stage from rfd's pinned message prefixes (`libdbus-missing`, `session-bus-unavailable`, `portal-call-failed`, `zenity-fallback-failed`, `other-error`) without printing details. Whether rfd's portal call then works is a live check. A shipping editor would need to make libdbus available the same way; that is a packaging fact for selection, not a decision made here.

## Recorded automated run

- All **12 unit tests**, formatting and shell syntax checks pass. Both decorated and borderless private-Xvfb runs passed: 6 pump iterations, 2 blocked-wait interruptions, one worker wake, one simulated-result wake and 3 frames each. Individual wait interruptions were approximately 150–175 ms against the 5-second bound.
- A temporary fault injection dropped proxy wakes: the same self-check rejected it with exit 1 (`waited_wakes=0`, no worker/result events). The injection was removed and both original runs passed again.
- Independent light review passed the loop, service boundaries, lifetimes, reporting and scope checks. Its staging-hygiene finding was fixed: `/target` ignores the local shared-cache symlink as well as directories; no host-specific symlink is committed.
- The Linux normal/build closure has 189 deduplicated package rows and no SDL package; dynamic symbols contain no `SDL_` imports. wayland-backend enables `client_system`/`dlopen`, not the C-logging-shim `log` feature. These are static/private-probe checks, not live service/runtime dependency tracing.
- Cargo.lock SHA-256: `a945885930dbfd9ad8b1c06acc97d21756b9b93bb7de0ed913ee9c8cbb380085`.
- Debug executable SHA-256: `c3058bb65e7a009f0fac8d8dd4f793b6b25534b74a536a0cc373cc11ebf3bd0f` (identity of this local build, not a reproducible-build promise).

## Pinned dependency/API sources

- [winit 0.30.13 pump API](https://docs.rs/winit/0.30.13/winit/platform/pump_events/trait.EventLoopExtPumpEvents.html) and [proxy](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html).
- [arboard 3.7.0](https://docs.rs/arboard/3.7.0/arboard/struct.Clipboard.html), text-only with `wayland-data-control`; [Linux selection API](https://docs.rs/arboard/3.7.0/arboard/trait.SetExtLinux.html).
- [rfd 0.17.2](https://docs.rs/rfd/0.17.2/rfd/struct.AsyncFileDialog.html), defaults disabled, `xdg-portal` + `wayland`; its Linux portal implementation dlopens the desktop's libdbus and can launch Zenity fallback. This is reported, not hidden by the dependency tree.
- [ashpd 0.13.13](https://docs.rs/ashpd/0.13.13/ashpd/desktop/file_chooser/index.html), `file_chooser` + `raw_handle` + `wayland`, default Tokio; [sandbox heuristic](https://docs.rs/ashpd/0.13.13/ashpd/fn.is_sandboxed.html).
- Tokio 1.53.3 (`rt`, `net`, `time`), softbuffer 0.4.8 (no KMS), Skrifa 0.48.0 and Zeno 0.3.3. The pinned Cargo.lock is the exact dependency closure; cached crate source/compiler checks are authoritative over preliminary API advice.
