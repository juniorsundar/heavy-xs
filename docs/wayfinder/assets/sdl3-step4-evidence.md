# SDL3 migration Step 4 evidence

**Status: in progress. The native window/storage/event/signal core and ashpd file dialogs are implemented; clipboard and the earlier independent slices are complete.** SDL remains for startup initialization and fatal UI. This is an internal staged checkpoint, not a completed native-platform migration or permission to waive its desktop/performance/failure-path acceptance gates.

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

## Native input helpers (preparatory checkpoint; now wired below)

`src/keymap.rs` maps winit `PhysicalKey`/`Key` to the SDL 3.4 scancode and unshifted-keycode numbers the editor's existing key naming uses (168 `KeyCode`s checked against the SDL header and the 3.4.14 scancode names; keys SDL has no scancode for map to 0). `src/clicks.rs` ports SDL 3.4.14's multi-click counting (500 ms; the hint value 4 is a per-axis distance, not squared; a press exactly 500 ms later resets; releases do not change state; per-source and per-button state; 255 saturation). Both are pure with unit tests (43 total pass) and are consumed by the winit producer in the cutover commit. Logged questions for that commit: shifted-letter contract case (4, 65) must use the key with Shift applied for the keycode only if SDL's unshifted-keycode contract demands it (SDL keycodes are unshifted, so the cutover passes the key without modifiers); winit `Meta`/`Hyper` map to 0.

### Native key-event adapter (preparatory checkpoint; now wired below)

Based on `31b6a5ee`, `keymap::key_event_codes` now combines a winit physical key, location, unmodified key and effective logical key into the existing `(scancode, keycode, numlock)` fields. The key-only helper could not preserve keypad identity or left/right modifier sides: keypad Enter/operators/digits now use their physical SDL keypad code, generic right-side modifiers get their right-side code, and AltGraph is not shifted twice. Ordinary layout-derived keys retain their unmodified character mapping.

Source inspection of the pinned winit showed that `key_without_modifiers` always uses XKB level zero and the public modifier state omits NumLock. The user chose **effective keypad mapping**, not a new library/dependency/lock-state query: each event's character/navigation result determines keypad naming. This matches standard PC keypad types; unusual XKB keypad variants can differ from SDL's actual-lock-bit rule. This sixth reviewed difference is recorded in the handoff. No guessed initial state or keypress-toggle cache is introduced.

Checks: 55 unit tests pass (11 focused keymap tests), covering all eleven digit/decimal keys both ways, winit's unmodified-navigation/effective-digit combination, unidentified keypad 5 while navigating, keypad operators/Enter/equal/comma, left/right modifiers including AltGraph, Shift/Caps/layout remaps, unknown physical keys and ordinary navigation. `sysevents` goldens pass unchanged on private Xvfb and offscreen; ordinary build and `git diff --check` pass; bounded read-only review found no blockers. Logs: `target/step4-key-events-*.log`, `target/step4-key-events/`.

At this preparatory checkpoint the adapter was **not wired to production input yet**. No native window/key capture, unusual-layout desktop test, full contract collection or Nix rebuild was claimed for that slice. The coupled native-core checkpoint below wires it.

## Native clipboard slice

Based on `0969de16`, `src/clipboard.rs` replaces all four SDL clipboard/primary operations in `src/system.rs` with the already-pinned arboard dependency. One lazy main-thread instance survives Lua restarts. Setters retain Lua argument coercion, first-NUL truncation and zero returns; invalid UTF-8 leaves the selection unchanged without lossy replacement. Empty input clears only the requested selection through arboard's selection-specific clear operation. Getters truncate valid text at its first NUL and return exactly one empty string for ordinary unavailable/non-text/conversion failures. API annotations now document these accepted rules. This independent internal checkpoint was taken before the window/event cutover; it does not change the selected clipboard architecture or qualify the rest of Step 4.

The historical system suite did not initialize SDL video, so all its clipboard gets recorded empty strings, even on Xvfb. Arboard can operate without an editor window: the first candidate run instead returned the four values `hello clip`, `multi\\nline`, `12` and `prim`. The user chose **service invariants**, not replacement goldens. `verify.sh` now runs the historical system observations without a display and automatically runs `clipboard-check.sh` whenever the system suite is selected. No golden was changed. The dedicated check runs ordinary editor processes without hooks: unavailable-display return shapes and hidden failures, then private-Xvfb clipboard/primary round trips, independent selection clearing, Unicode/newlines, NUL-before-invalid bytes, invalid-set preservation, numeric/error behavior, ownership across a real Lua restart and fresh-process reads while the owner stays alive. The Xvfb server has no clipboard manager and no connection to the host session; the test does not claim persistence after owner exit.

Checks: 49 unit tests pass; all 18 contract suites pass on private Xvfb and offscreen with unchanged goldens, including the new clipboard invariants; `startup-check.sh` passes 15/15 on private Xvfb; `normal-build-check.sh` passes 4/4, and the ordinary no-hooks binary also passes the dedicated clipboard check. Shell/Lua syntax, `git diff --check` and a bounded read-only review pass. No new dependency or lockfile change. Logs/provenance: `target/step4-clipboard/`, `target/step4-clipboard-*.log`.

Not checked here: native Wayland/XWayland clipboard integration, non-text external owners, externally supplied embedded-NUL text, owner-exit persistence against SDL without a manager, installed-artifact tracing or cost gates. The helper uses arboard's ordinary backend dispatch; these private-Xvfb checks establish X11 selection transfer only. No desktop clipboard or primary selection was touched.

## Coupled native window/storage/event/signal core

Based on `10af6ce5`, this internal checkpoint replaces SDL window ownership, surfaces, event pumping and signal-to-quit delivery together. The agreed later dialog/fatal/startup cleanup remains separate; it does not ship an SDL-surface bridge or a second signal producer.

- `src/native.rs` owns the lazy, process-lived winit loop and id-based window registry on the main thread. Window creation is staged into `resumed`/`about_to_wait`; the owner and its event loop are taken out of their cell before pumping, so callbacks never reborrow the global owner or enter Lua. Failed loop creation is retained. Window functions, cursors, hit zones and text-input requests use the native owner; compositor drag/resize starts only in a live left-button callback. Wayland app ID stays `com.lite_xl.LiteXL`. Whole-window opacity is the accepted no-op; no matching configuration/UI was present to remove.
- `RenWindow` owns tightly packed `0xFFRRGGBB` pixels and the existing command buffer. Drawing/cache/test reads all use this store. A present copies every visible row into a fresh softbuffer buffer, honoring its stride and not relying on buffer age. Wayland uses the pinned supported Premultiplied/opaque-ARGB route; X11 uses Ignored alpha. Zero/minimized presentation is skipped. Each window is shown after its first actual frame; a forced foreground refresh does not accidentally show a hidden window. Existing RGB goldens remain unchanged.
- `src/native_input.rs` converts winit keys, text/IME, pointer, wheel, touch and drop records. It uses the prepared key/click helpers, preserves native filename bytes, converts preedit byte offsets to character counts, suppresses ordinary Ctrl/Alt text without suppressing AltGraph, and uses native implicit dragging rather than cursor confinement. No timed text-deduplication heuristic is used. Hardware-specific behavior is not inferred from the unit or Xvfb checks.
- `src/platform.rs` owns the queue, proxy wake target, display-free Tokio notifier, generation fence and one coalesced process quit request. Publication precedes wake and notifier registration precedes predicate rechecking. A monotonic deadline spans internal wakes. Only consumption clears the quit; filtering/restart does not. Old dialog results are rejected under the same lock as generation/closure checks. The custom-event integer-token transport and all SDL polling/pushing are gone. Hook builds decode historical numeric fixtures into this same queue; that path is not OS-input evidence.
- `src/services.rs` supplies the single cached one-worker runtime. Startup synchronously inspects dispositions and registers only default SIGINT/SIGTERM before SDL initialization, leaving ignored/custom handlers alone. Tasks publish Rust state, never Lua or process termination. Runtime/setup errors fail before Lua; live errors wake the ordinary Lua error path. Generation retirement drains old native input and disables old text composition before queue retirement. Both normal teardown and the private Lua bootstrap `os.exit` wrapper close delivery before Lua destruction, without waiting for signal tasks or a dialog.
- SDL dialogs temporarily have no SDL parent and their callbacks now carry a generation fence. This is explicitly not the final parent/modality implementation. SDL fatal UI and initialization remain. `renderer.rs` has no SDL dependency left. Cargo binaries obtain the same dlopen-provider RPATHs already retained by Nix; no dependency/version/lock change was needed.

### Reviewed wait-readiness difference

The first native run passed 17/18 suites. The remaining two observations expected SDL's poll-cycle readiness after the actual queue was drained. The user chose **actual pending input** rather than reproducing that sentinel. This seventh reviewed difference is recorded in the handoff and API docs. The string-timeout/extra-argument cases now enqueue real input to test argument behavior; timeout, non-consuming readiness and empty-queue behavior retain separate checks. No golden was edited. An earlier foreground-refresh mismatch was fixed at its root (refresh must not show a hidden window), not normalized away.

### Checks performed

| Check | Result |
| --- | --- |
| `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib` | 63 pass; one ignored subprocess subject is explicitly invoked in six fresh children by its parent test (INT/TERM × default/ignored/custom). Includes queue ordering/coalescing, quit survival/consumption, generation rejection, notifier error wake, spurious-wake deadline and final-close assertions. |
| `PROFILE=xvfb scripts/rust-port/verify.sh`, debug and `CARGO_ARGS=--release` | All 18 suites pass in both builds; unchanged historical RGB/event goldens, native X11 backend tags, and private clipboard invariants. Offscreen/dummy window profiles are retired. |
| `scripts/rust-port/startup-check.sh`, debug and release | 19/19 in both builds: real SIGINT and SIGTERM separately reach a running full editor, exit cleanly without error.txt or update/draw after project removal; restart/global/environment and real missing-display Lua traceback/error.txt/exit-1 checks pass. |
| `scripts/rust-port/native-check.sh` | 14/14: SIGINT and SIGTERM wake actual indefinite waits before a loop, after failed loop creation and with a native loop; requests survive a real Lua restart without consumption or duplicate delivery. XTEST drives actual X11 capture through winit: plain/shifted text, right Ctrl, suppressed Ctrl+S text, NumLock-off navigation/on keypad digit, keypad Enter. Uses no injection hook; also passes 14/14 in release. |
| `scripts/rust-port/normal-build-check.sh` | 4/4: hook code/global/module absent, retired switch inert, 41 system exports. Native checks also pass 14/14 with this ordinary no-hooks binary. |
| `scripts/rust-port/hit-test-check.sh` | All 1,239,848 recorded C points agree. |
| `nix build --no-link --print-out-paths .#lite-xl` | Pass; installed release ELF passes native checks 14/14 and private clipboard checks 10/10 against staged test data with host display/bus/loader paths removed. |
| Shell/Lua syntax; `git diff --check`; bounded independent reviews | Pass; no blockers. Reviews prompted the app-ID fix and an additional focused review covered final teardown/argument-lifetime/test-isolation changes. |

Installed checkpoint ELF: `/nix/store/y6wzhiymnl8bydp8grpbyisvh27babwd-lite-xl-2.1.7/bin/lite-xl`, SHA-256 `b5be6245eee83d965bbf4adb5178343c39a5d3deadc941b5d76c95d239c03970`. Built from the uncommitted checkpoint based on `10af6ce5`, before final documentation/test-only changes; not a final migration artifact. Cargo.lock SHA-256 remains `16c7a91895650e7d38c4302457580a6fb79fe8659b89f499f50d08fc5e4c830f`. Its NEEDED list still includes `libSDL3.so.0`, as expected. This is runtime-provider/OS-path evidence, not the Step 5 no-SDL dynamic audit. Logs/provenance: `target/step4-native/`, `target/step4-native-*.log`.

All runtime checks used runner-owned Xvfb or no display, an unreachable session bus and private runtime directories. No host window, clipboard, portal or notification was used. XTEST is simulated input through the real X server, not physical keyboard or native-Wayland evidence.

## Native portal-dialog checkpoint

Based on `1f993e8b`, `src/dialogs.rs` replaces all SDL file-dialog requests/callbacks in `src/sysplugin.rs` with the already-pinned ashpd 0.13.13. No new Rust dependency, runtime, public export, helper process, fallback or single-pending restriction was added.

- Each call retains its own token-keyed main-thread parent lease and Lua generation independently of the unchanged caller id. A short-lived request thread acquires borrowed raw handles and constructs/awaits its parent identifier on that same thread, using the existing process runtime. Modality, Open/Save/Directory distinctions, ordered filters, location, custom Accept text and immediate zero-value returns are retained. Cancel text remains portal-controlled as already accepted.
- Lua option values are checked and copied into plain Lua snapshot tables before any Rust owner exists, so bad arguments/metamethod errors cannot skip Rust destructors. Metamethods are evaluated once. C-string fields stop at the first NUL, matching SDL's property boundary; this also corrects the former Rust `CString::new(...).unwrap_or_default()` behavior that incorrectly emptied an entire optional string on an embedded NUL. File paths stay native bytes; an unrepresentable UTF-8 suggestion uses byte-valued `current_file`, not replacement text or a filename rejection. D-Bus text-only labels still require valid UTF-8.
- Parent closure hides and removes the window from live input routing, dropping its draw resources but not its leased native handle. Request completion first drops its export and worker Arc, then returns the token through an internal proxy event; the main thread releases the last Arc. Retired-generation results are discarded but their leases are still released. Release events alone do not add public readiness or Lua events. Final exit closes delivery and does not join unanswered requests. Main-thread TLS teardown deliberately retains any still-leased display/window resources until process termination; this is exit-only retention, not steady-state request leakage.
- Pump unwinding restores the event loop/owner before propagating the panic, so an outstanding chooser cannot lose its borrowed display during unwinding. A cfg(test)-only private-Xvfb fault subject verifies this ownership path; it is not a driver-recovery feature or a production scripting override.
- Portal responses 1 and 2, and successful empty selections, cancel. Transport/export/setup/worker/URI-conversion failures produce the existing recoverable error outcome. Selected URIs are converted atomically without lossy filenames or partial success; malformed escapes, NULs and non-local schemes/hosts fail. Native hostname/localhost matching and SDL's valid-path slash quirks are retained. Eight safe URI cases were also compared with a standalone extraction of SDL 3.4.18's `SDL_URIToLocal`/`SDL_URIDecode`, including the explicit-local-host extra-slash quirk; these are source-helper checks, not evidence about a newer linked SDL version or a real chooser.

### Dialog diagnostic decision

The user selected **assert invariants; keep goldens** when the six historical unavailable-dialog observations changed from SDL driver-list text to a real portal connection failure. The legacy fixture now asserts the actual id/status and the expected non-empty unavailable-bus diagnostic before normalizing only its library-specific message to the historical observation. All goldens remain untouched. Runtime error messages retain the real details; they do not pretend to be SDL errors.

### Dialog checks

`dialog-check.sh` exercises real ashpd request serialization, cached D-Bus connection reuse, worker delivery and native parent leases against `tests/contract/portal.rs` on a runner-owned bus with **no activatable service directories**, plus a private Xvfb. It never contacts the user's bus/desktop or opens a real chooser. Its lib-test fixture is built separately without overwriting the verified application ELF; `jq` is a test-only dev-shell tool used to select the exact Cargo artifact.

The eight runner checks include the callback-unwind ownership fault; real X11 visibility/retention/destruction across parent GC; accept/cancel-1/cancel-2/empty/error outcomes; non-UTF-8 selected/default filename bytes; no partial URI acceptance; filter/Accept/location/modality serialization; one-time option metamethods; concurrent duplicate ids; stale-generation completion with caller-id reuse; and bounded process exit with an unanswered chooser. Ordinary debug, release and installed-artifact runs are recorded with the checkpoint validation. `verify.sh` automatically runs them whenever `sysplugin` is selected, so its legacy diagnostic normalization cannot substitute for service-path evidence.

### Dialog checkpoint validation and identity

- Library tests: **66 pass**. Three ignored subprocess subjects are invoked explicitly: the six signal-disposition child cases by their parent test, and the private portal plus unwind fault subjects by `dialog-check.sh`. Generic `cargo test -- --ignored` cannot accidentally start these subjects without their test-only isolation roles.
- Debug and release: **18/18 contract suites**, unchanged goldens, **10/10 clipboard checks**, **8/8 scripted dialog checks** in each full verification run. Startup **19/19** in both builds; release native OS-input/signal checks **14/14**.
- Ordinary no-hooks build: **4/4** absence/API checks and **8/8** scripted dialog checks. No test fixture or fault knob is compiled into this application.
- Nix release build passes. Its installed ELF passes **8/8** dialog, **14/14** native and **10/10** clipboard checks with staged test data, private displays/buses, and no inherited loader paths. It still needs `libSDL3.so.0` for the remaining startup/fatal slice. Bounded independent reviews found no blockers; this is not real desktop/chooser or performance qualification.

Installed checkpoint ELF: `/nix/store/f1rki3vrz6lmn9x5b28knqs4lw23v9i1-lite-xl-2.1.7/bin/lite-xl`; SHA-256 `2c93e958b235ca1b33b8f277c398a8419f1a9237408af8868ab57d4b7ebf28cf`. Built from the uncommitted checkpoint based on `1f993e8b`, before final evidence/test-isolation changes. Cargo.lock remains unchanged at SHA-256 `16c7a91895650e7d38c4302457580a6fb79fe8659b89f499f50d08fc5e4c830f`. Logs/provenance: `target/dialog-native/`, `target/dialog-*.log`; the standalone C helper check is scratch under `target/`, not installed or committed.

## Remaining work and limits

Implement native fatal notification and remove residual SDL initialization. Complete native desktop/presentation/IME/fractional-scale/held-drag/touch/drop/real-portal acceptance and calibrated cost gates. Required runtime/registration, spawn/export and live listener/driver failure injection and deterministic proxy-publication races are not fully qualified; the callback-unwind fault and scripted portal failures do not cover every failure. Do not treat this internal checkpoint as a passed full Step 4. The planner's proposed temporary SDL-surface bridge is rejected by the existing pixel-storage resolution. Signal ownership must move with the pump, not as an independently operating or duplicate signal producer.

No live-desktop Wayland/XWayland presentation/input, IME, selection, chooser, notification or performance check was performed here; the private-Xvfb evidence is listed above. Native X11-session hardware retains its accepted-unverified disposition; aarch64 was evaluated only. Passing private tests and resolving libraries do not qualify the replacement. Preserve all existing stop conditions, including an observed non-excepted IME/input difference. SDL removal and the final installed-artifact dynamic proof belong to Step 5, not this checkpoint.
