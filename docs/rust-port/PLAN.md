# Lite XL: porting the C core to Rust

**Status:** planning complete, no implementation started.
**Repo:** `/home/juniorsundar/Documents/casual/lite-xl` @ `234fa099` (v2.1.7, Lua `MOD_VERSION` 4.0.0).
**How to use this document:** read §0–§7 once. Then, in each session, read §12 (Progress log) and the current wave in §8, run `scripts/rust-port/verify.sh`, and continue. Append to the ledger and the progress log when you finish or learn something. Do not re-derive the contract — it is recorded in §11 with `file:line` citations.

---

## 0. Decisions already made

| # | Decision | Implication |
|---|---|---|
| D1 | **Incremental staticlib.** A Rust `staticlib` crate is linked into the existing Meson/CMake-free C build; `api/*.c` modules are replaced one at a time. | The editor runs after every wave. `main.c` is the last thing to move. |
| D2 | **Keep the native C plugin ABI** (`lite_xl_plugin_api.h`, `system.load_native_plugin`). | The Rust side must supply a `void *(*)(const char *)` symbol resolver answering the exact `lua_*`/`luaL_*`/`luaopen_*` names. This is the riskiest FFI item (wave W9). |
| D3 | **Crates where equivalent**, not a 1:1 C-lib port. | `pcre2`, `freetype-rs`, `sdl3` accepted. Two exceptions where the crate is *not* equivalent (see §9 D-1, D-2): regex objects must stay Lua *tables*; `dirmonitor` must keep the `mode()`/watch-id contract that `dirwatch.lua` branches on. |
| D4 | **Linux first.** | `cfg` stubs + `TODO(W-win)`/`TODO(W-mac)` for Windows/macOS. Windows `utfconv`/overlapped pipes and macOS FSEvents/bundle are out of scope for v1. Emscripten is out of scope. |

Non-negotiable requirements carried from the user: **`data/` (all 19k lines of Lua) must not change**, and third-party Lua plugins must keep working.

---

## 1. Goal, non-goals, definition of done

**Goal.** Replace every `.c` file in `src/` with Rust, preserving the Lua-facing contract byte-for-byte, one module at a time, so that at every commit the editor builds and runs.

**Definition of done (v1, Linux).**

1. `src/` contains no `.c`/`.h` (except what is justified in §9).
2. `cargo build` produces the `lite-xl` binary; Meson is gone for the Linux target.
3. `scripts/rust-port/verify.sh` passes: the contract harness output for the Rust build is identical to the recorded C-build baseline.
4. A manual pass of the app: typing, selection, IME-less text input, find/replace with regex, a `process`-backed command, treeview file watching, theme + font scaling, command palette, restart.
5. At least one third-party *native* plugin loads and runs (proves D2).

**Non-goals for v1.** Windows, macOS, Emscripten; changing `data/`; changing the rendering API shape; perf work beyond not regressing; new features.

---

## 2. Architecture of the hybrid build

```
                       lite-xl (C main.c, then Rust main.rs)
                                 |
        +------------------------+-------------------------+
        |                                                  |
   C modules (shrinking)                          Rust staticlib (growing)
   src/api/*.c                                    rust/  (crate lite_xl_core)
   src/renderer.c  rencache.c  renwindow.c                |
   src/custom_events.c  arena_allocator.c                 |
        |                                                  |
        +-------------> one lua_State <--------------------+
                        (created by C in main.c)
```

Key property: **both sides speak the same raw `lua_State` ABI.** C calls `api_load_libs(L)` which does `luaL_requiref(L, "regex", luaopen_regex, 1)`. A Rust module compiled with mlua's `module` feature defines exactly `luaopen_regex(state: *mut lua_State) -> c_int` (`#[mlua::lua_module(name = "regex")]` → `luaopen_regex`, see `mlua_derive` `lib.rs:52-53`). **Consequence: `src/api/api.c` does not change at all when a module moves to Rust.** The only C-side edit per wave is deleting one line from `src/meson.build`.

mlua module mode does **not** own or close the state (`Lua::entrypoint1` → `Lua::entrypoint` wraps a foreign pointer); the state stays owned by `main.c` until W8.

---

## 3. Repo layout and build wiring

### 3.1 Layout

```
rust/
  Cargo.toml            crate-type = ["staticlib"]
  Cargo.lock            committed
  src/
    lib.rs              module list, shared prelude
    prelude.rs          LuaError helpers, table/userdata helpers, "0 values" utilities
    regex.rs            W1
    dirmonitor.rs       W2   (+ src/dirmonitor/ backends as submodules)
    custom_events.rs    W2   (not Lua-visible; shared with W5 dialogs)
    process.rs          W3
    utf8.rs             W4   (+ unidata.rs generated from src/unidata.h)
    system.rs           W5
    rencache.rs         W6
    renwindow.rs        W6
    renderer.rs         W7   (+ api-renderer font/userdata layer)
    plugin_api.rs       W9
    main.rs             W8 (bin target, added later)
scripts/
  rust-port/
    verify.sh           build + run contract harness on both builds + diff
    gen-unidata.lua     regenerate rust/src/unidata.rs from src/unidata.h
    baseline/           recorded C-build contract output (checked in)
tests/
  contract/
    contract.lua        the test runtime module (see §7)
    cases/*.lua
docs/rust-port/
    PLAN.md             this file
    LEDGER.md           optional: open decisions + findings that must not be lost (§9)
```

### 3.2 `rust/Cargo.toml`

```toml
[package]
name = "lite_xl_core"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "lite_xl_core"
crate-type = ["staticlib"]

[dependencies]
mlua = { version = "0.12", features = ["lua54", "module"] }
libc = "0.2"
# added per wave:
# pcre2 = "0.2"        (W1)
# notify / inotify     (W2, see §9 D-2)
# freetype-rs = "0.38" (W7)

[profile.release]
panic = "abort"          # see §4.7
lto = "thin"
```

Notes.
- `mlua 0.12.2` and `mlua_derive 0.11.0` are already in the local cargo registry cache; `sdl3` is **not** (it will be downloaded). If the build must work offline, vendor first.
- Do **not** enable `vendored`: Lua is provided by the C build (`subprojects/lua-5.4.6` or system Lua). `module` + `vendored` is contradictory.
- `panic = "abort"` is a deliberate choice: a Rust panic unwinding through the C `lua_pcall` (which `longjmp`s) is UB. Aborting is the safe failure. mlua also catches panics inside callbacks it invokes; the two mechanisms are complementary. Revisit in W8 if abort proves too blunt.
- `#[mlua::lua_module]` for a state created by `luaL_newstate` may want `skip_memory_check`; see `mlua-0.12.2/src/state.rs:626-645`. Try without first, add if mlua reports memory errors on a foreign state.

### 3.3 Meson wiring (the one thing to get right on day 0)

Add to `src/meson.build`:

```meson
# ---- Rust core ----
cargo = find_program('cargo')
rust_profile = get_option('buildtype') == 'release' ? 'release' : 'debug'
rust_root = meson.project_source_root() / 'rust'
rust_target_dir = meson.project_build_root() / 'rust-target'

rust_lib = custom_target('lite_xl_core',
    output: 'liblite_xl_core.a',
    # copy it to a known path so meson does not have to guess cargo's layout
    command: [
        find_program('sh'), '-c',
        '( cd ' + rust_root + ' && cargo build --target-dir ' + rust_target_dir +
        ' --profile ' + rust_profile + ' ) && cp ' + rust_target_dir + '/' + rust_profile +
        '/liblite_xl_core.a @OUTPUT@',
    ],
    console: true,
    build_by_default: true,
    # cargo decides what to rebuild; do not let meson skip it by mtime
    build_always_stale: true,
)
lite_deps += declare_dependency(sources: rust_lib)
lite_link_args += [rust_lib.full_path()]
```

- `declare_dependency(sources: ...)` is what enforces build order before `executable()` links; passing the absolute `.a` path as a link arg is what actually links it.
- `cargo` is available in the dev shell via `nix develop` / the system toolchain. `flake.nix`'s `devShells` and `packages.lite-xl` **must** gain `cargo`/`rustc` (see §9 D-5) — the `packages` derivation builds offline in the Nix sandbox, so it needs a vendored cargo registry. Handle this in W8; until then, keep using `nix develop`.
- The existing build tree (`build/`) already has `wrap_mode=forcefallback` and falls back to `subprojects/`. If the linker complains about duplicate Lua symbols, that means mlua-sys linked its own Lua: that only happens if `vendored` is enabled, so don't enable it.

**Fallback if Meson+cargo proves awkward** (bounded to 1 hour of effort): a `scripts/rust-port/build.sh` wrapper that runs cargo then ninja, and drop the Meson integration entirely. Do not spend more than that; the modules themselves are the work.

### 3.4 Build / run commands (current, must keep working)

```sh
nix develop
meson setup build --wrap-mode=forcefallback   # already done in this checkout
ninja -C build
meson install -C build --destdir "$PWD/build/stage"
LITE_PREFIX="$PWD/build/stage/usr/local" ./build/src/lite-xl
```

`build-dev/` is a second, partial tree; ignore it (it has no `stage/`).

---

## 4. Invariants you must not break

These are the traps that will silently break Lua or plugins. Each has a citation.

1. **Return shapes: `nil` vs zero values.** mlua's `()` (`nil`) is *not* the same as "push nothing".
   - `system.absolute_path` returns **0 values** on failure, not `nil` (`system.c:654-672`).
   - `system.get_clipboard`/`get_primary_selection` return **0 values** when empty.
   - `regex.cmatch` returns **0 values** on no match (`regex.c:182-223`).
   - `process:returncode()` returns **0 values** while running (`process.c:703-709`).
   - `process:read` returns `""` at EOF/`EAGAIN`, and **0 values** only on a real read error (`process.c:602-650`). `data/core/process.lua` converts `""`→`nil`; that Lua layer stays.
   - In mlua, a `MultiValue` built empty reproduces "0 values"; `nil` does not.
2. **Userdata identity vs tables.** The Lua side checks `type()`.
   - `Font` is userdata; **a font *group* is a plain Lua table with the `Font` metatable** and a no-op `__gc` (`api/renderer.c:154`, `:198`). A Rust newtype for groups changes `type(font)` to `"userdata"` and breaks `data/core/statusview.lua:308,1243`.
   - `RenWindow` userdata holds a `RenWindow **` (pointer *to* the pointer, `api/renwindow.c:34`); `_restore` hands out a **second** userdata wrapping the same pointer (`:80`). `__gc` must not free a window that is `== persistant_window` (`:56`).
   - Compiled regexes are **plain tables** `{[1]=lightuserdata}` whose metatable is the **module table itself** (`regex.c:173`, `:372`), with `regex.__index` installed from Lua (`data/core/regex.lua:2`) so `pattern:gsub(...)` works. Observable via `getmetatable(re)`.
3. **Metatable names.** `"Font"`, `"Process"`, `"Dirmonitor"`, `"RenWindow"`, `"NativePlugin"` (`api/api.h:8-12`). Anything that does `luaL_checkudata(name)` must keep matching.
4. **Coordinate spaces.** `system.poll_event` mouse/key coordinates are multiplied by `window->scale_x/scale_y` → **pixels**; `renderer.get_size` and `renwindow:get_size` report **points** (`renderer.c:841`, `system.c:192-247`). Do not "unify" these.
5. **Time units.** `system.get_time` is **monotonic** seconds (`SDL_GetPerformanceCounter/Frequency`). `process:wait`/`start` `timeout` is **milliseconds** in C but `data/core/process.lua` passes seconds in the coroutine path — preserve the raw C unit.
6. **Paths and events.** `system.exec` background semantics (POSIX `"<cmd> &"`), `PATHSEP`, and `SDL_EVENT_USER`-relative custom event dispatch order.
7. **Panics must not unwind into Lua.** See §3.2.
8. **`get_time` is called 43×, `get_file_info` 31×, `absolute_path` 14× in `data/core`** — these are hot; keep them allocation-light.

---

## 5. Seams: C helpers the Rust side needs

`src/api/system.c` reaches into the `RenWindow` *struct* (`window_renderer->scale_x`, `->scale_y`, `->window`) and calls ren-layer functions. Rather than guess a `#[repr(C)]` layout (which changes with `LITE_USE_SDL_RENDERER`), add three tiny accessors to `src/renwindow.c` + declarations in `src/renderer.h` during **W0** or early **W5**:

```c
/* renwindow.c */
float ren_get_scale_x(RenWindow *ren) { return ren->scale_x; }
float ren_get_scale_y(RenWindow *ren) { return ren->scale_y; }
SDL_Window *ren_get_sdl_window(RenWindow *ren) { return ren->window; }
```

Already exported and safe to `extern "C"` from Rust (non-static C functions with declarations in `renderer.h`):

- `ren_init`, `ren_free`, `ren_create`, `ren_destroy`, `ren_resize_window`, `ren_update_rects`,
- `ren_get_window_list`, `ren_find_window`, `ren_find_window_from_id`, `ren_get_target_window`, `ren_set_target_window`, `ren_get_size`,
- `rencache_invalidate`, `rencache_begin_frame`, `rencache_end_frame`, `rencache_set_clip_rect`, `rencache_draw_rect`, `rencache_draw_text`, `rencache_show_debug`,
- `register_custom_event`, `push_custom_event`, `get_custom_event_callback_by_type`, `get_custom_event_name`, `init_custom_events`, `free_custom_events` (`custom_events.h`),
- `lxl_arena_*` (`arena_allocator.h`), used by `system.c` dialogs and `process.c` env serialisation.

Rule: **when Rust needs a C symbol, declare it in `rust/src/cffi.rs` with the exact C signature. Never re-declare a C struct.** Add an accessor instead.

`rust/src/cffi.rs` will accumulate these; keep it the single place with `extern "C"` declarations.

---

## 6. Day 0 bootstrap checklist (one session, ~2–4h)

- [ ] `cargo init --lib rust` and set `crate-type = ["staticlib"]`; commit `Cargo.lock`.
- [ ] Add the `#[mlua::lua_module(name = "rust_probe")]` probe module returning `{ ping = function() return "pong" end }`.
- [ ] Add `rust_probe` to `src/api/api.c`'s `libs[]` table **temporarily**; add the Meson wiring from §3.3.
- [ ] `ninja -C build` → verify `nm -g build/src/liblite_xl_core.a | grep luaopen_rust_probe` shows a global symbol with `T`.
- [ ] Run the editor, `:lua print(rust_probe.ping())` → `pong`.
- [ ] **Remove the probe** from `api.c` (keep the module behind `#[cfg(feature = "probe")]` if useful).
- [ ] Write `scripts/rust-port/verify.sh` and record the C baseline (§7).
- [ ] Add `cargo`/`rustc` to `flake.nix` devShell (keep `packages` for later, §9 D-5).

Stop conditions: if the `.a` links but `luaopen_*` is missing → check `#[no_mangle]` and that `module` (not `vendored`) is on. If Lua symbols duplicate → `vendored` got enabled. If Meson refuses to order the build → use the §3.3 fallback wrapper.

---

## 7. Verification harness (build once, use in every wave)

There is **no test suite in this repo** and CI only builds/packages. Verification must be built, not found.

### 7.1 The no-C-changes trick

`main.c:152-190` runs this embedded chunk:

```lua
dofile(DATADIR/core/start.lua)
core = require(os.getenv('LITE_XL_RUNTIME') or 'core')
core.init()
core.run()
```

So `LITE_XL_RUNTIME` selects an alternative runtime module, and `start.lua` sets `package.path` from `LITE_PREFIX`/`DATADIR`. Therefore:

```
tests/contract/contract.lua      # copied into <stage>/share/lite-xl/contract.lua
LITE_PREFIX=<stage> LITE_XL_RUNTIME=contract ./lite-xl
```

runs our test module with **zero changes to C or to `data/`**. `contract.lua` must expose `init()`, `run()`, `on_error(err)`.

### 7.2 `tests/contract/contract.lua` shape

```lua
local M = {}
function M.init()
  local out = {}
  local function rec(k, v) out[#out+1] = ("%-40s %s"):format(k, tostring(v)) end
  -- deterministic, normalised observations only
  for _, case in ipairs(require("cases")) do case(rec) end
  io.stdout:write(table.concat(out, "\n") .. "\n")
  os.exit(0)
end
function M.run() end
function M.on_error(err) io.stdout:write("HARNESS ERROR: "..tostring(err).."\n"); os.exit(1) end
return M
```

### 7.3 Rules for cases

- Only deterministic observations: shapes, types, `getmetatable(x)`, `type(x)`, `select('#', f())`, constant strings, integer offsets.
- Never print timestamps, pids, absolute temp paths, or font metrics that depend on the system. Normalise with `sed` in `verify.sh` where unavoidable.
- One case file per wave: `tests/contract/cases/{regex,dirmonitor,process,utf8,system,render}.lua`.
- Include *negative* cases: `select('#', system.absolute_path("/nonexistent/\0"))`, `regex.cmatch("a","b")`, `process.strerror(2)`, `type(regex.compile("("))`.
- `verify.sh` must run the C baseline **before** a wave deletes the C file and store it in `scripts/rust-port/baseline/<module>.txt` (checked in). After the wave, the Rust output must match.

### 7.4 `scripts/rust-port/verify.sh` (behaviour)

1. `ninja -C build` (current hybrid state).
2. `meson install -C build --destdir build/stage`; copy `tests/contract/*` into the stage share dir.
3. Run with `SDL_VIDEODRIVER=offscreen` (fall back to `xvfb-run -a` if offscreen is flaky — SDL3's offscreen driver is enough for W1–W5, not for W6/W7 rendering).
4. `diff -u scripts/rust-port/baseline/<wave>.txt <(normalised output)`.
5. Exit non-zero on any diff or on `HARNESS ERROR`.

Rendering waves (W6/W7) additionally want a screenshot comparison: `system.get_time`-free. Take `SDL_SaveBMP`-style output via `SDL_GetWindowSurface` only in `-Drenderer=false` builds, or simply assert the command buffer is deterministic by exposing a debug hook. Pragmatic approach for v1: render a fixed scene in the contract runtime and hash the resulting dirty-rect list, not pixels.

---

## 8. The waves

Each wave: **goal → Lua-visible contract to preserve → pitfalls → tests → done-when.** Order is chosen so the editor is runnable after each wave and the hardest module (FreeType) comes last.

### W0 — Bootstrap and harness
Covered in §6/§7. **Deliverable:** Rust `.a` linked into the C build, probe module works, `verify.sh` + C baseline recorded.

---

### W1 — `regex` (386 C lines) — *recommended first real module*

**Goal.** `rust/src/regex.rs` defines `luaopen_regex`; delete `api/regex.c` from `src/meson.build`.

**Contract** (`src/api/regex.c`; wrapper `data/core/regex.lua`):
- `regex.compile(pattern, options?)` → compiled object, or `nil, errmsg`. `options` is a string containing any of `i` (CASELESS), `m` (MULTILINE), `s` (DOTALL). Always `PCRE2_UTF`; JIT enabled.
- The compiled object must be a **table** `{ [1] = <pattern handle> }` whose metatable is the `regex` module table; the module table is also stored at `LUA_REGISTRYINDEX["regex"]` (`regex.c:372`) and carries `__gc` (`:363`) which frees via `rawgeti(1)`.
- `regex.cmatch(pattern|string, str, offset?, options?)` → `2*rc` integers, 1-based byte offsets, whole match first then each group, end = one past last. **0 values** if no match. Negative `offset` → relative to end, clipped (`regex_offset_relative`, `:120-130`). Raises `error()` on PCRE error and on `\K`-in-assertion (`ovector[0] > ovector[1]`).
- `regex.gmatch(pattern, subject, offset?)` → iterator. **No options arg.** If the pattern has captures, iteration starts at ovector index **2** (skips the whole match); empty captures push their 1-based index instead of a string. Holds the subject alive (`lua_settop(L,2)`).
- `regex.gsub(pattern, subject, replacement, limit?)` → `result, count`. `limit <= 0` means global via `PCRE2_SUBSTITUTE_GLOBAL`; positive `limit` loops non-global. Uses `PCRE2_SUBSTITUTE_EXTENDED | OVERFLOW_LENGTH` (`$1`, `\` escapes).
- Constants: `ANCHORED`, `ENDANCHORED` (**currently a bug: it equals `ANCHORED`**), `NOTBOL`, `NOTEOL`, `NOTEMPTY`, `NOTEMPTY_ATSTART` (`:373-384`). Decide §9 D-3.
- Lua side adds `regex.find`/`regex.match` on top; `find_offsets` subtracts 1 from every *end* delimiter.

**Pitfalls.** Table-with-metatable (not userdata). The `should_free`/`regex_compiled` ownership logic decides per-iterator free vs `__gc` free — mirror it exactly or you leak/double-free. Building a `luaL_checkudata`-style registry reference in a table's field `[1]` means that field is visible to Lua; keep it as lightuserdata.

**Crate choice.** `pcre2 = "0.2"` + `pcre2-sys` links the *system* `libpcre2-8` (good; the C build uses `-DPCRE2_STATIC` with the wrap). If `pcre2-sys` insists on building its own, fall back to raw FFI declarations in `cffi.rs` — it is ~15 functions and total control, and it removes a crate-version risk from the critical path. Prefer raw FFI if in doubt.

**Tests (`cases/regex.lua`).** type/metatable of `compile` output; `select('#', cmatch(...))` for no-match; capture offsets; `gmatch` with and without groups; empty capture → index; `gsub` count with `limit` 0/1/2; `$1` substitution; `compile("(")` → `nil, msg` non-nil; each constant value.

**Done when** `verify.sh` passes with `api/regex.c` deleted and the editor's syntax highlighting, find/replace, and `:find` all still work.

---

### W2 — `dirmonitor` + `custom_events` (175 + 76 + backend, ~330 lines)

**Goal.** `rust/src/dirmonitor.rs` (`luaopen_dirmonitor`) + `rust/src/custom_events.rs`. Delete `api/dirmonitor.c` and (Linux) `api/dirmonitor/inotify.c` from `meson.build`. **Keep `custom_events.c` in C for this wave** and call it through `cffi.rs` — it is shared with `system.c` dialogs and moving it early buys nothing. Move it in W5. (If you do port it here, `system.c` must then call the Rust version through `extern "C"`; that is fine too, but it is extra coupling for no gain.)

**Contract** (`src/api/dirmonitor.c`):
- Metatable `"Dirmonitor"`, `__index` = itself.
- `dirmonitor.new()` → userdata (SDL mutex + backend init).
- `:watch(path)` → integer watch id; starts the single check thread lazily on first watch.
- `:unwatch(id)` → nothing.
- `:check(callback, error_callback?)` → `nil` if the monitor was deinited (`length < 0`), `false` if no changes, `true` if changes were translated. Callback receives a **path string** in "multiple" mode or a **watch id number** in "single" mode (`:28-54`). Calls are deduped per `check` via a temp table at stack index 4; callback errors are routed to `error_callback` via `lua_pcall(..., 3)`.
- `mode()` → `"single"` when `get_mode_dirmonitor() == 1`, else `"multiple"`. **inotify is `"multiple"` (mode 2).**
- `__gc`: set `length = -1` under the mutex, deinit, join the thread, free.
- `luaopen_dirmonitor` calls `register_custom_event("dirmonitor", NULL)` and must `luaL_error` if that fails (`:167-169`).
- Thread: loops while `length >= 0`; calls the blocking `get_changes` **outside** the mutex; assigns under the mutex only if still 0; then `SDL_Delay(1)` + `push_custom_event("dirmonitor")` each iteration to wake the SDL main loop. The registered callback is a no-op, so `data/core/init.lua` never reacts to this event; `dirwatch:check` is polled from a Lua coroutine (`data/core/dirwatch.lua:151-166`).
- Backend interface: `init_/deinit_/add_/remove_/get_changes_/translate_changes_/get_mode_dirmonitor`. inotify: `poll` on the inotify fd plus a self-pipe; `translate` iterates `struct inotify_event` and calls `cb(wd, NULL)` when `mask & ~IN_IGNORED`; mode 2.

**Pitfalls.** The worker thread must **never touch the Lua state** — it only holds OS handles and pushes an SDL event. mlua's `Lua` is `!Send` by default; keep the thread function free of any `Lua` reference, and communicate through `push_custom_event` (C) or a `Mutex<Vec<...>>`. Reproduce the `volatile int length` memory ordering with an `AtomicI64` + `Mutex` for the payload. `data/core/dirwatch.lua:58-80` depends on `mode() == "single"`/`"multiple"` — see §9 D-2.

**Tests (`cases/dirmonitor.lua`).** `mode()` value; `:watch` returns an integer ≥ 0 for a temp dir; `:check` returns a boolean and `select('#', ...)`; `new()` metatable name; unwatch of an unknown id does not raise.

**Done when** treeview + autoreload still refresh on file creation/deletion.

---

### W3 — `process` (852 C lines; POSIX path only)

**Goal.** `rust/src/process.rs`; delete `api/process.c`.

**Contract** (`src/api/process.c`, wrapper `data/core/process.lua`):
- `process.start(cmd, opts?)` → `Process` userdata, **or raises** (never `nil, msg`). POSIX requires `cmd` to be a **table of strings** (`luaL_checktype`, `:376`). The Lua wrapper converts a string into `{cmd}` on POSIX and into a quoted command line on Windows, so the table requirement must be kept.
- `opts`: `detach` (bool), `timeout` (number, **seconds in the Lua path, ms in C**), `stdin`/`stdout`/`stderr` (integers in `[-3, 2]`, else `luaL_error "redirect to handles, FILE* and paths are not supported"`), `env` (**function**), `cwd` (string).
  - POSIX `env`: the function is called with an empty table and must return one string of `"K=V\0K=V\0\0"`; it is parsed in the child (`:509-521`).
- Child setup: `setpgid(0,0)` unless `detach`; `setsid()` if `detach`; `dup2`/close for redirections; `execvp`. A control pipe reports `exec` failure back to the parent so `start` can return it.
- Constants on the module table: `WAIT_INFINITE=-2`, `WAIT_DEADLINE=-1`, `STREAM_STDIN=0`, `STREAM_STDOUT=1`, `STREAM_STDERR=2`, `REDIRECT_DEFAULT=-1`, `REDIRECT_STDOUT=1`, `REDIRECT_STDERR=2`, `REDIRECT_PARENT=-3`, `REDIRECT_DISCARD=-2` (`:844-859`).
- `process.strerror(errnum)` → string.
- Methods (metatable `"Process"`, `__tostring` → `"Process"`): `pid()`, `returncode()` (0 values while running), `read(stream, size?)` (stream must be STDOUT/STDERR else `luaL_error`; default `READ_BUF_SIZE=2048`; `""` on EOF/EAGAIN; 0 values only on read error after SIGTERM), `read_stdout(size?)`, `read_stderr(size?)`, `write(data)` → bytes written (0 on `EAGAIN`), `close_stream(stream)`, `wait(timeout?)` → returncode or nothing on timeout (`WAIT_NONE=0` polls once; negative = infinite), `terminate()`/`kill()`/`interrupt()` → `true`, `running()` → bool, `__gc`.
- `__gc`: SIGTERM if running and not detached; then either sync wait+kill or queue to the kill-list worker.
- Module-level registry key `"__process_kill_list__"` holds a `process_kill_list_t` owning one SDL thread + mutex + cond (`:30, :86-93`). The module table's metatable `__gc` drains it and joins the thread (`:783-791`). Worker tasks retry every `PROCESS_TERM_DELAY=50ms`: SIGTERM while `tries < 3`, SIGKILL at `tries == 3`, then free.
- Signals map to `kill(-pid, sig)` (whole process group) on POSIX (`:250-294`).

**Pitfalls.** `wait`'s timeout unit mismatch is load-bearing. `close_stream`/`read` accept unchecked stream indices in C (out-of-bounds); bound them in Rust and note the behaviour change. `env` must be called with an empty table argument, not with the parent environment (POSIX); the Windows path differs. The kill-list worker is a Rust thread that must not hold a `Lua` handle. `read` must not block: set `O_NONBLOCK` on the parent-side pipe fds (`fcntl`) and reproduce the `EAGAIN → ""` behaviour exactly.

**Crate choice.** `std::process::Command` + `CommandExt::pre_exec` for `setpgid`/`setsid`/`dup2` (one `unsafe` block), `as_raw_fd` + `libc::fcntl` for non-blocking reads, `libc::waitpid(..., WNOHANG)`, `libc::kill(-pid, sig)`. The `env` function contract is easier to satisfy with the same `fork`/`exec` shape than with `Command::envs`, but `pre_exec` + `Command::env_clear`/manual `execvp` covers it. Raw `libc::fork` is the fallback if `pre_exec` fights you.

**Tests (`cases/process.lua`).** `select('#', p:returncode())` while sleeping; `p:read_stdout()` content and `""`→nil at EOF via `data/core/process.lua`; `p:wait(1)`; `p:kill()`; `process.strerror(2)`; each constant; `start({})`-style bad args raise.

**Done when** the command palette's process-backed commands and `:lua` file execution still work.

---

### W4 — `utf8extra` (1313 C lines + `unidata.h` 3753 lines)

**Goal.** `rust/src/utf8.rs` + generated `rust/src/unidata.rs`; delete `api/utf8.c`.

**Contract** (`src/api/utf8.c`, upstream starwing/luautf8). Exports: `offset, codes, codepoint, len, sub, reverse, lower, upper, title, fold, byte, char, escape, insert, remove, charpos, next, width, widthindex, ncasecmp, find, gmatch, gsub, match`, plus the field `charpattern`. This is a **Lua-pattern engine over codepoints**, not regex: `string.find`/`gmatch`/`gsub` semantics with `%a %c %d %l %p %s %t %u %x %g %w` classes, `len` returns `nil, pos` on invalid sequences unless lax, and pattern classes come from `unidata.h`.

**Generation, not transcription.** Write `scripts/rust-port/gen-unidata.lua` (or a C dumper) that reads the C tables and emits `rust/src/unidata.rs` with the same `range_table {first,last,step}` / `conv_table {first,last,step,offset}` contents for: `alpha, lower, upper, xdigit, space, unprintable, graph, compose, cntrl, digit, alnum_extend, punct, doublewidth, ambiwidth, tolower, toupper, totitle, tofold`. Add a test that the generated tables' lengths and checksums match the C ones. Width rules: double = 2, ambi = 1 or 2, compose/unprintable = 0, else 1.

**Pitfalls.** `charpattern` must be the same string. Invalid-UTF-8 handling differs per function; port function by function and diff against the C build with the C build still linked (`utf8extra` is used editor-wide via `data/core/utf8string.lua:5`).

**Tests (`cases/utf8.lua`).** `len`/`codepoint`/`offset`/`sub` on ASCII + CJK + invalid sequences; `width` for a double-width char; `find`/`gmatch` with `%a`; `charpattern` equality.

**Done when** `verify.sh` passes and cursor movement over CJK text still feels right.

---

### W5 — `system` (1546 C lines) + `custom_events` + native plugin loader

**Goal.** `rust/src/system.rs` + `rust/src/custom_events.rs` + `rust/src/plugin_api.rs`; delete `api/system.c`, `custom_events.c`.

**Contract.** The full surface is in §11.A and §11.B (≈42 functions + the event tuple table). Highlights and traps:
- `system` is registered as a **global** and in `package.loaded` (`luaL_requiref(..., 1)`). `luaopen_system` also creates the `NativePlugin` metatable **and** registers the `dialogfinished` custom event — both side effects must survive (`:1537-1543`).
- Two different "empty" conventions: `nil` (e.g. `get_file_info` failure) vs **0 values** (`absolute_path`, `get_clipboard`).
- `get_file_info` returns `{size, modified, type, symlink}`; `symlink` is only set when the target is a directory and `lstat` reports a link.
- `get_fs_type` is `#if __linux__`-gated (note: `#if`, not `#ifdef`) and uses `statfs` magic numbers; elsewhere always `"unknown"`.
- `exec` runs in the background (`"<cmd> &"` + `system()`).
- `fuzzy_match(str, ptn, files=false)` is a case-insensitive subsequence match with `score += run*10 - (case mismatch)`, `score -= 10` on a miss, and final `score - strLen*10`; **0 means no match**; `files=true` starts from the end. `data/core/common.lua:161-217` treats `> 0` as a hit.
- `path_compare(p1, t1, p2, t2)` sorts directories first, compares numeric runs as integers, treats `PATHSEP` as a terminator, ASCII-case-folds, and when lengths are equal puts lowercase first. Used by `data/plugins/treeview.lua`.
- Window functions take a `RenWindow` userdata as arg 1 and use the §5 accessors for scale. **`window_hit_info` is one global** shared by all windows (last `set_window_hit_test` wins, `:68, :508-511`).
- The cursor cache (`cursor_cache[SDL_SYSTEM_CURSOR_POINTER+1]`) is never freed.
- `ftruncate` uses `luaL_checkudata(..., LUA_FILEHANDLE)` and needs Lua ≥ 5.3.
- Dialogs are async: `open_file_dialog(win, id, opts?)` etc. return nothing and deliver `dialogfinished(id, "accept"|"cancel", ...)` later. `opts` = `{default_location, title, accept_label, cancel_label, allow_many, filters=[{name,pattern}]}`. `:1259` pushes the event; the Lua consumer is `data/core/init.lua:896`.
- `load_native_plugin(name, path)`: `SDL_LoadObject(path)`; registers a `package.native_plugins[name]` table with a `handle` field and the `NativePlugin` metatable (`__gc` = `SDL_UnloadObject`); looks up `luaopen_lite_xl_<basename>` (basename = after the last `.` in `name`) and calls `entrypoint(L, api_require)`; falls back to `luaopen_<basename>(L)`; errors if neither is found or if the entrypoint returns 0. See W9.

**Also in this wave.** Move `custom_events` to Rust (`register_custom_event` stores name→`{type, callback}` plus `#<base36 type>`→name, idempotent, first registration calls `SDL_RegisterEvents(1)`, property-locked) and delete `arena_allocator.c` in favour of mlua-owned strings for dialog results. `custom_events.h`'s callback type is `int (*)(lua_State *, SDL_Event *)` and is called **on the main thread inside the event loop**, so it can borrow the `Lua` handle — but only through a mechanism that does not create aliasing with the outer `Lua` borrow. Use `Lua::from_ptr`-style borrowing inside the callback or store the state pointer in a thread-local; do not hold a `Lua` across the loop.

**Pitfalls.** This wave is breadth, not depth. Expect 1–2 days of "one more function" only if the file-dialog callbacks and `poll_event` conversion are not done carefully. Do `poll_event` first (it is the hot path and defines the coordinate/return conventions), then the FS functions, then the window functions, then dialogs, then the plugin loader.

**Tests (`cases/system.lua`).** every return-shape gotcha in §4.1; `get_file_info` on a file/dir/symlink/missing; `absolute_path` 0-values failure; `fuzzy_match` score for a known pair; `path_compare` on `{"dir","file","a","b"}` permutations; `list_dir` on a temp dir; `get_sandbox()` string member; `errno`-style messages through `strerror`.

**Done when** `data/core` (130 `system.*` call sites) still works end to end and the contract diff is clean.

---

### W6 — `renwindow` + `rencache` (144 + 349 lines)

**Goal.** `rust/src/renwindow.rs`, `rust/src/rencache.rs`; delete `src/renwindow.c`, `src/rencache.c`. **`src/renderer.c` (FreeType) stays in C and is called through `cffi.rs`.**

**Contract.**
- Command buffer per window: `{type, size, rect}` header, `alignof(max_align_t)` aligned; types `SET_CLIP`, `DRAW_TEXT`, `DRAW_RECT`; payloads in `rencache.c:45`. Growth rate `1.2` from `CMD_BUF_INIT_SIZE = 512 KiB`; on allocation failure set `resize_issue` and drop commands for the frame.
- Grid diffing: `CELLS_X=80`, `CELLS_Y=50`, `CELL_SIZE=96`, two static cell buffers swapped per frame, FNV-1a 32-bit (`HASH_INITIAL=2166136261`, prime `16777619`), hash **full command bytes** into every overlapping cell, compare per cell, merge changed cells into dirty rects (`push_rect`), replay all commands per dirty rect, then `ren_update_rects`.
- `rencache_draw_text` returns `x + width` (points) and pushes a rect with an **unrounded** `x_offset` while `rect_to_grid` rounds elsewhere — preserve the asymmetry.
- `rencache_draw_rect` drops zero-size and non-overlapping commands; `rencache_set_clip_rect` intersects with the screen rect; text clipping is manual inside the glyph loop (`renderer.c`), not SDL clip.
- Window: surface mode vs `LITE_USE_SDL_RENDERER` mode; `renwin_update_scale` sets `scale_x/y = surface.w/window_w`; the SDL-renderer path asserts an integer, equal-xy scale.
- `renwindow._persist()` stores a single global `persistant_window`; `__gc` skips destroying it; `_restore()` returns a new userdata wrapping the same pointer.

**Pitfalls.** `#[repr(C)]` structs are mandatory because the bytes are hashed (`rencache.c:22-70`) — padding differences change the cache-hit behaviour (not correctness, but determinism, which the harness checks). The static cell buffers assume **one window**; keep that assumption and comment it (`shortcut:` comment). Because `rencache` calls the C renderer (`ren_draw_text`, `ren_font_group_get_width`), the Rust side needs those `extern "C"` declarations before W7.

**Tests.** Hash/dirty-rect determinism for a fixed scene (see §7.4): expose a debug flag that prints the dirty rect list per frame, and diff it. Also: `_persist`/`_restore` identity, `__gc` non-destruction, `get_size` in points.

**Done when** the editor renders correctly and the dirty-rect dump is stable across runs and matches the C baseline.

---

### W7 — `renderer` (896 C lines, FreeType) + `api/renderer.c` (434)

**Goal.** `rust/src/renderer.rs` (rasteriser) + `renderer_api.rs` (font userdata/Lua binding); delete `renderer.c` and `api/renderer.c`. This is the hardest and most visual wave — do it with everything else already portable.

**Contract.** §11.C has the full Lua surface; internals:
- Glyph cache: `CharMap` codepoint→glyphID (`CHARMAP_ROW=128`, sentinel `-1`), `GlyphMap` metrics `[3][128]`, atlases `[2]` with width = glyph width + `FONT_WIDTH_OVERFLOW_PX=9`, surfaces `width × 96*h` bump allocators, glyph formats Grayscale (`INDEX8`) / Subpixel (`RGB24`), surface reuse with a last-100 MIN-waste fit.
- Load/render options: mono vs `FT_LOAD_TARGET_LIGHT`/`NORMAL`; `FT_LOAD_NO_HINTING` when hinting is none; LCD filter weights `{0x10,0x40,0x70,0x40,0x10}` unless hinting is none; style transforms via `FT_Outline_Embolden`/`FT_Outline_XY`; italic shear; `x_translation = bitmap_idx * (64/3)` for subpixel positioning.
- Blending: `(color.c*src.c*color.a + dst.c*(65025 - src.c*color.a) + 32767) / 65025`; destination alpha preserved; `alpha == 255` → `SDL_FillSurfaceRect`, else a 1×1 surface blit. Subpixel index = `fmod(pen_x, 1.0) * 3`, computed across the whole string.
- Font groups: first font with the glyph wins, **whitespace always uses `fonts[0]`**, missing glyph retries box char `0x25A1` (unless the codepoint is that or whitespace). Max 10.
- Metrics: height, baseline, `underline_thickness = height/14` when absent, `space_advance` from `' '`. `set_size` resets `tab_size` to 2.
- `_persist`/scale: `ren_font_group_get_width` divides by `fonts[0]->scale` in SDL-renderer mode; `update_font_scale` re-rasterises when the surface scale changes.

**Lua-side traps.** A font **group is a table**, not userdata; `Font:copy` defaults its size to the group **height**, not the size; `-1` sentinels mean "keep original"; the colour table accepts `{r,g,b}` or `{r,g,b,a}` and errors with `table[i]: number expected, got <type>`; `Font.load` errors as `"failed to load font: %s"` with `SDL_GetError()`. `style.lua:38-42` is the main `font.load` caller.

**Crate choice.** `freetype-rs = "0.38"` owns `Library`/`Face` and supplies glyph/metrics APIs. `ft.rs` supplements its missing LCD-weight function, metrics-only flag and SDL stream callbacks (Unicode filenames). **User decision after testing:** keep Meson's FreeType 2.14.1 in W7. The crate's `bundled` feature builds 2.13.2 and changed six JetBrains Mono hinted hashes; system 2.14.3 also changes LCD pixels. No pixel expectations were updated. Meson prepends its uninstalled pkg-config directory for Cargo, so the crate probes the selected library and a fallback build does not require a system FreeType development package (checked with `PKG_CONFIG_LIBDIR=/nonexistent`). For the recorded baseline use `-Dforce_fallback_for=freetype2` (or `--wrap-mode=forcefallback`); other FreeType builds are supported but need their own C comparison. Pure-Rust (`swash`, `fontdue`) remains rejected for v1. W8 must explicitly resolve FreeType version/configuration before moving build ownership to Cargo.

**Tests.** `tests/contract/cases/renderer.lua` records 200 C observations: font API/errors/metrics and logical-RGB surface hashes for antialiasing × hinting, sizes, fractional pen positions, styles, blending, clipping, tabs, missing glyphs and groups. Only the build-specific font-directory error path is normalized. Additional assertions check deferred-draw font/group GC anchoring, release after replay, and malformed font tables. All 14 suites pass in debug, release and `-Drenderer=true` using FreeType 2.14.1. Nine compiling mutants are caught: subpixel index, glyph advance, tab reset, stale cache after resize, fallback-box width, red blending, baseline, monochrome expansion and missing font anchoring. Real offscreen editor startup/render/shutdown and 200 valid/invalid font-load cycles pass in all three builds with no FD growth.

**Status.** Implemented and automatically verified. Independent review found and fixed a shared-reference/mutable-borrow alias in the drawing loop and missing FreeType diagnostic strings. Deliberate safety differences: unknown hinting formats the supplied string rather than a mis-typed enum pointer; empty/mutated font tables raise instead of dereferencing null. **Still unverified:** the app looks the same in a real window by eye, retina/HiDPI scale > 1, macOS/Windows. The screenshot-by-eye acceptance check remains manual.

---

### W8 — `main` + `api/api.c` + `arena_allocator` + `utfconv` → full Rust binary

**Goal.** `rust/src/main.rs` (a `bin` target) replaces `src/main.c`, `src/api/api.c`, `src/arena_allocator.c`, `src/utfconv.h`; Meson is dropped for Linux; `cargo build` produces `lite-xl`.

**Contract** (`src/main.c:99-230`, `data/core/start.lua:1-43`):
- Order: `SDL_SetAppMetadata("Lite XL", LITE_PROJECT_VERSION_STR, "com.lite_xl.LiteXL")` → `SDL_Init(SDL_INIT_EVENTS)` → `SDL_SetEventEnabled(DROP_FILE)` → `atexit(SDL_Quit)` → `ren_init()` → `init_custom_events()` → `luaL_newstate` → `luaL_openlibs` → `api_load_libs(L)`.
- Globals set from C before `start.lua`: `ARGS` (1-based argv table), `PLATFORM` (`SDL_GetPlatform()`), `ARCH` (`LITE_ARCH_TUPLE`), `RESTARTED` (bool), `EXEFILE` (exe path, fallback `argv[0]`), optional `MACOS_RESOURCES`.
- Embedded chunk semantics: `os.exit` wrapped to default `close=true`; `HOME = os.getenv(LITE_OS_HOME)`; `DATADIR = MACOS_RESOURCES or (prefix .. '/share/lite-xl') or exedir .. '/data'`; `dofile(DATADIR/core/start.lua)`; `require(os.getenv('LITE_XL_RUNTIME') or 'core')`; `core.init()`; `core.run()`.
- Error path: write `error.txt` (or `USERDIR/error.txt` + `core.on_error`), print `debug.traceback(nil, 2)`, `system.show_fatal_error`, `os.exit(1)`.
- Restart: if `core.restart_request` is truthy → `lua_close`, `rencache_invalidate()`, `has_restarted = 1`, jump back to state creation. **The Lua state is recreated per restart and the SDL window survives** (`renwindow._persist`). `RESTARTED` must be set correctly across iterations. `autorestart.lua` depends on this.
- `get_exe_filename` platform logic: Linux `/proc/self/exe` readlink; `SIGPIPE` ignored on POSIX.
- Defines to carry over: `LITE_PROJECT_VERSION_STR="<ver>"` (version = 2.1.7 + ` (git-<8>)` when not a release build), `LITE_ARCH_TUPLE="<cpu_family>-<system>"`, `LITE_USE_SDL_RENDERER` when the `renderer` option or darwin.
- `data/core/start.lua` also reads `VERSION` (substituted `@PROJECT_VERSION@`) and `SCALE` (env `LITE_SCALE`/`GDK_SCALE`/`QT_SCALE_FACTOR`).

**Pitfalls.** `atexit(SDL_Quit)` and the restart path interact; the persisted window is never destroyed, by design. `lua_close` must run before `SDL_Quit`. Keep `error.txt` semantics (a stuck `error.txt` is the primary user-facing diagnostic).

**Also in this wave:** the `bin` target means the crate needs a second `crate-type`; do not split into two crates unless it helps. `requirements` for the Nix package: add `cargo`/`rustc`, vendor the registry (§9 D-5).

**Decisions (user, before W8 began).** FreeType: the crate's bundled 2.13.2, with the renderer baseline re-recorded and the drift reviewed. Lua and PCRE2: vendored crates (`mlua/vendored` = Lua 5.4.9, `pcre2-sys` = PCRE2 10.46, forced static by `.cargo/config.toml`). Meson: **kept for non-Linux platforms**, so the C that only they need stays (`main.c`, `api/api.c`, `api/renwindow.c`, `arena_allocator.c`, `utfconv.h`, the macOS/Windows/BSD `dirmonitor` backends, `bundle_open.m`). Nix: dev shell gains `cargo`/`rustc`, and (finished after the first W8 commit) the package builds with `buildRustPackage` from `Cargo.lock`, which needs zlib in `buildInputs` for the bundled FreeType's libpng.

**Done when** `cargo build --release` produces a working `lite-xl` with no C in the link, and restart still works.

**Result.** Two feature sets in one crate: `standalone` (default; the `lite-xl` binary) and `module` (the staticlib Meson links, as before). `rust/src/app.rs` ports `main.c` and `api.c` (startup order, globals, embedded Lua chunk, restart loop, `SIGPIPE`, `/proc/self/exe`); `rust/src/renwindow_api.rs` ports `api/renwindow.c`; `rust/build.rs` supplies version/arch strings and the window icon (extracted from `icon.inl`, still the single source for Meson). `scripts/install-linux.sh` replaces `meson install` on Linux; `verify.sh` now builds with Cargo; `scripts/rust-port/startup-check.sh` covers what the module suites cannot (globals, restart with a persisted window, exit code, `error.txt`).

**Findings.**
- `#[mlua::lua_module]` requires mlua's `module` feature (a different Lua link mode), so `lib.rs` defines the `luaopen_*` entry points with a small `lua_entry!` macro that calls `Lua::entrypoint1`, the same call the attribute expands to. They stay `#[no_mangle]` under `module` because `api.c` calls them by name.
- **Behaviour drift, reviewed and accepted:** FreeType 2.13.2 changes exactly six hinted JetBrains Mono hashes (`mono none/grayscale/subpixel` × `slight`/`full`, e.g. `961b4f10/543 → edb35ba6/541`); `mono none none`, every Fira case and the fira+icons group are unchanged. Re-recorded and stable over 3 runs. Lua moves 5.4.6 → 5.4.9 and PCRE2 10.44 → 10.46 (the regex, utf8 and tokenizer suites are unchanged). The Meson build still uses FreeType 2.14.1, so **the two builds now render hinted mono text differently**; the committed baseline matches Cargo. Unverified: the Meson/macOS build against it (its `renderer` suite would differ in those six lines).
- The Lua 5.4.6 `lua-unicode.diff` (Windows UTF-8 wrapper) is irrelevant on Linux and is not applied by the vendored crate.
- Exiting with a live window under `SDL_VIDEODRIVER=offscreen` segfaults in `SDL_Quit`. Pristine upstream (`234fa099`, 10/10) and the Meson build do too, so it predates the port; `startup-check.sh` uses the `dummy` driver for that case.
- Mutation check of `app.rs`/`renwindow_api.rs`: 6 of 6 caught by `startup-check.sh` (RESTARTED always false, never set after a restart, restart request ignored, `__gc` destroying the persistent window, `_restore` returning nil, a wrong module registered). An independent review found no parity defects against `main.c`/`api.c`/`renwindow.c` (the embedded Lua chunk was byte-compared) and four low items, all fixed.
- A release binary is 3.7 MB, links only SDL3 and libc/libm, and contains no `lxl_test` hooks.
- The Meson build still works with the new `module` feature split: 8 non-hook suites pass against its binary.

**Open:** macOS/Windows (still Meson), AppImage/packaging scripts still assume Meson, a real-window run by eye.

---

### W9 — Native plugin ABI (`resources/include/lite_xl_plugin_api.h`)

**Goal.** `rust/src/plugin_api.rs` implementing `api_require` so `system.load_native_plugin` keeps working (D2).

**Contract** (`src/api/system.c:974-1098`):
- `XL` is `void *(*)(const char *symbol)`. `api_require` answers a **fixed table** of `lua_*`, `luaL_*`, `luaopen_*` and `"api_load_libs"` names, version-gated by `LUA_VERSION_NUM`, returning `NULL` for anything unknown. The table is built from `P(...)/U(...)/S(...)` macros (43 entries in the 5.4 subset).
- Entrypoint resolution: `luaopen_lite_xl_<basename>` first, `luaopen_<basename>` fallback; must return non-zero or the loader errors.
- The plugin header resolves symbols with `IMPORT_SYMBOL` from the `XL` function; other TUs see declarations only.

**Pitfalls.** This is the one place where `unsafe` FFI is unavoidable: the resolver must return real function pointers to the Lua C API. Resist reimplementing Lua's API — resolve against `mlua::ffi`/`mlua_sys` functions, which are the same symbols the linked Lua provides. The symbol table must include everything an existing plugin might use; a missing entry is `NULL` and the plugin will crash. Verify with at least one real plugin (e.g. a small `lite-xl-lsp`-style or a purpose-built test plugin) both the legacy and the XL entrypoint paths.

**Done when** a test `.so` with `luaopen_lite_xl_test(L, XL)` loads via `require` and can call `luaL_newlib`-equivalent functions through `XL`.

**Status: done.** The resolver itself (`api_require` in `rust/src/sysplugin.rs`, address table `rust/src/plugin_syms.rs` generated from the header by `scripts/rust-port/gen-plugin-syms.lua`) landed in W5, so no `plugin_api.rs` was needed. W5's `sysplugin` suite only exercised 27 of the header's 155 symbols, and a wrong address in any other entry would have gone unnoticed (a plugin would crash or misbehave). W9 closes that gap.

**What W9 added.**
- `tests/contract/plugin/apiplug.c`: a real plugin built against the shipped header without linking Lua, calling 46 test functions that cover the stack, conversions, arithmetic, tables, metatables, calls and errors, load/dump, coroutines, to-be-closed slots, warnings, GC, allocator, the whole debug interface (hooks, locals, upvalues), the auxiliary library (arg checks, buffers, refs, requiref, traceback), a nested `luaL_newstate` and every `luaopen_*`.
- `tests/contract/cases/pluginapi.lua` + `baseline/pluginapi.txt` (81 observations), **recorded from pristine upstream `234fa099` (C core, Lua 5.4.6)**, identical over 3 runs, then run unchanged against the Rust build (debug, release, `sdl_renderer`) and against the Meson `module` build.
- Mutation check: swapping the address of 16 table entries (`lua_settop`/`lua_gettop`, `lua_pcallk`/`lua_callk`, `lua_resume`/`lua_status`, `luaL_ref`/`luaL_unref`, hooks, upvalues, buffers, `luaopen_math`/`luaopen_string`, `luaopen_os`/`luaopen_io`, ...) is caught in all 16. The first run missed `luaopen_math`↔`luaopen_string` because the record only showed a type; it now records each library's entry count, and the swap is caught.

**Finding (a real ABI difference, now fixed).** The vendored Lua from the `lua-src` crate is built with `LUA_COMPAT_5_3` (and, in debug builds, `LUA_USE_APICHECK`); Lite XL's Meson build uses neither. A plugin or Lua script on the Cargo build could therefore see `math.pow`, `math.cosh`, `math.sinh`, `math.tanh`, `math.frexp`, `math.ldexp`, `math.log10` and the `__le`-via-`__lt` fallback that do not exist in any released Lite XL, and `luaL_*` misuse would be tolerated in debug builds where release builds crash. `.cargo/config.toml` now passes `CFLAGS=-ULUA_COMPAT_5_3 -ULUA_USE_APICHECK` with `force = true` (cc-rs appends CFLAGS after the crate's `-D` flags, verified in `cc-1.8.0`); the suite records these facts, so removing the flags fails `pluginapi` (exit 134). Checked in the Nix package too, and with a hostile ambient `CFLAGS` from another directory. Caveat: Cargo finds this file from the *current directory*, not `--manifest-path`, so `verify.sh` now `cd`s into the repo; a build driven from outside it (some packaging setups) must set the same `CFLAGS` itself. Side effect: with `CFLAGS` set, cc-rs drops its default warning flags for the other C crates.

**Known remaining differences (accepted, not hidden).**
- `lua_closethread` resolves in Rust but not in the C resolver's table (the header declares it; the C table is hand-kept and lagged). Covered by `sysplugin`.
- The vendored Lua is 5.4.9, the Meson build's is 5.4.6. The header says 5.4.6; every constant and struct layout the header defines (`lua_Debug`, `luaL_Buffer`, `LUAL_NUMSIZES`, `LUAI_MAXSTACK`, ...) was diffed against 5.4.9 and matches.
- Both builds export no `lua_*` symbols (plugins must go through `XL`), so plugins that link Lua directly are unsupported, exactly as the header states.

**Review follow-ups (independent reviewer):** unbounded `memcpy` in the dump writer now bounds-checked; two records that could not fail (`closure result`, `where`) now record real values (`where` is also recorded from a Lua caller, where `luaL_where` is non-empty); `force = true` and the `cd` in `verify.sh`. Remaining known limits: the baseline is Linux/POSIX-only (`luaL_execresult` wait-status encoding, `strerror` text, `%g` locale — the harness never calls `setlocale`); the suite leaves the shared Lua state in incremental-GC mode with unusual parameters.

**Not verified:** plugins other than this purpose-built one (no third-party `.so` is available here), macOS/Windows plugin loading (`.dylib`/`.dll` paths in `system.load_native_plugin`).

### W1 — `regex` (done)

`rust/src/pcre.rs` (hand-written PCRE2 8-bit FFI) + `rust/src/regex.rs`; `src/api/regex.c` deleted from `src/meson.build`. `src/api/api.c` needed no change, as predicted.

- **Parity:** `scripts/rust-port/verify.sh` green. `regex` (54 observations) and `tokenizer` (11 lines) are byte-identical to the C baseline. Covers compile options, offsets, negative offsets, `NOTBOL`, empty captures, `gmatch` with/without groups, `gsub` global/limited/growing/escapes/empty matches, `re[1]`, method-call syntax, constants, and error text.
- **`tokenizer` suite added:** drives the real `core.syntax` + `core.tokenizer` + `plugins.language_lua` path (the heaviest regex consumer) and threads the tokenizer state across lines, as `Highlighter` does.
- **D-4 resolved:** raw FFI, not the `pcre2` crate. 13 extern functions, no build-script or vendoring risk, and the exact C semantics (including `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH`) stay visible.
- **D-1 resolved:** compiled patterns stay a Lua *table* with the module table as metatable (`type(re) == "table"`, `type(re[1]) == "userdata"`, `getmetatable(re) == regex`), because `data/plugins/detectindent.lua:104,117` inspects the value and plugins use method syntax.
- **D-3 confirmed:** `ENDANCHORED` still aliases `ANCHORED`, with a `shortcut:` comment in `rust/src/regex.rs`.

**Pre-existing C bug found (not introduced by the port):** `f_pcre_match` calls `lua_rawgeti(L, 1, 1)` unconditionally, so passing a *string* pattern to `regex.cmatch` reads a Lua string as a table and segfaults. `data/core/regex.lua:21` compiles the pattern first, so the Lua layer never hits it, and the Rust port handles string patterns correctly. The C fix would be to move that `lua_rawgeti` into the `LUA_TTABLE` branch. The contract suite therefore always compiles before `cmatch`.

**Known deviation (accepted):** C raises errors with `luaL_error` (a plain string); mlua raises a userdata whose `tostring` is `"<message>\ntraceback"`. The message text is made byte-identical (`bad argument #2 to 'regex.cmatch' (string expected, got nil)`), so only `type(err)` and the extra traceback line differ. `tests/contract/cases/regex.lua` normalises this. Revisit if a plugin ever string-matches an error value.

**Lesson for later waves:** the C `gsub` keeps the substituted subject and its *length* in separate variables. Taking the whole buffer as the next subject produced trailing NULs; the port must truncate to the written length.

### W4 — `utf8extra` (done)

`rust/src/utf8.rs` (≈1.7k lines incl. the codepoint pattern engine) + generated `rust/src/unidata.rs`; `src/api/utf8.c` deleted from `src/meson.build`. `src/unidata.h` stays in the tree as the generator's source of truth. `api.c` unchanged.

- **Parity:** `utf8` (780 observations: every export × 18 strings incl. malformed UTF-8, 40 patterns through find/match/gmatch/gsub, 15 digests over ~14k codepoints each for case conversion/width/class tables) and `utf8_edge` (717 observations: malformed/truncated patterns, empty subjects, `math.maxinteger`/`mininteger` arguments, replacement kinds, escapes, deep patterns, 100 kB subjects) are byte-identical to the C baseline. Both ran in debug **and** release (`-Dbuildtype=release`) builds.
- **Tables are generated, not transcribed:** `scripts/rust-port/gen-unidata.lua [--check]` parses `src/unidata.h` into `rust/src/unidata.rs` (18 tables, entry counts + FNV-1a checksums in comments). The first run found 17 of 18 tables because `%w` excludes `_` in Lua patterns (`alnum_extend_table`); the script asserts exactly 18.
- **Delegation failed, so don't repeat it:** a `worker` subagent given the 1.3k-line port timed out at 600 s and left a compressed, *functionally wrong* file (pattern engine stubbed as substring search). It was discarded. Porting by hand against the 1.5k-line oracle was faster and correct. Subagents are fine for investigation, not for large ports that need the C open.

**Behaviours that were easy to get wrong (all caught by the oracle):**
- `check_capture` does `l -= '1'` — the *character*, not the number 1 — so `%1` backrefs index capture 0.
- `utf8_offset` computes `s + offset - 1` and can sit one byte *before* the string; Rust needs a signed index there (usize underflowed).
- The C reads `*p` one past the end (the NUL terminator) in several places (`%b`, `escape`); the port uses `decode_at_end` for those and must not use the strict decoder.
- C `int` truncation is observable: `relat` takes `int idx`, `widthindex`/`width` use `int` accumulators, so `math.maxinteger` acts like `-1`. Port with `as i32` and `wrapping_*`.
- `matchdepth` is decremented/incremented around a function with many returns; Rust keeps it in a wrapper (`match_at` → `match_inner`) so it always balances. Note the C tests the *old* value (`matchdepth-- == 0`).
- Debug builds panic on integer overflow where C wraps; `lua_Integer` index arithmetic uses `wrapping_*`. **`panic = "abort"` means any such panic would kill the editor**, which is why `utf8_edge` exists.

**Performance (release, same machine, 200-line sample, lower is better):**

| op | C | Rust | |
|---|---|---|---|
| `find` plain | 0.2 | 1.1 ms | 5× slower; `lmemfind` uses `windows().position` instead of `memchr`+`memcmp` |
| `find` pattern | 2.4 | 2.7 | ≈ |
| `gsub` | 43 | 57 | 1.3× |
| `gmatch` | 22 | 39 | 1.7× — re-borrows both strings and rebuilds `MatchState` every call |
| `len` | 17 | 26 | 1.5× — `decode` is not `#[inline]` |
| `sub` | 180 | 280 | 1.6× — most of it is mlua call overhead (20k calls) |
| `lower` | 21 | 23 | ≈ |
| `width` | 106 | 125 | 1.2× |

Correctness first; nothing here is per-frame hot except `sub`/`len`/`find`/`width` called from `docview`. Candidates if it matters: `memchr` for plain find, `#[inline]` on `decode`/`next_i`, cache `MatchState` in the `gmatch` closure, avoid `Variadic` allocation (take fixed arg tuples). Not done: out of scope for a port, and unmeasured in the real editor.

**Deviations (accepted):** error values are mlua userdata, not C strings (message text matches for most paths; the suites record status only). `to_utf8`/`lua_tolstring` on a non-string in `lower`/`width` was not exercised for exotic types beyond number/string.

### W2 follow-up — `dirmonitor` re-entrancy bug (found by a user, fixed)

**Symptom** (real use, `autoreload`): `core/dirwatch.lua:146: bad argument 'self' to 'DirMonitor.watch': error mutably borrowing userdata`.

**Cause.** `dirwatch:check` calls `self:unwatch(path)` and `self:watch(path)` **from inside the `check` callback** (to re-arm a watch on a changed file). In mlua, `check` is a `&self` method and holds a shared borrow of the userdata while it calls Lua; `watch` was a `&mut self` method (it stored the worker's `JoinHandle`), so the nested call could not take the exclusive borrow. The C has no borrow checker and does not care. The contract suite never made that call from inside a callback, so it passed.

**Fix.** The worker handle is now `Mutex<Option<JoinHandle>>` and `watch` is a `&self` method. `tests/contract/cases/dirmonitor.lua` gained a re-entrancy case (`unwatch`, `watch`, `mode` from inside the callback), the baseline re-recorded from pristine upstream (3 identical runs); reverting the fix makes the suite fail (`watch` → `false`). End to end, `dirwatch` re-arming a changed file: 6/6 changes and 0 errors, vs 1 change and 1 error on the previous binary.

**Lesson for any port to mlua.** A `&mut self` userdata method that can be called while the same object is already borrowed (a `&self` method that calls back into Lua) turns a harmless C re-entrancy into a runtime error. Audited: no other userdata method runs a Lua callback (`process` never calls Lua; `utf8` callbacks and the `env` function are not methods on userdata). A nested `check` inside `check` is deliberately not pinned: the C re-reads the same unconsumed buffer, the Rust takes it first; nothing calls it that way.

### W10 — Linux only: remove Meson and every remaining C source (done)

**Decision (user, 2026-10-17):** only Linux matters for now. macOS and Windows support is dropped rather than ported, so everything that existed only for them, or only for the Meson build, is deleted. Restoring them later means porting the platform pieces (FSEvents/kqueue/Win32 dirmonitor, `bundle_open.m`, the Win32 process/system branches) to Rust and adding packaging back; upstream `234fa099` has all of it.

**Deleted:** all of `src/` (`main.c`, `api/*.c`, the dirmonitor backends `fsevents.c`/`kqueue.c`/`win32.c`/`dummy.c`/`inotify.c`/`inodewatcher.cpp`, `bundle_open.m`, `arena_allocator.*`, `unidata.h`, the headers and `meson.build`); `meson.build`, `meson_options.txt`, `data/meson.build`, `scripts/meson.build`; the Meson wraps and `subprojects/packagefiles` (the extracted `subprojects/*/` trees are ignored, untracked, and can be removed by hand); `scripts/build.sh`, `common.sh`, `package-appimage.sh`, `package-innosetup.sh` and `innosetup/`, `generate-release-notes.sh` with `resources/release-notes.md`; `resources/{macos,windows,cross}`, `shell.html` (wasm), `icons/icon.{icns,ico,rc}`; `.github/` (CI built with Meson on all three platforms; no CI replaces it yet); `rust/src/probe.rs` and `scripts/rust-port/gen-unidata.lua` (its input `src/unidata.h` is gone, so `rust/src/unidata.rs` is now the source).

**Kept:** `resources/icons/icon.inl` (build.rs reads it), `resources/include/lite_xl_plugin_api.h` (the plugin ABI; `gen-plugin-syms.lua` and the fixtures use it), the C plugin fixtures in `tests/contract/plugin/` (they *are* native plugins), `scripts/rust-port/fixtures/hit_test_reference.h` (the C oracle for `hit-test-check.sh`), `generate_header.sh`, `data/core/start.lua`'s AppImage `OWD` handling (it is upstream `data/`, harmless).

**Rust:** the `module`, `standalone` and `probe` features are gone; their dependencies are unconditional and the crate is an `rlib` plus the `lite-xl` binary (no `staticlib`). `lib.rs` has the single `compile_error!` for non-Linux targets. The two macOS-only `Cmd+W` quit-flush blocks in `sysevents.rs` and the Windows/macOS icon skip in `renwindow_api.rs` are removed. `lua_entry!` no longer exports `#[no_mangle]` symbols (nothing in C calls them now). `Cargo.lock` loses the `staticlib`-only/optional entries.

**Flake:** systems are `x86_64-linux` and `aarch64-linux`; the devShell drops meson/ninja and the system pcre2/freetype (Cargo builds its own); `lua5_4` stays for `gen-plugin-syms.lua`.

**Not verifiable here:** that nothing outside this repo (packagers, CI forks) relies on the deleted scripts.

### Upstream bug fixed on this branch — `common.fuzzy_match` with table items

**Symptom** (real use, typing with the `lsp` plugin): `core/common.lua:156: attempt to compare two table values`, from `autocomplete.lua:364`.

**Not a port bug.** It reproduces on pristine upstream `234fa099`. `fuzzy_match_items` sorted `{ text = item, score = ... }` records with `a.text < b.text` as the tie-break; autocomplete's items are tables (`{ text, info }` with `__tostring`, no `__lt`), so the first two suggestions with equal scores raised. Plain strings were unaffected.

**This is a deliberate behaviour change in `data/`**, the first one on this branch (until now `data/` was byte-identical to upstream). The records now carry `key = tostring(item)` and tie-break on that. String items sort exactly as before.

**Tests.** New `fuzzy` suite (`tests/contract/cases/fuzzy.lua`, 17 observations). Its string rows were recorded from the unmodified upstream build and are unchanged by the fix; the object rows crash on upstream (`ERR`) and are recorded from the fixed code, where they equal the string behaviour. With the fix reverted the suite fails (exit 2); with it, passes. End to end: typing into a real `DocView` with the real `autocomplete` plugin raises on the old `common.lua` and not on the new.

**Consequence for the baselines:** the Rust build now intentionally differs from upstream on this one function. Any future comparison against an upstream binary should expect the `objects, ...` rows to differ.

### W3 — `process` (done)

`rust/src/process.rs` (raw `libc` fork/exec, not `std::process`) replaces `src/api/process.c`; `api.c` unchanged. Adds the `libc` dependency.

- **Parity:** the `process` suite (86 observations, baseline recorded from C first) is identical in debug and release builds and stable over repeated runs. It drives `core.process` (the real wrapper): constants (floats, as C's `lua_pushnumber`), exit codes, stdout/stderr, sized reads and EOF, `write`/`close_stream`, stream objects, every redirect, `cwd`, `env`, missing executables, bad arguments, `WAIT_DEADLINE`, all three signals, detached children, and a dropped userdata killing its child.
- **Checked outside the harness:** a live child is killed at `lua_close` (module `__gc` joins the kill-list worker), a `detach = true` child survives it, and 300 started processes plus 300 failed `exec`s leave the editor at the same 5 fds with no zombies.
- **Why raw `fork`, not `std::process`:** the `env` contract (a block of `K=V\0…\0\0` applied in the child), the exec-failure control pipe, and `setpgid`/`setsid` ordering are one-to-one with the C. `pre_exec` would have fought each of them. The child path uses only libc calls.
- **Kill-list:** a `static` worker thread behind a `Mutex`+`Condvar`s, started at module load. The C keeps it in the Lua registry; equivalent because there is one Lua state. It never touches Lua. A `Drop` on the userdata replaces `__gc`.

**Deviations (all on paths where the C is undefined or crashes):**
- Closed fds use `-1`, not `0` (a real fd in C: writing to a closed stdin wrote to fd 0).
- `stdout = REDIRECT_DEFAULT` read `child_pipes[-1]` in C; it now means the stream's own default.
- `close_stream` with a stream outside 0–2 indexed out of bounds in C; it now raises.
- A negative read size and an empty command (`start({})`, NULL `argv[0]`) now fail cleanly instead of misbehaving.
- An env block entry without `=` ended the child via a NULL write; it now stops parsing.
- Errors are mlua userdata rather than strings (same accepted deviation as W1/W4).

**Not covered:** `process.start` with a string command is rejected in C and Rust alike (only the wrapper accepts one). `interrupt()` is asserted via the child dying, not the signal number. The Windows branch is not ported (Linux-first).

**Lesson:** two of my own checks gave false results (a busy-wait that never ended, and counting fds while spawns were still in flight). Synchronize on a ready file, not a sleep.

### W2 — `dirmonitor` (done; `custom_events` deliberately not)

`rust/src/dirmonitor.rs` (Linux/inotify only, `libc` directly) replaces `src/api/dirmonitor.c` **and** `src/api/dirmonitor/inotify.c`. `src/custom_events.c` stays in C and Rust calls it through `extern "C"` (`register_custom_event`, `push_custom_event`, with `CustomEvent` declared `#[repr(C)]`), as the plan recommended. `api.c` is unchanged.

- **Parity:** the `dirmonitor` suite (43 observations, baseline recorded from C first) is identical in debug and release, stable over 5 repeated runs. It exercises real inotify events: create, burst (deduplicated to one report per watch per check), delete, move across directories, unwatch, callback errors routed to the error callback and swallowed without one, bad arguments, independent monitors, and 300 create/drop cycles.
- **Checked outside the harness:** the real `core.dirwatch` + `system.wait_event` loop sees both watched paths and the event loop is woken by the worker's custom event; shutdown with 5 live monitors exits 0 five times; 200 create/drop cycles leave fds and threads unchanged (5 / 2).
- **Meson:** on `inotify` neither `api/dirmonitor.c` nor the backend is compiled. Other backends (fsevents, kqueue, win32, dummy) keep the C module, so macOS/Windows builds are unaffected. Marked with a `shortcut:` comment.

**Deviations (the first two fix real C bugs):**
- **The C's inotify walker is wrong.** It steps `sizeof(struct inotify_event)` (16) instead of `16 + len`, so for any event with a file name it also "reports" garbage watch ids made of name bytes (observed: real id `1`, delivered `1,6f6c5f61,742e656d` — the ASCII of the file name). `dirwatch.lua` ignores unknown ids, so nothing visible breaks. Rust walks events correctly; the suite only records known ids so both agree. This is a bug fix, not parity.
- **C shutdown races its worker.** `__gc` closes the inotify and pipe fds *before* joining the thread blocked in `poll` on them, and relies on closing to wake it. Rust writes to the self-pipe, joins, *then* closes.
- `check` releases the mutex before running Lua callbacks (the C holds it, so a callback calling back into the monitor would deadlock).
- The inotify fd is non-blocking and `poll`/`read` retry on `EINTR`/`EAGAIN`.
- The module table is a plain table with `new` and `mode`; the C returned its metatable (with methods on it). `data/` only calls `dirmonitor.new()` and instance methods.

**Not done / not covered:** `custom_events.c` itself (moves in W5 with `system`). The macOS, Windows and kqueue backends. The `dummy` backend's `mode() == "single"` path is untouched C. A `"single"`-mode Rust backend does not exist.

---

## 9. Open decisions ledger (append as they settle)

| id | decision | status | recommendation |
|---|---|---|---|
| D-1 | Compiled `regex` object as a Lua **table** with the module table as metatable (vs a Rust userdata) | open → unless a Lua file indexes `re[1]`, a userdata is cleaner and lets `regex.__index` be set in Rust | Check `grep -rn 'regex\.compile' data` call sites + `detectindent.lua:104` (which is a *syntax* pattern table, unrelated) before deciding. Default: keep the table, match the C exactly. |
| D-2 | `dirmonitor` via the `notify` crate vs a direct `inotify` backend | open | **Use inotify directly** (`inotify` crate or `libc`). `notify` is recursive/filter-oriented and cannot express the `mode()` `"single"`/`"multiple"` + watch-id contract that `dirwatch.lua:58-166` branches on. This is the one place where D3's "crate where equivalent" does not apply. |
| D-3 | `regex.ENDANCHORED` currently equals `ANCHORED` (bug at `regex.c:373-384`) | open | Keep bug-compatible in the port, add a `shortcut:` comment, and note it. Changing it is a behaviour change outside the port's charter. |
| D-4 | `pcre2` crate vs raw FFI in `cffi.rs` | open | Prefer raw FFI if the crate tries to vendor/build its own PCRE2; the C build already links `libpcre2-8`. |
| D-5 | `flake.nix`: add `cargo`/`rustc`, vendor the cargo registry for the sandboxed `packages.lite-xl` build | **done (Linux)** — `packages.lite-xl` is now `rustPlatform.buildRustPackage` with `cargoLock.lockFile = rust/Cargo.lock`, so crates are fetched by hash and the sandboxed build needs no network. Built and run from the Nix store in a clean env (`env -i`). Linux only (`platforms.linux`); macOS has no Nix package until it leaves Meson. | Required for W8. `mlua 0.12.2` is cached locally; `sdl3` is not. Either vendor or accept that the Nix package needs network. |
| D-6 | `panic = "abort"` vs catch-and-`lua_error` at every entrypoint | open | Abort for v1. Revisit if a panic becomes a supportable error path. |
| D-7 | Keep `custom_events.c` in C for W2 and move it in W5, or move it in W2 | open | Keep in C until W5; it is shared with dialogs and moving it early adds coupling. |
| D-8 | Does anything need `arena_allocator`? | open | Only `system.c` dialogs and `process.c` env. Replace with mlua-owned `String`s; delete the file in W5. |

## 10. Risk register

| risk | severity | mitigation |
|---|---|---|
| Meson + cargo build ordering is fiddly | low | §3.3 recipe + the bounded fallback wrapper; day-0 checkpoint |
| Rust panic unwinds through C `lua_pcall` (UB) | high | `panic = "abort"`, `catch_unwind` at hand-written entrypoints, mlua's own panic handling |
| Native plugin ABI (D2) cannot be satisfied | medium | W9 is isolated; if it fails, ship a runtime deprecation for `load_native_plugin` rather than blocking the port |
| Font rendering paranoia (subpixel/hinting/filter weights) | high | W7 last, `freetype-rs` over pure Rust, visual comparison |
| No test suite exists | high | Build the harness *first* (§7), baseline the C build before deleting C files |
| `unidata.h` transcription drift | medium | Generate, don't transcribe; checksum the tables |
| Multi-window assumptions baked into statics (`cells`, `persistant_window`, `window_hit_info`) | low | Preserve as-is; note with `shortcut:` comments |
| Rust module needs C statics/structs that change layout | medium | §5 accessor rule; never re-declare a C struct in Rust |
| Threaded code (dirmonitor, process kill list) sharing a `!Send` `Lua` | medium | Workers never touch Lua; communicate via SDL custom events / mutexes |
| Long gaps between sessions lose context | medium | This document + the progress log + `verify.sh` as the single entry point |

---

## 11. Reference: the full Lua-visible contract

### 11.A `system` — function list and return shapes

Full details in `src/api/system.c`. `(err)` means the second return value is a message string.

| function | args | returns |
|---|---|---|
| `poll_event` | – | event tuple (see §11.B), or 0 values |
| `wait_event` | `[timeout_seconds]` | boolean |
| `set_cursor` | `[opt="arrow"]` (`arrow,ibeam,sizeh,sizev,hand`) | – |
| `set_window_title` | `win, title` | – |
| `set_window_mode` | `win, [opt="normal"]` (`normal,minimized,maximized,fullscreen`) | – |
| `get_window_mode` | `win` | string |
| `set_window_bordered` | `win, bool` | – |
| `set_window_hit_test` | `win` or `win,title_h,controls_w,resize_border` | – |
| `get_window_size` | `win` | `w,h,x,y` |
| `set_window_size` | `win,w,h,x,y` | – |
| `window_has_focus` | `win` | boolean |
| `set_text_input_rect` | `win,x,y,w,h` | – |
| `clear_ime` | `win` | – |
| `raise_window` | `win` | – |
| `set_window_opacity` | `win, n` | boolean |
| `text_input` | `win, bool` | – |
| `show_fatal_error` | `title, msg` | – |
| `rmdir` | `path` | `true`, or `false, msg` |
| `mkdir` | `path` | `true`, or `false, msg` |
| `chdir` | `path` | – (**raises** on failure) |
| `list_dir` | `path` | table, or `nil, msg` |
| `absolute_path` | `path` | string, or **0 values** |
| `get_file_info` | `path` | `{size, modified, type, symlink?}`, or `nil, msg` |
| `get_fs_type` | `path` | string (`"unknown"` off Linux) |
| `ftruncate` | `file*, [len=0]` | `true`, or `false, msg` |
| `get_clipboard` | – | string, or **0 values** |
| `set_clipboard` | `text` | – |
| `get_primary_selection` | – | string, or **0 values** |
| `set_primary_selection` | `text` | – |
| `get_process_id` | – | integer |
| `get_time` | – | monotonic seconds (float) |
| `sleep` | `seconds` | – |
| `exec` | `cmd` | – (background) |
| `fuzzy_match` | `str, ptn, [files=false]` | integer score (0 = no match) |
| `path_compare` | `p1, t1, p2, t2` | boolean |
| `setenv` | `key, val` | boolean |
| `load_native_plugin` | `name, path` | entrypoint result count (**raises** on failure) |
| `open_file_dialog` / `open_directory_dialog` / `save_file_dialog` | `win, id, [opts]` | – (async → `dialogfinished`) |
| `get_sandbox` | – | `"none"|"unknown"|"flatpak"|"snap"|"macos"` |

### 11.B `poll_event` tuples (name, then args in this order)

`quit`; `resized(w,h)` (also calls `ren_resize_window`); `exposed` (invalidates rencache); `minimized`; `maximized`; `restored`; `mouseleft`; `focuslost`; `enteringforeground`; `enteredforeground`; `enteringbackground`; `enteredbackground`; `filedropped(path,x,y)`; `keypressed(name)`; `keyreleased(name)`; `textinput(text)`; `textediting(text,start,len)`; `mousepressed(button,x,y,clicks)`; `mousereleased(button,x,y)`; `mousemoved(x,y,dx,dy)`; `mousewheel(y,-x)`; `touchpressed(x,y,id)`; `touchreleased(x,y,id)`; `touchmoved(x,y,dx,dy,id)`; `dialogfinished(id,status[,list])`.

Mouse/key coordinates are scaled to **pixels**. `SDL_EVENT_WINDOW_FOCUS_GAINED` flushes queued `KEY_DOWN` and emits nothing. Events with `type >= SDL_EVENT_USER` dispatch to the registered custom-event callback; a callback returning `0` continues to the next event. Consumers: `data/core/init.lua:930` (`core.step`), `:896` (`dialogfinished`).

### 11.C `renderer` / `renwindow` / fonts

`renderer.show_debug(bool)`, `renderer.get_size()` → `w,h` (**points**), `renderer.begin_frame(win)` (asserts no target window), `renderer.end_frame()`, `renderer.set_clip_rect(x,y,w,h)`, `renderer.draw_rect(x,y,w,h,color?)`, `renderer.draw_text(font|group, text, x, y, color, tab?)` → new pen x.

`renderer.font`: `load(path,size,opts?) → Font` (errors `"failed to load font: %s"`), `Font:copy(size?,opts?)`, `Font.group(table) → FontGroup` (a **table**), `Font:set_tab_size(n)`, `Font:get_tab_size()`, `Font:get_width(text,tab?)`, `Font:get_height()`, `Font:get_size()`, `Font:set_size(size)`, `Font:get_path()` (group → table of strings), `__gc`. Options: `antialiasing` (`none|grayscale|subpixel`), `hinting` (`none|slight|full`), and booleans `italic,bold,underline,smoothing,strikethrough`. Maximum group size 10.

`renwindow.create(title[,w,h])`, `:get_size()` (points), `:_persist()`, `:_restore()`, `__gc`.

Tab tables: `{tab_offset = n}` or a plain number for simple tabs.

### 11.D `process` / `regex` / `dirmonitor`

See W1–W3 in §8; they are complete there.

### 11.E Globals, defines, environment

Set by C: `ARGS`, `PLATFORM`, `ARCH`, `RESTARTED`, `EXEFILE`, `MACOS_RESOURCES` (macOS).
Set by `data/core/start.lua`: `VERSION` (substituted), `MOD_VERSION_*`, `SCALE`, `PATHSEP`, `EXEDIR`, `DATADIR`, `USERDIR`, `package.path`/`cpath`, `package.native_plugins`, `package.searchers[3]`.
Defines: `LITE_PROJECT_VERSION_STR`, `LITE_ARCH_TUPLE`, `LITE_USE_SDL_RENDERER`, `PCRE2_STATIC`, `MACOS_USE_BUNDLE`.
Env vars: `LITE_PREFIX`, `LITE_USERDIR`, `LITE_XL_RUNTIME`, `LITE_SCALE`, `LITE_OS_HOME`, `XDG_CONFIG_HOME`, `GDK_SCALE`, `QT_SCALE_FACTOR`.

### 11.F Build environment (this checkout)

- Meson 1.10.2, Ninja 1.13.2, gcc 15.3.0, pkg-config 0.29.2, all from the Nix dev shell (`nix develop` / direnv). `IN_NIX_SHELL=impure`.
- System libs available: SDL3 3.4.14, freetype 2.6.20 `.pc` version 26.6.20, libpcre2-8 10.47. Lua has **no** `.pc` file — that is why `src/meson.build:22-38` has the `use_system_lua` special case.
- The existing `build/` tree uses the `subprojects/` wraps (`lua-5.4.6`, `freetype-2.14.1`, `pcre2-10.44`). `build-dev/` is stale; ignore it.
- `rustc`/`cargo` 1.97.1 are on PATH outside the dev shell too.
- There is **no test suite**; `.github/workflows/build.yml` only builds and packages.

---

## 12. Progress log (append-only)

| date | wave | state | notes / next action |
|---|---|---|---|
| — | planning | done | This document. Decisions D1–D4 from the user; D-1..D-8 open in §9. |
| 2026-10-16 | W0 | **done** | Added `rust/` (staticlib, `mlua 0.12` `lua54`+`module`, feature `probe`), Meson wiring in `src/meson.build`, option `core_features`, `tests/contract/` + `scripts/rust-port/verify.sh`, smoke baseline, `cargo`/`rustc` in the flake devShell. Probe verified end to end (`rust_probe.ping()` → `pong`) then removed from `api.c`; `api.c` is unchanged. **Next: W1 (`regex`).** Findings below. |
| 2026-10-16 | W1 | **done** | `rust/src/pcre.rs` + `rust/src/regex.rs`; `src/api/regex.c` removed. `verify.sh` green: `regex` (54 obs) and `tokenizer` (11 obs, the real syntax-highlighting path) match the C baseline exactly. Found a pre-existing segfault in C `regex.cmatch` with string patterns. **Next: W5 (`system`, + `custom_events`).** |
| 2026-10-16 | W2 | **done** | `rust/src/dirmonitor.rs` (inotify, Linux) replaces `dirmonitor.c` + `inotify.c`; `custom_events.c` stays C. `verify.sh` green on 7 suites incl. `dirmonitor` (43 obs), debug and release; real `dirwatch` loop, shutdown and leak checks pass. Fixes two C bugs (event-walk stride, shutdown race). **Next: W5.** |
| 2026-10-16 | W3 | **done** | `rust/src/process.rs` (raw libc fork/exec) + `libc` dep; `src/api/process.c` removed. `verify.sh` green on 6 suites incl. `process` (86 obs), debug and release; kill-at-`lua_close`, detach and fd-leak checks pass. **Next: W2.** |
| 2026-10-16 | W4 | **done** | `rust/src/utf8.rs` + generated `unidata.rs`; `src/api/utf8.c` removed. `verify.sh` green on 5 suites incl. `utf8` (780 obs) and `utf8_edge` (717 obs), debug and release. Worker subagent timed out; ported by hand. ≈1.2–1.7× slower than C on microbenchmarks (plain `find` 5×). **Next: W2 (`dirmonitor` + `custom_events`) or W3 (`process`).** |

**W0 findings (these supersede the snippets in §3.3 / §7.4 where they differ):**
- Meson reserves the `rust_` option prefix → option is `core_features` (`meson configure build -Dcore_features=probe`).
- Meson links a `custom_target` `.a` by itself when it is passed as `declare_dependency(sources:)`; do **not** also add its path to `link_args` (duplicate, wrong relative path). Only `-lpthread -ldl -lm` are added.
- Use `cargo build --manifest-path`, not `cd rust && …`, otherwise the relative `@OUTPUT@` in `cp` is wrong.
- `build_always_stale: true` means cargo runs every `ninja`; it is a no-op when nothing changed (~0.02s).
- `-Wl,--whole-archive` was **not** needed: each `luaopen_*` is referenced from `api.c`, so the linker pulls its object in. A module with no reference in `api.c` (like the probe) needs a one-line `api.c` registration to be reachable.
- mlua resolved to `0.12.2` / `mlua-sys 0.13`; it built offline from the local registry cache.
- Headless runs: `SDL_VIDEODRIVER=offscreen` works for the contract runtime. For the real editor use `LITE_USERDIR=<empty dir>` — the maintainer's `~/.config/lite-xl` has plugins (`lintplus`, `lsp`) that crash headless and are unrelated to the port.
- Harness is installed beside the data dir (`contract.lua`, `contract_cases/`), selected by `LITE_XL_RUNTIME=contract` and `CONTRACT_SUITES=<name>`. `verify.sh` fails (exit 1) on baseline mismatch — checked with a deliberately wrong baseline.
- **Known breakage:** `nix build .#lite-xl` (the sandboxed `packages` derivation) will now fail because `cargo` is not in its `nativeBuildInputs` and the sandbox has no network. Tracked as D-5; only `nix develop` + `ninja` works until then.

When a wave lands, record: files added/deleted, `verify.sh` result, anything that surprised you, and the next wave to pick up.
| 2026-10-16 | W5 | **done** | `rust/src/{system,sysevents,syswindow,sysplugin,custom_events,hit,plugin_syms}.rs` replace `api/system.c` and `custom_events.c`; the `lxl_rust_system_overlay` hook in `api.c` is gone (`api.c` is byte-identical to upstream again). 12 contract suites + `hit-test-check.sh` (1.24M grid points) green in debug and release; shipped binaries contain no `lxl_test` hooks; the real editor (`core.init`/`core.run`, offscreen) boots and exits 0. Behaviour changes: `lua_closethread` now resolves for native plugins (header is the source of truth, see `gen-plugin-syms.lua`); mouse/touch events for an unknown window are dropped instead of crashing; `ftruncate` on a closed file raises. **Not verified:** a real file dialog (never automated, see below), minimise/maximise in a real window (offscreen SDL never reports them), macOS/Windows. **Safety:** `verify.sh` runs every suite with a dead D-Bus address so no real dialog can open. **Next: W6.** |
| 2026-10-16 | W6 | **done** | `rust/src/{renwindow,rencache}.rs` replace `renwindow.c` + `rencache.c`, and with them the window-management block of `renderer.c` (`ren_create`/`ren_destroy`/the window list/`ren_get_size`/`ren_update_rects`), because `struct RenWindow` is now **Rust-owned and opaque to C** — its layout changes with `LITE_USE_SDL_RENDERER`, so a C mirror would be a silent corruption risk. `renderer.c` keeps FreeType/pixels and is otherwise ~90 lines lighter; `renderer.h` still declares the `ren_*` API. Both modes are ported: `LITE_USE_SDL_RENDERER` is the `sdl_renderer` cargo feature, which meson adds when `-Drenderer=true` or on macOS, and **the whole 13-suite set passes in that mode too** (offscreen SDL drives a real `SDL_CreateRenderer` + texture). Verification: `rencache` suite (30 observations) exposes `lxl_test.dirty_rects` + `lxl_test.pixel` and was recorded from C — grid priming, cell↔rect merging, clipping, text hashing, resize invalidation, and read-back pixels (format-independent, so one baseline covers both modes). Mutation-checked 10 ways: 7 caught, 1 (resize invalidation) caught only after adding a real resize case, and 2 (`draw_rect`'s early-exit, `last_clip_rect` bookkeeping) are **equivalent mutants** — `end_frame` filters those commands by clipping anyway, so the C guards are a fast path, not behaviour. One real bug found and fixed by the pixel cases: the text was copied 8 bytes past its slot and thus outside the hashed region, so changing only text content did not dirty a cell. **Not verified:** real-window rendering by eye, retina/HiDPI scale (`query_surface_scale` asserts an integer ratio and was only exercised at scale 1), macOS/Windows. **Next: W7.** |
| 2026-10-16 | W7 | **implemented; manual visual check pending** | `rust/src/{renderer,renderer_api,ft}.rs` replace `renderer.c` + `api/renderer.c`; `freetype-rs` owns font/library lifetimes. User chose to retain Meson's FreeType 2.14.1 after bundled 2.13.2 changed hinted pixels; W8 must resolve build ownership without silent pixel drift. All 14 suites pass in debug/release/SDL-renderer mode, including all 200 C renderer observations (unchanged pixel hashes); nine mutants caught, GC anchoring checked, real offscreen editor startup/shutdown and font-load FD checks pass. Review fixes: drawing-loop aliasing and complete FT diagnostics. Not verified: real-window appearance, HiDPI > 1, macOS/Windows. **Next: manual visual acceptance, then W8.** |
| 2026-10-16 | W9 | **done** | Resolver already in place from W5; W9 adds the missing proof: `pluginapi` suite (81 observations recorded from pristine upstream) exercising nearly all of the 155-symbol API through a real plugin, 16/16 address-swap mutants caught. Found and fixed an ABI difference: vendored Lua had `LUA_COMPAT_5_3` and API checks that released Lite XL does not (`.cargo/config.toml`). 15 suites pass in debug, release and `sdl_renderer`; Meson `module` build passes it too. |
| 2026-10-16 | W8 | **done (Linux)** | See the W8 section: `cargo build` produces `lite-xl` with Lua/PCRE2/FreeType built by Cargo and only SDL3 linked dynamically. 14 contract suites + `startup-check.sh` (8 checks) pass in debug, release and `sdl_renderer`. Renderer baseline re-recorded for FreeType 2.13.2 (six hinted-mono hashes). Meson kept for other platforms and still builds. **Not verified:** real window by eye, macOS/Windows. |
| 2026-10-17 | W10 | **done (Linux only)** | Meson, every C source under `src/`, macOS/Windows/wasm packaging and resources, and `.github/` CI deleted at the user’s request; Cargo features collapsed to `test_hooks` + `sdl_renderer`. See the W10 section. |
