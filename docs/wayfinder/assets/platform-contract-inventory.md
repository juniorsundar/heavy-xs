# Observable SDL3 platform contract inventory

Evidence for [Inventory the observable platform contract](../issues/24-inventory-platform-contract.md). This describes the current SDL3-backed layer as Lua, plugins and the tests observe it. It selects no replacement, approves no compatibility loss and changes no production code.

## Identity and validation

Source revision `f52aa27355f6b3c08703d3f23f06cd2538024a14`. Only `docs/wayfinder/*` differ from it in the working tree; the tracked `src/`, `data/`, `tests/` and fallback expectations are unchanged, and no baseline was rewritten during this work.

| Artifact | Identity |
|---|---|
| HEAD commit | `f52aa27355f6b3c08703d3f23f06cd2538024a14` |
| Cargo.lock SHA-256 | `1a2b1485592910d3bc588c04d8a7f12f0d285b9997a6cab91eac66a718f72a35` |
| `sdl3-sys` ([Cargo.lock:339](../../../Cargo.lock#L339)) | `0.7.2+SDL-3.4.18` (crate version and the SDL header version it was generated from) |
| System SDL3 | **3.4.14** from `pkg-config --modversion sdl3` inside `nix develop` ([flake.nix](../../../flake.nix)) |

The bindings' `+SDL-3.4.18` suffix is not the linked library: the build links the system libsdl3 3.4.14. Anything the 3.4.18 bindings expose but 3.4.14 lacks would be a build-time gap, not a runtime one.

**Evidence levels.** Every behaviour below is source-read unless a fixture, probe or script is named in its row. On this revision:

- `scripts/rust-port/verify.sh` ran to completion on the default target and stage (no environment overrides) in under five minutes: **15 of 16 suites OK** — `customevents`, `dirmonitor`, `fuzzy`, `pluginapi`, `regex`, `rencache`, `renderer`, `smoke`, `sysevents`, `sysplugin`, `system`, `syswindow`, `tokenizer`, `utf8_edge`, `utf8`; `process` failed one timing assertion (`interrupt then dead`: expected `true`, observed `false`). That suite covers the `process` module, not the platform layer, and the failure was not investigated or fixed.
- No real window was opened, no compositor ran, and **no scale ≠ 1, Wayland, X11, IME, touch or drag-and-drop run was performed**. Everything in this inventory that needs a real window is marked so in Coverage gaps.
- Every relative link and source anchor in this file was resolved against the working tree.

## The observable surface

`app::api_load_libs` registers `system` and `renwindow` as Lua libraries ([app.rs:65](../../../src/app.rs#L65)); `system` is also a global. Counts on this revision:

| Surface | Count | Source of truth |
|---|---|---|
| `system.*` functions | **41** | [baseline export line](../../../scripts/rust-port/baseline/system.txt#L2), [overlay](../../../src/system.rs#L509) |
| `renwindow.*` functions | **4** (`create`, `get_size`, `_persist`, `_restore`) plus `__gc` | [renwindow_api.rs:130](../../../src/renwindow_api.rs#L130) |
| `system.poll_event` event names | **26** (25 reach Lua; `dirmonitor` is swallowed) | [sysevents.rs:95](../../../src/sysevents.rs#L95) |
| `SDL_SetHint` calls | **8** | [renderer.rs:992](../../../src/renderer.rs#L992) |

## `system`: filesystem, process, time, clipboard, matching

All of these are mlua functions in `src/system.rs`; argument checking mimics `luaL_checkstring`/`luaL_checknumber` (numbers are string-coercible, embedded NUL truncates a C string, wrong type raises `bad argument #n to '<name>'`).

| Lua-visible signature | Behaviour and units | Raises | SDL / libc behind it | Source | Coverage |
|---|---|---|---|---|---|
| `get_file_info(path) -> {size,type?,modified,symlink?}` or `nil,msg` | `size` bytes, `modified` seconds with nanoseconds, `type` `"file"`/`"dir"` else absent, `symlink` only set for a directory that is a symlink | no (error tuple) | `libc::stat`/`lstat`, `strerror` | [system.rs:89](../../../src/system.rs#L89) | [system.lua:62](../../../tests/contract/cases/system.lua#L62) |
| `absolute_path(path) -> string?` | `realpath`; **zero values** (not `nil`) on failure, so `if x == nil` misses it | no | `libc::realpath` | [system.rs:120](../../../src/system.rs#L120) | [system.lua:95](../../../tests/contract/cases/system.lua#L95) |
| `get_fs_type(path) -> string` | statfs magic table (`ext2/ext3`, `nfs`, `fuse`, `smb`, `smb2`, `reiserfs`, `tmpfs`, `ramfs`, `ntfs`), else `"unknown"` | yes on statfs failure | `libc::statfs` | [system.rs:133](../../../src/system.rs#L133) | [system.lua:151](../../../tests/contract/cases/system.lua#L151) |
| `list_dir(path) -> table` or `nil,msg` | 1-based array of names, in `SDL_EnumerateDirectory` order | no | `SDL_EnumerateDirectory` | [system.rs:165](../../../src/system.rs#L165) | [system.lua:137](../../../tests/contract/cases/system.lua#L137) |
| `mkdir(path) -> bool, msg?` | mode 0755 | no | `libc::mkdir` | [system.rs:194](../../../src/system.rs#L194) | [system.lua:117](../../../tests/contract/cases/system.lua#L117) |
| `rmdir(path) -> bool, msg?` | **`SDL_RemovePath`, so it deletes regular files and empty directories alike**; a non-empty directory fails with SDL's message | no | `SDL_RemovePath` | [system.rs:205](../../../src/system.rs#L205) | [system.lua:117](../../../tests/contract/cases/system.lua#L117) |
| `chdir(path)` | no return value | yes, `chdir() failed: <strerror>` | `libc::chdir` | [system.rs:215](../../../src/system.rs#L215) | [system.lua:107](../../../tests/contract/cases/system.lua#L107) |
| `ftruncate(file, length?) -> bool, msg?` | `length` defaults to 0; negative passes through to `ftruncate(2)`. Reads the `FILE*` out of Lua's file userdata via `luaL_testudata(L, 1, "FILE*")` | yes on a non-file argument (deviation: the C crashes on a closed handle) | `libc::ftruncate`/`fileno` | [system.rs:225](../../../src/system.rs#L225) | [system.lua:159](../../../tests/contract/cases/system.lua#L159) |
| `get_process_id() -> integer` | `std::process::id()` | — | none | [system.rs:275](../../../src/system.rs#L275) | [system.lua:185](../../../tests/contract/cases/system.lua#L185) |
| `get_time() -> number` | **seconds, floating point**, from `SDL_GetPerformanceCounter()/SDL_GetPerformanceFrequency()`; monotonic, unrelated to wall clock | — | performance counter | [system.rs:279](../../../src/system.rs#L279) | [system.lua:185](../../../tests/contract/cases/system.lua#L185) |
| `sleep(seconds)` | `SDL_Delay(seconds*1000)` truncated to whole milliseconds; negatives clamp to 0 | yes on non-number | `SDL_Delay` | [system.rs:284](../../../src/system.rs#L284) | [system.lua:185](../../../tests/contract/cases/system.lua#L185) |
| `exec(command)` | appends `" &"` and calls `system(3)`; **never waits, never reports failure** | yes on non-string | `libc::system` | [system.rs:292](../../../src/system.rs#L292) | [system.lua:201](../../../tests/contract/cases/system.lua#L201) |
| `setenv(key, value) -> bool` | always overwrites; `""` key and `"="-in-key` fail (SDLError path) | yes on missing arg | `SDL_setenv_unsafe` | [system.rs:302](../../../src/system.rs#L302) | [system.lua:214](../../../tests/contract/cases/system.lua#L214) |
| `get_clipboard() -> string?` | zero values when SDL has no text; text copied out of SDL's allocation | — | `SDL_GetClipboardText` + `SDL_free` | [system.rs:332](../../../src/system.rs#L332) | [system.lua:230](../../../tests/contract/cases/system.lua#L230) |
| `set_clipboard(text)` | NUL-truncated copy; return value ignored, so an SDL failure is silent | yes on non-string | `SDL_SetClipboardText` | [system.rs:339](../../../src/system.rs#L339) | [system.lua:230](../../../tests/contract/cases/system.lua#L230) |
| `get_primary_selection() -> string?` | same shape as `get_clipboard` | — | `SDL_GetPrimarySelectionText` | [system.rs:345](../../../src/system.rs#L345) | [system.lua:230](../../../tests/contract/cases/system.lua#L230) |
| `set_primary_selection(text)` | same as `set_clipboard` | yes on non-string | `SDL_SetPrimarySelectionText` | [system.rs:352](../../../src/system.rs#L352) | [system.lua:230](../../../tests/contract/cases/system.lua#L230) |
| `fuzzy_match(haystack, needle, file?) -> integer?` | case-insensitive subsequence score; zero values for no match; `file` truthy reverses the walk | yes on missing arg | none (pure) | [system.rs:362](../../../src/system.rs#L362) | [fuzzy.lua](../../../tests/contract/cases/fuzzy.lua#L1) |
| `path_compare(p1,t1,p2,t2) -> bool` | directories first, numeric runs compared as integers, `/` as terminator; `t` compared with `strcmp(type,"dir")` | yes on missing arg | none (pure) | [system.rs:402](../../../src/system.rs#L402) | [system.lua:266](../../../tests/contract/cases/system.lua#L266) |
| `get_sandbox() -> string` | `"none"`, `"unknown"`, `"flatpak"`, `"snap"`, `"macos"`; anything else maps to `"unknown"` | — | `SDL_GetSandbox` | [system.rs:308](../../../src/system.rs#L308) | [system.lua:214](../../../tests/contract/cases/system.lua#L214) |

`get_sandbox` is load-bearing for the UI: `config.use_system_file_picker = system.get_sandbox() ~= "none"` ([config.lua:266](../../../data/core/config.lua#L266)), i.e. inside a sandbox the editor enables the system file picker instead of the command-palette picker.

## `system`: window, cursor, text input, message box

All fifteen are raw `lua_CFunction`s in `src/syswindow.rs` taking a `RenWindow` userdata first; argument checking uses real `luaL_check*`, so type errors read exactly as the C's. Coordinates and sizes here are **window points** (`SDL_GetWindowSize`/`SDL_SetWindowSize`), never surface pixels.

| Lua-visible signature | Behaviour and units | Raises | SDL behind it | Source | Coverage |
|---|---|---|---|---|---|
| `set_cursor(kind?)` | option `"arrow"` (default), `"ibeam"`, `"sizeh"`, `"sizev"`, `"hand"`; one cached `SDL_Cursor` per option, never freed | yes on unknown option | `SDL_CreateSystemCursor`/`SDL_SetCursor` | [syswindow.rs:44](../../../src/syswindow.rs#L44) | [syswindow.lua:123](../../../tests/contract/cases/syswindow.lua#L123) |
| `set_window_title(window, title)` | string copied by SDL | yes on non-string | `SDL_SetWindowTitle` | [syswindow.rs:76](../../../src/syswindow.rs#L76) | [syswindow.lua:76](../../../tests/contract/cases/syswindow.lua#L76) |
| `set_window_mode(window, mode?)` | option `"normal"` (default), `"minimized"`, `"maximized"`, `"fullscreen"`; fullscreen is a boolean set, the other three call Restore/Maximize/Minimize | yes on unknown option | `SDL_SetWindowFullscreen`, `SDL_RestoreWindow`, `SDL_MaximizeWindow`, `SDL_MinimizeWindow` | [syswindow.rs:83](../../../src/syswindow.rs#L83) | [syswindow.lua:37](../../../tests/contract/cases/syswindow.lua#L37) |
| `get_window_mode(window) -> string` | window flags in the order fullscreen → minimized → maximized → `"normal"` | — | `SDL_GetWindowFlags` | [syswindow.rs:111](../../../src/syswindow.rs#L111) | [syswindow.lua:37](../../../tests/contract/cases/syswindow.lua#L37) |
| `set_window_bordered(window, bool)` | truthiness decides | — | `SDL_SetWindowBordered` | [syswindow.rs:126](../../../src/syswindow.rs#L126) | [syswindow.lua:76](../../../tests/contract/cases/syswindow.lua#L76) |
| `get_window_size(window) -> w,h,x,y` | **points**; four integers, position included | — | `SDL_GetWindowSize`, `SDL_GetWindowPosition` | [syswindow.rs:132](../../../src/syswindow.rs#L132) | [sysevents.lua:149](../../../tests/contract/cases/sysevents.lua#L149) |
| `set_window_size(window, w,h,x,y)` | numbers truncated to int; then `ren_resize_window` refreshes the surface and the scale factors | yes on non-number | `SDL_SetWindowSize`, `SDL_SetWindowPosition` | [syswindow.rs:144](../../../src/syswindow.rs#L144) | [syswindow.lua:56](../../../tests/contract/cases/syswindow.lua#L56) |
| `window_has_focus(window) -> bool` | `SDL_WINDOW_INPUT_FOCUS` bit only | — | `SDL_GetWindowFlags` | [syswindow.rs:156](../../../src/syswindow.rs#L156) | [syswindow.lua:28](../../../tests/contract/cases/syswindow.lua#L28) |
| `raise_window(window)` | no return value | — | `SDL_RaiseWindow` | [syswindow.rs:162](../../../src/syswindow.rs#L162) | [syswindow.lua:76](../../../tests/contract/cases/syswindow.lua#L76) |
| `set_window_opacity(window, opacity) -> bool` | `0.0..1.0`; **always returns `true`** — left over from SDL2's `0/-1` return, so an SDL failure is reported as success | yes on non-number | `SDL_SetWindowOpacity` | [syswindow.rs:167](../../../src/syswindow.rs#L167) | [syswindow.lua:76](../../../tests/contract/cases/syswindow.lua#L76) |
| `set_window_hit_test(window, title_height?, controls_width?, resize_border?)` | with extra args calls `SDL_SetWindowHitTest` with a callback that classifies using the window's *point* size; **one global three-number slot shared by every window**, so the last call wins | yes on non-number args | `SDL_SetWindowHitTest`, `SDL_GetWindowSize` | [syswindow.rs:197](../../../src/syswindow.rs#L197), [hit.rs:18](../../../src/hit.rs#L18) | [syswindow.lua:99](../../../tests/contract/cases/syswindow.lua#L99) + [hit-test-check.sh](../../../scripts/rust-port/hit-test-check.sh) |
| `text_input(window, enabled)` | the one window function tolerant of a NULL `RenWindow` (returns quietly); truthiness starts/stops IME | yes on non-userdata | `SDL_StartTextInput`, `SDL_StopTextInput` | [syswindow.rs:213](../../../src/syswindow.rs#L213) | [syswindow.lua:110](../../../tests/contract/cases/syswindow.lua#L110) |
| `set_text_input_rect(window, x,y,w,h)` | integers; passed to `SDL_SetTextInputArea` with rect scale 0 | yes on non-number | `SDL_SetTextInputArea` | [syswindow.rs:228](../../../src/syswindow.rs#L228) | [syswindow.lua:110](../../../tests/contract/cases/syswindow.lua#L110) |
| `clear_ime(window)` | ends composition | — | `SDL_ClearComposition` | [syswindow.rs:240](../../../src/syswindow.rs#L240) | [syswindow.lua:110](../../../tests/contract/cases/syswindow.lua#L110) |
| `show_fatal_error(title, message)` | `SDL_ShowSimpleMessageBox` with no parent window, so on a headless run it can block on a real dialog | yes on non-string | `SDL_ShowSimpleMessageBox` | [syswindow.rs:247](../../../src/syswindow.rs#L247) | [syswindow.lua:135](../../../tests/contract/cases/syswindow.lua#L135) (argument validation only) |

Hit regions that Lua asks for are the borderless title bar's: `title_height`, `controls_width` and an icon-spacing resize border ([titleview.lua:45](../../../data/core/titleview.lua#L45)); `system.set_window_hit_test(core.window)` with no numbers clears the callback. `hit.rs` yields `SDL_HitTestResult` codes 0/1/2/4/6/7/8/9 — draggable only when `y < title_height && x > rb && x < w - controls_width`, the top and right edges are unreachable because the C's `RESIZE_FROM_TOP`/`RESIZE_FROM_RIGHT` are 0 ([hit.rs:14](../../../src/hit.rs#L14)); every other point is `NORMAL`, so the window stays resizable on Linux's SDL path.

## `system`: plugins and dialogs

| Lua-visible signature | Behaviour | Raises | SDL behind it | Source | Coverage |
|---|---|---|---|---|---|
| `load_native_plugin(name, path) -> integer` | `SDL_LoadObject`, entry point named after the part after the last `.` (`luaopen_lite_xl_<base>` with an `api_require` resolver, else `luaopen_<base>`); stores `{handle=…}` in `package.native_plugins[name]` with the `NativePlugin` metatable whose `__gc` calls `SDL_UnloadObject`; returns the entry point's return value | yes if load/entry missing, or entry returns 0 | `SDL_LoadObject`, `SDL_LoadFunction`, `SDL_UnloadObject` | [sysplugin.rs:57](../../../src/sysplugin.rs#L57) | [sysplugin.lua](../../../tests/contract/cases/sysplugin.lua#L1) |
| `open_file_dialog(window, id, options?)` | returns immediately (0 values); the result arrives as `dialogfinished`. Options: `default_location`, `title`, `accept_label`, `cancel_label`, `allow_many`, `filters = {{name=,pattern=}}` (patterns are SDL glob patterns, e.g. `*.lua`) | yes if `id` is not an integer, `options` is not a table, or a filter is missing `name`/`pattern` | `SDL_ShowFileDialogWithProperties` + properties | [sysplugin.rs:317](../../../src/sysplugin.rs#L317), [open_dialog:262](../../../src/sysplugin.rs#L262), [dialog_filters:240](../../../src/sysplugin.rs#L240) | [sysplugin.lua:168](../../../tests/contract/cases/sysplugin.lua#L168) |
| `save_file_dialog(window, id, options?)` | same, plus `filters` | as above | same with `SDL_FileDialogType::SAVEFILE` | [sysplugin.rs:320](../../../src/sysplugin.rs#L320) | [sysplugin.lua:170](../../../tests/contract/cases/sysplugin.lua#L170) |
| `open_directory_dialog(window, id, options?)` | same; `allow_many` honoured, `filters` ignored | as above | same with `OPENFOLDER` | [sysplugin.rs:323](../../../src/sysplugin.rs#L323) | [sysplugin.lua:172](../../../tests/contract/cases/sysplugin.lua#L172) |

Opening a dialog needs a session D-Bus (the desktop portal). The contract run deliberately points `DBUS_SESSION_BUS_ADDRESS` at a dead path and clears `SDL_FILE_DIALOG_DRIVER`, so SDL answers with an error event instead of a window ([verify.sh:50](../../../scripts/rust-port/verify.sh#L50)); the suite therefore covers the **error** payload, the options table, argument errors and "id is arbitrary", never a real pick.

## `renwindow`

| Lua-visible signature | Behaviour | Raises | SDL behind it | Source | Coverage |
|---|---|---|---|---|---|
| `renwindow.create(title, width?, height?) -> RenWindow` | initialises the video subsystem on first use, sizes from `SDL_GetCurrentDisplayMode` × 0.8 when width/height < 1, creates the window with `RESIZABLE \| HIGH_PIXEL_DENSITY \| HIDDEN`, sets the 64×64 RGBA icon, adds it to the window list; the first rendered frame calls `SDL_ShowWindow` | yes on video-init/window-create failure and on a missing display mode | `SDL_InitSubSystem(VIDEO)`, `SDL_CreateWindow`, `SDL_GetCurrentDisplayMode`, `SDL_CreateSurfaceFrom`/`SDL_SetWindowIcon`, `SDL_ShowWindow` | [renwindow_api.rs:46](../../../src/renwindow_api.rs#L46), [init_window_icon:28](../../../src/renwindow_api.rs#L28), [renwindow.rs:195](../../../src/renwindow.rs#L195) | every event suite, e.g. [sysevents.lua:44](../../../tests/contract/cases/sysevents.lua#L44) |
| `window:get_size() -> w, h` | the window **surface** dimensions, as Lua numbers. The surface is what `renderer.*` draws into and what the clip rects are expressed in; SDL sizes a window surface in *pixels*, and this path does **not** divide by the scale (only the `sdl_renderer` build (still present; the map plans to retire it) does) | — | `SDL_GetWindowSurface` | [renwindow_api.rs:103](../../../src/renwindow_api.rs#L103), [ren_get_size:169](../../../src/renwindow.rs#L169) | [sysevents.lua:46](../../../tests/contract/cases/sysevents.lua#L46) (scale 1 only) |
| `window:_persist()` | marks this window as surviving a Lua-state restart; `__gc` then skips `ren_destroy` | — | — | [renwindow_api.rs:112](../../../src/renwindow_api.rs#L112), [f_renwin_gc:95](../../../src/renwindow_api.rs#L95) | [startup-check.sh](../../../scripts/rust-port/startup-check.sh#L20) |
| `renwindow._restore() -> RenWindow?` | hands back the persistent window to a new Lua state, or `nil` | — | — | [renwindow_api.rs:117](../../../src/renwindow_api.rs#L117) | [startup-check.sh](../../../scripts/rust-port/startup-check.sh#L20) |

`data/core/init.lua` uses the pair for `core.restart`: `core.window = core.window or renwindow._restore() or renwindow.create("")` ([init.lua:374](../../../data/core/init.lua#L374)).

## `system.poll_event` events

`poll_event()` returns `event_name, args…` or nothing at all when the queue is empty; events that produce nothing loop to the next one inside the same call. Units marked *points* come from SDL's window coordinates; *pixels* means window coordinates multiplied by the renderer surface scale.

| Event | Arguments | SDL source | Rules | Source | Coverage |
|---|---|---|---|---|---|
| `quit` | — | `SDL_EVENT_QUIT` | — | [sysevents.rs:105](../../../src/sysevents.rs#L105) | [sysevents.lua:54](../../../tests/contract/cases/sysevents.lua#L54) |
| `resized` | `w, h` (*points*) | `WINDOW_RESIZED` | calls `ren_resize_window` first, so surface + scale refresh before Lua sees it | [sysevents.rs:107](../../../src/sysevents.rs#L107) | [sysevents.lua:69](../../../tests/contract/cases/sysevents.lua#L69) |
| `exposed` | — | `WINDOW_EXPOSED` | calls `rencache_invalidate()` | [sysevents.rs:121](../../../src/sysevents.rs#L121) | [sysevents.lua:60](../../../tests/contract/cases/sysevents.lua#L60) |
| `minimized` / `maximized` / `restored` | — | matching `WINDOW_*` | — | [sysevents.rs:125](../../../src/sysevents.rs#L125) | [sysevents.lua:55](../../../tests/contract/cases/sysevents.lua#L55) |
| `mouseleft` | — | `WINDOW_MOUSE_LEAVE` | — | [sysevents.rs:128](../../../src/sysevents.rs#L128) | [sysevents.lua:55](../../../tests/contract/cases/sysevents.lua#L55) |
| `focuslost` | — | `WINDOW_FOCUS_LOST` | — | [sysevents.rs:129](../../../src/sysevents.rs#L129) | [sysevents.lua:55](../../../tests/contract/cases/sysevents.lua#L55) |
| *(none)* | — | `WINDOW_FOCUS_GAINED` | **swallowed, and flushes queued `KEY_DOWN`** so alt-tab key repeats cannot reach Lua | [sysevents.rs:131](../../../src/sysevents.rs#L131) | [sysevents.lua:79](../../../tests/contract/cases/sysevents.lua#L79) |
| `filedropped` | `path, x, y` (*pixels*) | `DROP_FILE` | position is `SDL_GetMouseState()` (points) × scale; a drop with no matching window is reported at `0,0`; `SDL_EVENT_DROP_FILE` is explicitly enabled at startup | [sysevents.rs:137](../../../src/sysevents.rs#L137), [app.rs:103](../../../src/app.rs#L103) | [sysevents.lua:161](../../../tests/contract/cases/sysevents.lua#L161) |
| `keypressed` | `name` | `KEY_DOWN` | name from `key_name`, see below | [sysevents.rs:156](../../../src/sysevents.rs#L156) | [sysevents.lua:84](../../../tests/contract/cases/sysevents.lua#L84) |
| `keyreleased` | `name` | `KEY_UP` | same | [sysevents.rs:159](../../../src/sysevents.rs#L159) | [sysevents.lua:103](../../../tests/contract/cases/sysevents.lua#L103) |
| `textinput` | `text` | `TEXT_INPUT` | UTF-8 string, may be empty | [sysevents.rs:163](../../../src/sysevents.rs#L163) | [sysevents.lua:119](../../../tests/contract/cases/sysevents.lua#L119) |
| `textediting` | `text, start, length` | `TEXT_EDITING` | `start`/`length` are SDL's **UTF-8 codepoint offsets** into `text` (-1 means "no selection"); `data/core/ime.lua` converts to byte offsets and treats `start<0`/`length<0` as caret-only | [sysevents.rs:164](../../../src/sysevents.rs#L164), [ime.lua:13](../../../data/core/ime.lua#L13) | [sysevents.lua:122](../../../tests/contract/cases/sysevents.lua#L122) |
| `mousepressed` | `button, x, y, clicks` (*pixels*, 1-based `clicks`) | `MOUSE_BUTTON_DOWN` | button 1 captures the mouse at SDL level; button names below; an event for an unregistered window is dropped (the C dereferences NULL) | [sysevents.rs:172](../../../src/sysevents.rs#L172) | [sysevents.lua:128](../../../tests/contract/cases/sysevents.lua#L128) |
| `mousereleased` | `button, x, y` (*pixels*) | `MOUSE_BUTTON_UP` | releases capture on button 1 | [sysevents.rs:191](../../../src/sysevents.rs#L191) | [sysevents.lua:137](../../../tests/contract/cases/sysevents.lua#L137) |
| `mousemoved` | `x, y, dx, dy` (*pixels*) | `MOUSE_MOTION` | **coalesces**: pumps and drains every queued motion event, keeps the last position and sums the deltas | [sysevents.rs:209](../../../src/sysevents.rs#L209) | [sysevents.lua:141](../../../tests/contract/cases/sysevents.lua#L141) |
| `mousewheel` | `y, x` (Lua numbers) | `MOUSE_WHEEL` | **order is vertical first**, horizontal is negated (`-e.wheel.x`) so shift+scroll matches vertical; raw SDL floats (fractional/precise values pass through); `direction`/`integer_x`/`integer_y` are ignored, so `FLIPPED` is not corrected | [sysevents.rs:241](../../../src/sysevents.rs#L241) | [sysevents.lua:146](../../../tests/contract/cases/sysevents.lua#L146) |
| `touchpressed` | `x, y, finger_id` (*points*) | `FINGER_DOWN` | normalised finger coordinates × `SDL_GetWindowSize`, so **touch is in points while mouse is in pixels**; no coalescing | [sysevents.rs:249](../../../src/sysevents.rs#L249) | [sysevents.lua:149](../../../tests/contract/cases/sysevents.lua#L149) |
| `touchreleased` | `x, y, finger_id` (*points*) | `FINGER_UP` | same mapping | [sysevents.rs:249](../../../src/sysevents.rs#L249) | [sysevents.lua:151](../../../tests/contract/cases/sysevents.lua#L151) |
| `touchmoved` | `x, y, dx, dy, finger_id` (*points*) | `FINGER_MOTION` | coalesces like motion: last position, summed deltas | [sysevents.rs:268](../../../src/sysevents.rs#L268) | [sysevents.lua:153](../../../tests/contract/cases/sysevents.lua#L153) |
| `enteringforeground` | — | `WILL_ENTER_FOREGROUND` | calls `ren_refresh_after_foreground()`; `core.step` breaks its event loop on this event to split a frame | [sysevents.rs:301](../../../src/sysevents.rs#L301), [renwindow.rs:425](../../../src/renwindow.rs#L425) | [sysevents.lua:61](../../../tests/contract/cases/sysevents.lua#L61) |
| `enteredforeground` | — | `DID_ENTER_FOREGROUND` | same refresh, no break | [sysevents.rs:301](../../../src/sysevents.rs#L301) | [sysevents.lua:61](../../../tests/contract/cases/sysevents.lua#L61) |
| `enteringbackground` / `enteredbackground` | — | `WILL/DID_ENTER_BACKGROUND` | — | [sysevents.rs:311](../../../src/sysevents.rs#L311) | [sysevents.lua:61](../../../tests/contract/cases/sysevents.lua#L61) |
| `dialogfinished` | `id, status, result?` | custom event (kind `dialogfinished`, registered first, type `SDL_EVENT_USER`) | `status` is `"accept"` (then `result` is an array of absolute paths), `"cancel"` (3 values), `"error"` (then SDL's message) or `"unknown"`; pushed by the dialog thread, converted on the main thread by `dialogfinished_callback` | [sysplugin.rs:190](../../../src/sysplugin.rs#L190), [registration:584](../../../src/system.rs#L584) | [sysplugin.lua:168](../../../tests/contract/cases/sysplugin.lua#L168), forged in [test_hooks.rs:322](../../../src/test_hooks.rs#L322) |
| *(swallowed)* | — | custom event `dirmonitor` | registered with a **no-op callback**, so the event is consumed and polling continues to the next event; it exists only to wake the event loop | [dirmonitor.rs:331](../../../src/dirmonitor.rs#L331), [custom_events.rs:56](../../../src/custom_events.rs#L56) | [customevents.lua](../../../tests/contract/cases/customevents.lua#L1) (callback presence), [dirmonitor.lua](../../../tests/contract/cases/dirmonitor.lua#L1) (module only) |
| *(none)* | — | `WINDOW_PIXEL_SIZE_CHANGED`, `WINDOW_DISPLAY_SCALE_CHANGED` | the default arm resizes the window (so the surface/scale update) and then yields **nothing** to Lua | [sysevents.rs:315](../../../src/sysevents.rs#L315) | [sysevents.lua:72](../../../tests/contract/cases/sysevents.lua#L72) |
| *(none)* | — | any unhandled type, any unknown user event | falls through and loops to the next event | [sysevents.rs:308](../../../src/sysevents.rs#L308) | [sysevents.lua:170](../../../tests/contract/cases/sysevents.lua#L170) |

Critically, `poll_event` **never calls `SDL_PumpEvents` except while coalescing motion**; the pump itself happens inside `SDL_PollEvent`.

### Which threads push events

| Producer | Thread | Event | Note |
|---|---|---|---|
| SDL video driver | SDL internal | everything above | — |
| `dirmonitor` worker (`dirmonitor_check_thread`) | own named thread, 1 ms sleep loop | `dirmonitor` | `push_custom_event` is documented thread-safe; the thread never touches Lua ([dirmonitor.rs:175](../../../src/dirmonitor.rs#L175)) |
| SDL file dialog | SDL's own thread | `dialogfinished` | `dialog_callback` only builds a byte block; Lua runs later on the main thread ([sysplugin.rs:143](../../../src/sysplugin.rs#L143)) |
| test hooks | main thread | any | `lxl_test.push_event` uses `SDL_PushEvent` directly ([test_hooks.rs:227](../../../src/test_hooks.rs#L227)) |

### `system.wait_event(timeout?) -> bool`

Seconds with fractions, negatives clamp to 0; with no argument it waits forever. `true` means an event arrived, `false` means timeout. It does **not** consume the event: a subsequent `poll_event` still sees it ([sysevents.rs:340](../../../src/sysevents.rs#L340), covered in [sysevents.lua:207](../../../tests/contract/cases/sysevents.lua#L207)). `data/core/init.lua` drives the whole frame loop through it ([init.lua:1057](../../../data/core/init.lua#L1057)).

## Key naming, modifiers and text input

`key_name` ([sysevents.rs:57](../../../src/sysevents.rs#L57)) builds the Lua string:

1. **Keypad first**: for scancodes `KP_1..KP_PERIOD` **while the `NUM` modifier bit is clear**, the name becomes the navigation name (`end`, `down`, `pagedown`, `left`, `""`, `right`, `home`, `up`, `pageup`, `ins`, `delete`) — including the empty string for `KP_5`. With NumLock set, the ordinary path runs instead.
2. Otherwise: `SDL_GetKeyName(keycode)` when the keycode is ASCII (`< 128`) **or** has the scancode bit set; `SDL_GetScancodeName(scancode)` otherwise. So Latin letters and named keys are layout-aware, while non-Latin layouts (Cyrillic, Greek, CJK keycodes) fall back to the QWERTY physical name.
3. The result is lower-cased with ASCII `to_ascii_lowercase`, so `A`/`a` collapse and SDL's `"Left"` becomes `"left"`. The C's `char buf[16]` overflow is gone, so long scancode names (`Keypad Memory Store`) now survive intact — a **deliberate deviation** recorded in the module header ([sysevents.rs:1](../../../src/sysevents.rs#L1)) and pinned by the "kp memstore" case.

Modifier state is **not** passed to Lua: modifiers only influence the keypad rule. Keysyms are what Lua sees, never raw scancodes, and there is no way for Lua to ask for the physical key.

Text input: `system.text_input(window, bool)` starts/stops SDL text input ([init.lua:687](../../../data/core/init.lua#L687) calls it whenever the active view's `supports_text_input()` changes), `set_text_input_rect` tells SDL where the IME panel should point, `clear_ime` cancels composition. `ime.set_location` only forwards changed rectangles and passes **surface coordinates** ([ime.lua:79](../../../data/core/ime.lua#L79), called from [docview.lua:408](../../../data/core/docview.lua#L408)); `SDL_SetTextInputArea` expects window points, so at scale ≠ 1 the IME panel is misplaced unless the coordinates happen to coincide. `SDL_HINT_IME_IMPLEMENTED_UI=1` asks SDL for its own candidate UI. `data/core/ime.lua` keeps its own `editing` flag because `clear_ime` "doesn't work everywhere".

Mouse buttons are named by `button_name` ([sysevents.rs:43](../../../src/sysevents.rs#L43)): `1=left, 2=middle, 3=right, 4=x, 5=y`, anything else `"?"`. Click counts come straight from SDL's `clicks` field (1-based, SDL's own double-click logic, radius hint 4 px).

## Window behaviour, scale and units

- **Modes** are the four option strings above; `data/core` round-trips through `system.get_window_mode` for the title bar and saves `window_mode` in `session.lua` with `window` = `get_window_size` ([init.lua:27](../../../data/core/init.lua#L27), [init.lua:375](../../../data/core/init.lua#L375)). On restart, `"normal"` restores the saved rectangle and `"maximized"` re-maximizes; `"fullscreen"` is not restored.
- **Borderless** is `config.borderless` → `set_window_bordered(window, false)` plus the hit-test region ([init.lua:255](../../../data/core/init.lua#L255)).
- **Icon**: 64×64 RGBA from `resources/icons/icon.inl`, extracted at build time ([build.rs:39](../../../build.rs#L39)) and uploaded once per window with `SDL_CreateSurfaceFrom`/`SDL_SetWindowIcon`.
- **Focus** is queried only through `window_has_focus`; the event side is `focuslost` (there is no `focusgained` name — that SDL event is swallowed).
- **Raise** and **opacity** are straightforward SDL calls, except that opacity's boolean is hard-coded true.
- **Scale**: on the default (non-`sdl_renderer`) build `update_scale` sets `scale_x = surface.w / window.w` and `scale_y = surface.h / window.h` in floats ([renwindow.rs:377](../../../src/renwindow.rs#L377)); these are the factors `poll_event` multiplies mouse, motion and drop coordinates by, converting SDL's point coordinates into the pixel space `renderer.*` draws in. There is no exposed "scale" getter in Lua; the only way Lua observes scaling is the difference between `system.get_window_size` (points) and `window:get_size()` (surface). `ren_resize_window` is called on `resized`, `set_window_size`, and the pixel-size/display-scale-changed default arm, and that is what refreshes the factors.
- **`renwindow.create` always requests `SDL_WINDOW_HIGH_PIXEL_DENSITY`** ([renwindow_api.rs:69](../../../src/renwindow_api.rs#L69)), so on a scaled display scale_x/scale_y are expected to differ from 1.
- **Integer-ratio assertion**: the `sdl_renderer` build (still present; the map plans to retire it) asserts that pixels/points is integral, non-zero and equal on both axes ([renwindow.rs:251](../../../src/renwindow.rs#L251)); it also rejects fractional scale outright. The active build has no such assertion and simply keeps the float ratios, which is what makes fractional scaling *potentially* work but untested (see gaps).
- `SDL_SetWindowOpacity`, `SDL_RaiseWindow`, hit-testing and `SDL_WINDOW_HIGH_PIXEL_DENSITY` are the window features the map already flags as gaps for candidate crates.

## Environment shaping

`SDL_SetHint` calls, all in `video_init` on first window creation ([renderer.rs:985](../../../src/renderer.rs#L985)):

| Hint | Value | Why it is there |
|---|---|---|
| `SDL_HINT_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR` | `0` | makes X11 compositors keep compositing the window (needed for opacity/transparency) |
| `SDL_HINT_MOUSE_FOCUS_CLICKTHROUGH` | `1` | click-through on a focused but not hovered window |
| `SDL_HINT_IME_IMPLEMENTED_UI` | `1` | SDL draws its own IME candidate UI |
| `SDL_BORDERLESS_WINDOWED_STYLE` | `1` | **passed as a string literal**: the 0.7.2 bindings export no constant for it; treat the window as normal when borderless |
| `SDL_BORDERLESS_RESIZABLE_STYLE` | `1` | same, allow resizing/aero-snap on a borderless window |
| `SDL_HINT_MOUSE_DOUBLE_CLICK_RADIUS` | `4` | SDL's click-count radius |
| `SDL_HINT_RENDER_DRIVER` | `software` | only affects the `sdl_renderer` build (it creates an SDL renderer) |

The X11 hint and the X11-specific borderless hints are the **only X11-vs-Wayland code paths in the tree**: there is no Wayland/X11 branching, no `xkbcommon` use, and no compositor detection in Lua. Everything else delegates the choice to SDL, which means Wayland specifics (primary selection availability, `wl_surface` scale events, IME) are SDL behaviour this inventory cannot see from source.

Startup enables three SDL event types explicitly: `SDL_EVENT_DROP_FILE` ([app.rs:103](../../../src/app.rs#L103)) and `SDL_EVENT_TEXT_INPUT`/`TEXT_EDITING` per Lua state ([app.rs:151](../../../src/app.rs#L151)).

Test-relevant environment variables:

| Variable | Effect | Where |
|---|---|---|
| `SDL_VIDEODRIVER=offscreen` | default driver for every contract suite; gives a real window with no compositor, so `renwindow.create`, resizes and surface reads work | [verify.sh:52](../../../scripts/rust-port/verify.sh#L52) |
| `SDL_VIDEODRIVER=dummy` | the restart and quit startup checks, because a persistent window must not be destroyed and exiting with a live window under `offscreen` segfaults in `SDL_Quit` | [startup-check.sh:47](../../../scripts/rust-port/startup-check.sh#L47) |
| `DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/lxl-contract-bus` | degrades file dialogs to an `"error"` event instead of opening a real picker | [verify.sh:50](../../../scripts/rust-port/verify.sh#L50) |
| `SDL_FILE_DIALOG_DRIVER=` (empty) | no forced dialog backend, so SDL uses the portal/GTK path and fails cleanly | [verify.sh:50](../../../scripts/rust-port/verify.sh#L50) |
| `LXL_TEST_C_EVENTS=1` | keeps the C `poll_event`/`wait_event`/window functions in the same binary so the baseline can be recorded; only compiled under `test_hooks` | [system.rs:544](../../../src/system.rs#L544) |
| `TARGET_DIR`, `STAGE_DIR`, `CARGO_ARGS`, `RECORD`, `CONTRACT_SUITES` | harness plumbing for verify.sh | [verify.sh:9](../../../scripts/rust-port/verify.sh#L9) |

Since this inventory, [Step 1 of the handoff](sdl3-step1-evidence.md) retired `LXL_TEST_C_EVENTS` and the C recording path. The driver, bus and dialog settings above now come from the runner-owned profiles in [private-display.sh](../../../scripts/rust-port/private-display.sh) (`offscreen`, `dummy`, private `xvfb`), and `RECORD=1` only writes a review copy. The table records the harness as it was inventoried.

**Test hooks** (`--features test_hooks`, never shipped) install the global `lxl_test` table ([test_hooks.rs:356](../../../src/test_hooks.rs#L356)): `push_event` injecting `quit`, `key_down/up`, `text_input`, `text_editing`, `mouse_down/up`, `mouse_motion`, `mouse_wheel`, `finger_*`, `drop_file`, `window`, `app`, `other` and a forged `dialog_finished`; `dirty_rects`; `pixel(window,x,y)` and `surface_hash(window,rect)` reading the window surface through `SDL_GetPixelFormatDetails`/`SDL_GetRGBA` and returning colour only, alpha excluded; and five `custom_*` helpers over the custom-event registry. Event constants are re-exported as `lxl_test.EVENT_*`. Both pixel readers go through the window surface, so they only work because the default build draws straight into it.

## Coverage gaps

Behaviours with **no test at all**:

1. `window:get_size()` and every unit-relative question at scale ≠ 1: no run has ever had `scale_x/scale_y != 1`. The pixel-vs-point question for `get_size` (surface pixels vs the code comment's "points") is therefore unverified — a real HiDPI window is needed.
2. Fractional scaling: the active build has no integer-ratio check, so behaviour between 1.0 and 2.0 is entirely unobserved.
3. IME end-to-end: `textediting` is tested with synthetic values, but no real preedit, no candidate UI placement, no `set_text_input_rect` correction at scale ≠ 1, and `clear_ime`'s platform coverage claim is untested.
4. Real dialogs: only the "no portal available" error path runs; `accept`/`cancel` payloads, `allow_many`, filter effects and the `id` round trip with a live portal are unobserved. `dialogfinished`'s forged event covers the Lua-side fan-out only.
5. Drag-and-drop from a real file manager (path encoding, no-window drops at a real coord) and touch/gesture from a real touch device.
6. Clipboard/primary-selection interoperation with a real owner: set → get round trips in-process, but keep-alive behaviour after the editor exits, and the Wayland primary-selection availability question, are untouched.
7. `windows_has_focus`/`raise_window`/`set_window_bordered`/`set_window_mode` under a real window manager: the offscreen driver accepts calls and reports flags, but title-bar, aero-snap and hit-test regions are never exercised (the hit grid is compared against the C fixture, not against a compositor).
8. Opacity: only the (always-true) return value is observed; nothing checks whether the window actually becomes translucent.
9. `show_fatal_error`: only argument validation runs; the dialog itself is never shown (it would block a headless run).
10. The `dirmonitor` event's wake-up role: the thread and the module are tested, but that `poll_event` consumes the custom event and returns "nothing" is covered only through the generic "registered, no callback" case.

Behaviours that are **only testable with a real window** (not merely untested): fullscreen transitions and their `resized`/`restored` echo, maximize/minimize round trips, window position (`set_window_size` x/y), mouse capture, double-click counting, wheel `FLIPPED`/precise handling, `SDL_GetMouseState` after a drag, and anything X11-vs-Wayland divergent.

Surprising things worth carrying into the decision:

- `system.rmdir` deletes **files**, not just empty directories (`SDL_RemovePath`); the map already decided to preserve that.
- `set_window_opacity` reports success unconditionally, and `set_clipboard`/`set_primary_selection` swallow SDL failures entirely.
- `absolute_path` returns zero values, not `nil`, on failure.
- Touch coordinates are points while mouse coordinates are pixels; wheel is unrelated to either; `filedropped` is pixels.
- `SDL_WINDOW_FOCUS_GAINED` is invisible to Lua, and it silently discards queued key presses.
- Hit-test state is a single global, so only one borderless window can have regions.
- `get_time` is a performance counter, so it must not be persisted or compared across processes; `data/core` uses it for blink and thread scheduling.
- SDL is used for far more than windowing here: `SDL_RemovePath`, `SDL_EnumerateDirectory`, `SDL_LoadObject`/`SDL_LoadFunction` (plugin loading), `SDL_setenv_unsafe`, `SDL_GetSandbox`, `SDL_malloc`/`SDL_strdup`/`SDL_free` for dialog payloads, the command-buffer allocator (`SDL_realloc`, [renwindow.rs:457](../../../src/renwindow.rs#L457)), and the window surface as the pixel store the test hooks read.
- The font loader still opens files through SDL only to reuse SDL's error wording — unchanged by this inventory.
- `hints`: `SDL_BORDERLESS_WINDOWED_STYLE`/`SDL_BORDERLESS_RESIZABLE_STYLE` are set by string, so a typo would fail silently; nothing verifies they took effect.
