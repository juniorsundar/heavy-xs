# SDL3 migration Step 4 evidence

**Status: in progress. Dependency/build, font-file opening, font-error transport, glyph-atlas storage, translucent-rectangle blend, Rust draw-view, key-name, owned-input-queue, sandbox/environment and clipboard slices are complete; winit key and click helpers are prepared.** Windows, presentation, input, dialogs and fatal UI still use SDL. This is not evidence of a completed native cutover or permission to waive its acceptance gates.

## Dependency/build checkpoint

Based on `0e4247cc70e51a8c09bf0eb012f446bf500b1db4` on `sdl3/step4-native-cutover`, following the [implementation handoff](sdl3-migration-handoff.md#step-4--complete-native-platform-storage-and-service-cutover).

- Added the selected exact winit, arboard, ashpd and Tokio versions/features. Softbuffer uses upstream git revision `9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`, not released 0.4.8. Tokio includes the selected signal/sync features; no runtime or listener is initialized by these dependency declarations.
- Cargo.lock contains 277 packages (previously 64); every previous name/version pair remains. The added packages include target-specific dependencies which are not all compiled on Linux. Lock SHA-256: `16c7a91895650e7d38c4302457580a6fb79fe8659b89f499f50d08fc5e4c830f`.
- Computed the git-source hash with `fetchgit` from the flake's pinned nixpkgs, using the same URL/revision/defaults as its `importCargoLock`: `sha256-X8gdXyRoFeGo1RhYarenBmW+c11xgu9NuQVo/UmpOmw=`. The subsequent package build successfully consumed it through `cargoLock.outputHashes."softbuffer-0.4.8"`.
- Kept SDL and all existing SDL build inputs. Explicitly named native dlopen providers alongside them; the installed ELF gains their RPATH after Nix's shrink-rpath phase. No wrapper or executable rename. Private D-Bus tooling is in the dev shell only.

### Checks performed

| Check | Result |
| --- | --- |
| `env -u LD_LIBRARY_PATH cargo check --offline` (lock resolution), followed by locked builds below | Pass |
| `nix build --no-link .#lite-xl` | Pass: x86_64 release, ordinary configuration, git dependency vendored through pinned nixpkgs |
| `PROFILE=xvfb scripts/rust-port/verify.sh` | All 18 suites pass; windowed suites report X11 |
| `PROFILE=offscreen scripts/rust-port/verify.sh` | All 18 suites pass; windowed suites report offscreen |
| `scripts/rust-port/startup-check.sh`, dummy and private Xvfb separately | 12/12 each, including isolated child SIGINT, restart and missing-display failure diagnostics |
| `scripts/rust-port/normal-build-check.sh` | 4/4: no hook global/module or strings, retired switch inert, 41 system exports |
| `scripts/rust-port/hit-test-check.sh` | 1,239,848-point recorded C grid agrees |
| `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib` | 17 pass |
| `nix eval --offline --raw .#packages.aarch64-linux.lite-xl.pname` | Evaluates; **not** an aarch64 build |
| Installed ELF RPATH load probe, `env -i` | All nine sonames below load with `RTLD_NOW` |
| `git diff --check` | Pass |
| Read-only `zai/glm-5.3-flash` review of Cargo/Nix diff | No actionable findings |

The isolated C load probe used precisely the installed ELF's RPATH and no environment, opening `libwayland-client.so.0`, `libwayland-cursor.so.0`, `libxkbcommon.so.0`, `libxkbcommon-x11.so.0`, `libX11.so.6`, `libX11-xcb.so.1`, `libxcb.so.1`, `libXcursor.so.1` and `libXi.so.6`. It establishes library resolution only: it creates no display connection, clipboard or native event loop. The SDL contract passes do not establish native-crate runtime behavior.

Built artifact: `/nix/store/aiws994dry2x3rnxkc6zdr6ch8bdn6q5-lite-xl-2.1.7/bin/lite-xl`, SHA-256 `435c3dc23db818e558a99a042cec108dffea4182e8551e5f44b975d7512548db`. Hook-enabled contract binary SHA-256 `4f6a51ecfae4444a8af0386c38c90322791fb1bc5d1fc5281d34c28320270429`. These identify this uncommitted-checkpoint build, not a later rebuilt binary after a commit changes build metadata.

Local, noncanonical logs/provenance: `target/step4-deps/`, `target/step4-deps-*.log`, `target/step4-nix-build.log`, `target/step4-softbuffer-hash.log`. The installed closure contains no Xvfb/xauth/compiler test tools. It still contains SDL's transitive `dbus-1.16.2-lib`; that is distinct from adding private-bus test executables to runtime dependencies.

## Font-file opening slice

Based on `a0743451`, removed the preliminary `SDL_IOFromFile`/`SDL_CloseIO` from `src/font.rs`. The existing libc open/fstat/mmap path now handles empty paths, open errors and directories with the same SDL wording. Native filename bytes still reach `open` unchanged. Descriptor cleanup and mapping lifetime are preserved; a failed fstat still takes the original unknown-format path rather than being reported as a directory. At this checkpoint renderer error transport was still SDL-backed; the following slice removes that carrier.

Checks performed:

- `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib font::`: four tests pass (empty/missing/directory errors and a mapping readable after unlinking a non-UTF-8 filename).
- `scripts/rust-port/verify.sh renderer`, separately with `PROFILE=xvfb` and `PROFILE=offscreen`: both match unchanged historical goldens, including all recorded font-load errors.
- `git diff --check`: pass.
- Bounded read-only `zai/glm-5.3-flash` review: no blockers. The exact strerror assertion uses the current C-locale text, like the historical golden; other locales were not tested.

Local logs/provenance: `target/step4-font/`, `target/step4-font-xvfb.log`, `target/step4-font-offscreen.log`. No full contract, Nix rebuild or desktop test was repeated for this one-file slice.

## Font-error transport slice

Based on `9d69f1de`, `src/renderer.rs` now returns owned `Result<*mut RenFont, String>` errors from font loading/copying instead of writing SDL's last-error state. All callers are in `src/renderer_api.rs` or the copy implementation; the two functions no longer need internal C exports. The Lua/native-plugin interface is unchanged. Load/copy failure prefixes and prior CString handling of interior NULs are retained. The Lua binding copies the message onto its stack and drops the owned CString before deliberately raising an error. Drawing still uses SDL surfaces.

The bounded implementation delegation on `opencode-go/deepseek-v4.1-flash` completed successfully. This establishes that one request worked, not that the earlier intermittent provider failures cannot recur.

Checks performed:

- `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib`: all 23 tests pass, including new direct load-error and copy-after-source-removal tests.
- `scripts/rust-port/verify.sh renderer`, separately on private Xvfb and offscreen: unchanged renderer goldens pass, including font errors, successful copies and groups.
- Repository caller search: no remaining callers with the old signatures; no `SDL_SetError`/`SDL_GetError` calls remain in the affected files.
- `git diff --check`: pass.
- Bounded read-only GLM review: no present correctness blocker; userdata receives its metatable only after successful load/copy, as before.

Logs/provenance: `target/step4-font-errors/` and `target/step4-font-errors-*.log`. The existing invalid-pixel-size branch was preserved but not independently forced; the full contract collection, Nix build and desktop paths were not rerun for this slice.

## Glyph-atlas storage slice

Based on `e03289ae`, glyph atlases in `src/renderer.rs` now use boxed, zero-filled Rust coverage buffers (3 bytes/pixel subpixel, 1 grayscale) instead of SDL `RGB24`/`INDEX8` surfaces. Rows are packed (`width * bytes_per_pixel`) rather than SDL's 4-byte-aligned pitch; all reads and writes use the stored pitch, so placement and blending arithmetic are unchanged. Boxing keeps handed-out coverage pointers stable while an atlas grows; clearing the cache drops them. The internal write-only `bytesize` estimate now counts the Rust structs. The window surface and translucent-rectangle blit remain SDL until the draw-target cutover.

Checks performed:

- `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib`: 23 pass, no warnings.
- `scripts/rust-port/verify.sh renderer` and `rencache`, each on private Xvfb and offscreen: unchanged goldens, including text pixel hashes.
- `git diff --check`: pass. Bounded read-only GLM review: no findings.

Logs: `target/step4-atlas/`, `target/step4-atlas-*.log`. Full contracts, Nix and desktop paths not rerun for this slice.

## Translucent-rectangle blend slice

Based on `f464884d`, translucent `ren_draw_rect` no longer stretches a 1x1 SDL `RGBA32` surface. Stretching one colour yields that colour, so it now blends each clipped destination pixel directly with SDL 3.4.18's `ALPHA_BLEND_CHANNEL` arithmetic (`SDL_blit.h:497`, including its `Uint16` truncation), and destination alpha as `(255, dA, sA)` when the format has an alpha mask. `ren_init`/`ren_free` and the global surface are removed with their `src/app.rs` calls. Opaque fills and the destination window surface remain SDL until the draw-target cutover.

Checks: 23 unit tests pass; `renderer` and `rencache` goldens (translucent hashes at alpha 1, 30-210, 128 and 254) pass on private Xvfb and offscreen; bounded GLM review found no actionable issues. Logs: `target/step4-rect/`, `target/step4-rect-*.log`.

## Rust draw-view slice

Based on `e896a6c7`, drawing no longer calls SDL surface APIs. `RenSurface` is a per-frame Rust view of the window's pixels (pointer, size, pitch, channel masks/shifts, clip, scale), built once per frame from the SDL window surface; that remaining SDL lookup is replaced by the Rust pixel store at the window cutover. The clip lives in the view and is set by rencache's redraw loop, the only drawing path, with `SDL_SetSurfaceClipRect`/`SDL_GetRectIntersection` arithmetic ported (including empty and overflow results). Opaque rectangles fill whole `u32` pixels with `SDL_MapSurfaceRGB`'s 8-bit packing; test hooks read channels through the view. `ren_set_clip_rect` and the `renwin_get_surface` C export are gone. Shortcut, asserted in debug builds: drawing supports only 4-byte, 8-bit-per-channel window formats, as the text blend already required.

Checks: 25 unit tests pass, including two pinning the clip arithmetic; all 18 contract suites pass on private Xvfb and offscreen with unchanged goldens; `normal-build-check.sh` passes; bounded GLM review found no blockers. Logs: `target/step4-view/`, `target/step4-view-*.log`. Nix and desktop paths not rerun.

## Key-name slice

Based on `2bee0f9a`, new `src/keynames.rs` names keys without `SDL_GetKeyName`/`SDL_GetScancodeName`: the editor's NumLock-off keypad table, SDL's six special ASCII names, other ASCII as the character, scancode-masked keys and non-ASCII keycodes by scancode name, ASCII lowercase. The scancode-name table was extracted from SDL 3.4.18 source and compared with all 512 names returned by the linked SDL 3.4.14 (`target/step4-keys/names.c`); the only difference, 3.4.18's added scancode 165 name `Front`, is kept unnamed as in 3.4.14. `src/sysevents.rs` still receives SDL events but now calls the Rust naming.

Reviewed difference by user decision: letter keys are always named by their plain letter; SDL used the layout's shifted character (Turkish dotless-i read `İ`). Recorded in the handoff's reviewed differences. Not observed on a real non-US layout.

Checks: 26 unit tests pass, including the contract's key cases plus out-of-range and scancode-165 cases; `sysevents` goldens pass on private Xvfb and offscreen. Logs: `target/step4-keys/`, `target/step4-keys-*.log`.

## Owned-input-queue slice

Based on `e19ef020`, staged by user decision: `poll_event`/`wait_event` now hand Lua only owned Rust records (`src/platform.rs`), with SDL still the native producer. `sysevents.rs` converts one SDL event per poll into a record, so SDL keeps its queue, poll-cycle sentinel and wait semantics (an earlier draft that drained SDL into the Rust queue changed `wait_event` results after a drained poll cycle; reverted after probing SDL 3.4.14 behaviour, `target/step4-queue/probe*.c`). Motion/touch coalescing pulls same-kind events forward and folds them; focus gain flushes key presses from both queues.

Custom events are registered, numbered (from `0x8000`, never reused, kept across Lua restart) and dispatched in Rust; callbacks take an owned record. Pushes from worker threads still travel through SDL's queue as an integer token naming a Rust-owned record, which keeps ordering relative to native events and wakes `SDL_WaitEvent`. Dialog results and forged test events carry owned payloads: no `SDL_malloc`/`SDL_strdup`/`SDL_free` of event payloads remains. `init_custom_events` can no longer fail, so its startup error path is gone. Shortcut, documented in `platform.rs`: SDL token transport and producer remain until the native pump replaces them; tokens still queued in SDL at exit are freed with the process. The plugin API exposes no SDL, so nothing outside the editor can mint colliding SDL event types.

Checks: 28 unit tests pass (queue order/flush/coalescing and registry numbering/refusals/restart added); all 18 contract suites pass on private Xvfb and offscreen with unchanged goldens; `startup-check.sh` 12/12 on dummy and private Xvfb; `normal-build-check.sh` 4/4; normal and hook builds warning-free; bounded GLM review found no blockers. Logs: `target/step4-queue/`, `target/step4-queue-*.log`.

## Sandbox, environment and platform slice

Based on `733568fb`: `get_sandbox`, `setenv` and the `PLATFORM` global no longer call SDL. `PLATFORM` is the literal `"Linux"` (Linux is the only supported target). `app::run` calls `system::snapshot_environment` where `SDL_Init` used to snapshot the environment: it records only whether `SNAP`, `SNAP_NAME` and `SNAP_REVISION` are present (empty counts, as `SDL_getenv` saw them). `setenv` keeps SDL's rules (empty or `=`-containing names are refused, C-string truncation, always overwrite), sets that snapshot flag first and then calls libc `setenv`, so an external change is invisible to the snapshot exactly as before. `get_sandbox` classifies at its first call with SDL 3.4's Linux priority (`/.flatpak-info`, then all three SNAP variables, then `/run/host/container-manager`, else `none`) and caches the answer for the life of the process, across Lua restarts. No window, portal or Tokio is involved.

Checks: 30 unit tests pass, including priority and snapshot/setter semantics. `startup-check.sh` gained real-process cases: `none` cached across a restart, SNAP set at launch with empty `SNAP` counting, and a `setenv` after the first answer not changing it; 15/15 on private Xvfb and on dummy. All 18 contract suites pass on both profiles with unchanged goldens (the `sandbox`/`setenv` rows included); `normal-build-check.sh` 4/4; bounded GLM review found no blockers. Not tested: a real Flatpak or container-manager filesystem marker (classification of those two inputs is unit-tested only). Logs: `target/step4-env-*.log`.

## Native input helpers (prepared, not yet wired)

`src/keymap.rs` maps winit `PhysicalKey`/`Key` to the SDL 3.4 scancode and unshifted-keycode numbers the editor's existing key naming uses (168 `KeyCode`s checked against the SDL header and the 3.4.14 scancode names; keys SDL has no scancode for map to 0). `src/clicks.rs` ports SDL 3.4.14's multi-click counting (500 ms; the hint value 4 is a per-axis distance, not squared; a press exactly 500 ms later resets; releases do not change state; per-source and per-button state; 255 saturation). Both are pure with unit tests (43 total pass) and are consumed by the winit producer in the cutover commit. Logged questions for that commit: shifted-letter contract case (4, 65) must use the key with Shift applied for the keycode only if SDL's unshifted-keycode contract demands it (SDL keycodes are unshifted, so the cutover passes the key without modifiers); winit `Meta`/`Hyper` map to 0.

## Native clipboard slice

Based on `0969de16`, `src/clipboard.rs` replaces all four SDL clipboard/primary operations in `src/system.rs` with the already-pinned arboard dependency. One lazy main-thread instance survives Lua restarts. Setters retain Lua argument coercion, first-NUL truncation and zero returns; invalid UTF-8 leaves the selection unchanged without lossy replacement. Empty input clears only the requested selection through arboard's selection-specific clear operation. Getters truncate valid text at its first NUL and return exactly one empty string for ordinary unavailable/non-text/conversion failures. API annotations now document these accepted rules. This independent internal checkpoint was taken before the window/event cutover; it does not change the selected clipboard architecture or qualify the rest of Step 4.

The historical system suite did not initialize SDL video, so all its clipboard gets recorded empty strings, even on Xvfb. Arboard can operate without an editor window: the first candidate run instead returned the four values `hello clip`, `multi\\nline`, `12` and `prim`. The user chose **service invariants**, not replacement goldens. `verify.sh` now runs the historical system observations without a display and automatically runs `clipboard-check.sh` whenever the system suite is selected. No golden was changed. The dedicated check runs ordinary editor processes without hooks: unavailable-display return shapes and hidden failures, then private-Xvfb clipboard/primary round trips, independent selection clearing, Unicode/newlines, NUL-before-invalid bytes, invalid-set preservation, numeric/error behavior, ownership across a real Lua restart and fresh-process reads while the owner stays alive. The Xvfb server has no clipboard manager and no connection to the host session; the test does not claim persistence after owner exit.

Checks: 49 unit tests pass; all 18 contract suites pass on private Xvfb and offscreen with unchanged goldens, including the new clipboard invariants; `startup-check.sh` passes 15/15 on private Xvfb; `normal-build-check.sh` passes 4/4, and the ordinary no-hooks binary also passes the dedicated clipboard check. Shell/Lua syntax, `git diff --check` and a bounded read-only review pass. No new dependency or lockfile change. Logs/provenance: `target/step4-clipboard/`, `target/step4-clipboard-*.log`.

Not checked here: native Wayland/XWayland clipboard integration, non-text external owners, externally supplied embedded-NUL text, owner-exit persistence against SDL without a manager, installed-artifact tracing or cost gates. The helper uses arboard's ordinary backend dispatch; these private-Xvfb checks establish X11 selection transfer only. No desktop clipboard or primary selection was touched.

## Remaining work and limits

The coupled main-thread platform owner, persistent draw target, native event conversion/waits, generation-fenced dialogs, process-signal integration and fatal notification remain to implement and validate. The planner's proposed temporary SDL-surface bridge is rejected by the existing pixel-storage resolution. Signal ownership must move with the pump, not as an independently operating or duplicate signal producer.

No live desktop, native Wayland/XWayland presentation/input, IME, selection, chooser, notification or performance check was performed here. Native X11 retains its accepted-unverified disposition; aarch64 was evaluated only. Library resolution and passing SDL tests do not qualify the replacement. Preserve all existing stop conditions, including an observed non-excepted IME/input difference. SDL removal and the final installed-artifact dynamic proof belong to Step 5, not this checkpoint.
