# THROWAWAY: native window, scale and IME probe

Question: can winit 0.30.13 + softbuffer 0.4.8 provide usable CPU pixel presentation, scale changes, IME and borderless move/resize on Linux?

**Pending human review. No production stack selected.** This binary does not load the editor or modify its configuration. Sources and dependencies live only on a throwaway prototype branch. Rendering this test field is not editor rendering parity. The editor loop/pump, clipboard and dialogs belong to a separate prototype.

## Run

From this worktree, one command (builds in the existing Nix development shell):

```sh
bash docs/wayfinder/assets/sdl3-window-probe/run.sh --backend wayland --borderless
```

For the X11 client backend, repeat with `--backend x11`. **On a Wayland desktop this normally tests XWayland, not a native X11 session.** To count as native X11 review, run in a verified X11 desktop session and record that separately. The probe forces the requested winit backend and logs the reported session type; the session variable alone does not prove which server is running.

Omit `--borderless` for the decorated window. `--smoke-seconds 3` exits automatically and fails if no frame was presented. It proves only startup/presentation, not visual or IME correctness. The launcher does not change display scale or switch sessions.

- Green top strip: drag the borderless window with the left button. Blue edges and purple corners: resize. All eight directions should work. These are app-initiated operations after a click, **not** an SDL-style per-point hit-test callback.
- Type in the field. Clicking the field sets the caret to the end, not to the nearest character. Backspace deletes a Unicode scalar; Enter is logged but does not insert a newline. Escape closes when not composing; composition cancellation is left to the IME.
- F7 cycles pointer shapes; click to return to position-dependent cursors.
- Drop a file to inspect drop delivery (only the basename is printed). Touch events are logged if hardware exists.
- IME preedit is gold, committed text is white and the caret is cyan. Cursor byte offsets are handled at UTF-8 boundaries. The IME candidate rectangle follows the caret. A missing-glyph count is visible; tofu boxes are not evidence of successful rendering.
- The launcher optionally uses a local fontconfig CJK match for fallback. `PROBE_FONT` can override it. Font collections use face zero. No font files or discovered font paths are committed. Not all fontconfig matches contain CJK; check the missing-glyph count.

Logs go to the terminal, not to files by default. Keyboard events may contain what you type: use throwaway test text, not passwords or private content. Review logs before sharing. Dropped filenames can also identify personal material.

## Bounded live checklist

Record each item as pass/fail/pending; do not infer one backend's result from the other.

1. At **1×** and **genuine fractional scale** (for example 1.25×), confirm the physical/logical sizes and scale shown. Grid lines are exactly one buffer pixel wide. Resize the window and, where available, move it between differently scaled outputs. Look for blurred/doubled grid lines, stale areas, wrong size, misplaced text and incorrect pointer hit regions. Do not alter environment scale variables to fake physical scale.
2. In borderless mode, drag the green strip and resize all edges/corners. Check pointer shapes. Repeat decorated mode for comparison. Minimise/restore through the desktop and ensure redraw recovers.
3. Type ordinary text and dead-key accents; confirm no missing/duplicate characters. Enable a real IME, compose CJK, move within the preedit, select a candidate and commit/cancel. Check that the popup follows the cyan caret and preedit/commit ordering is logged. If the IME is not installed/configured, record **pending** rather than substituting synthetic events.
4. Check file-manager drop delivery, generated window icon where the desktop supports it, F7 cursors and touch if available. Unsupported/no hardware remains pending.
5. Repeat on native Wayland and native X11. A forced X11 client under Wayland is useful extra XWayland evidence, not the native X11 gate.

## Checks and limits

```sh
cargo test --locked --manifest-path docs/wayfinder/assets/sdl3-window-probe/Cargo.toml
```

Unit tests cover region classification, zero-sized presentation, physical/logical conversion, UTF-8 caret editing and measurement/drawing agreement around missing glyphs. They do not establish compositor behaviour or working IME. Use `cargo tree --locked --target x86_64-unknown-linux-gnu` and `ldd` on the executable for dependency checks; libraries loaded later via dlopen need a separate runtime check. The crate's name contains `sdl3` only to identify the experiment, not an SDL dependency.

### Automated validation of this snapshot

- Debug build, `cargo fmt --check`, launcher `bash -n` and all **5 unit tests passed**.
- Launcher smoke checks passed on a private **Xvfb** server for decorated and borderless windows: physical 800×520, scale 1.0, at least one successful present before a three-second timeout. This does not verify native X11-session window management, visual correctness or IME.
- Missing-backend and unavailable-Wayland startup checks returned nonzero as expected.
- Linux normal/build dependency tree has 112 deduplicated rows and no SDL package. `cc` is a build dependency of wayland-backend 0.3.17, but that crate only compiles its C logging shims with the `log` feature; selected features are `client_system`, `default` and `dlopen`, not `log`.
- `ldd` lists libgcc_s, libm, libc and the Linux loader; no dynamic `SDL_` symbols were found. Xvfb launcher traces showed X11/XCB/xkbcommon libraries opening at runtime and no SDL library loads. Wayland runtime loading remains untested. The reused development shell still includes SDL; this is not proof that the final editor build environment is SDL-free.
- Independent light review passed. Its missing-glyph measurement drift finding was fixed and covered by a unit test. Live keyboard/IME duplicate-or-missing input must still be checked.
- Cargo.lock SHA-256: `25d736f989c7f615073a59355bf0f05f968291077217ebf6bac6db48722d2c85`. Debug executable SHA-256: `a1c021a2848a2c8553b5e1cc06f1c078db5ee9456e5f807baa89f08c861d23fb` (identity of the tested executable, not a reproducible-build promise).

**All human desktop checklist items remain pending.** No framebuffer images or raw event logs are included in the branch.

Limitations: no shaping/bidi/grapheme editor; a single-line field capped at 512 committed UTF-8 bytes; no text scrolling; helper glyphs are rasterized afresh on redraw (not a performance benchmark); very long text can extend past the field; no opacity API exists in winit 0.30.13 (transparency is not equivalent); no claim of whole-editor acceptance, native X11 coverage from Xvfb/XWayland, or testing unavailable input hardware. These limitations are not proposed feature losses for the editor.
