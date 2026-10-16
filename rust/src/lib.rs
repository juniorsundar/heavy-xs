//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>`; `app::api_load_libs`
//! registers them, and `app::run` is the editor's `main` (`src/main.rs`).
//!
//! Two builds share this crate: `standalone` (default; Cargo builds Lua, PCRE2, FreeType and
//! links SDL3) and `module` (a static library the Meson build links, for platforms not yet
//! moved off Meson).

#[cfg(not(feature = "module"))]
pub mod app;
mod custom_events;
mod ft;
mod renderer;
mod renderer_api;
mod rencache;
mod renwindow;
#[cfg(not(feature = "module"))]
mod renwindow_api;
#[cfg(target_os = "linux")]
mod dirmonitor;
mod pcre;
mod hit;
mod plugin_syms;
mod process;
mod system;
mod syswindow;
mod sysevents;
mod sysplugin;
mod unidata;
mod utf8;

#[cfg(feature = "test_hooks")]
mod test_hooks;

mod regex;

// Nothing references pcre2-sys' items (rust/src/pcre.rs declares its own externs), so link it
// explicitly.
#[cfg(feature = "standalone")]
extern crate pcre2_sys;

use mlua::prelude::*;

/// Defines `luaopen_<name>` for a module written against mlua. This is what
/// `#[mlua::lua_module]` generates, but that macro only compiles with mlua's `module` feature
/// and the standalone build links a vendored Lua instead.
macro_rules! lua_entry {
    ($open:ident, $module:path) => {
        // Under `module`, C (`src/api/api.c`) calls these by name.
        #[cfg_attr(feature = "module", no_mangle)]
        pub(crate) unsafe extern "C-unwind" fn $open(state: *mut mlua::lua_State) -> std::os::raw::c_int {
            mlua::Lua::entrypoint1(state, |lua: &Lua| $module(lua))
        }
    };
}

lua_entry!(luaopen_regex, regex::luaopen_regex);
#[cfg(target_os = "linux")]
lua_entry!(luaopen_dirmonitor, dirmonitor::luaopen_dirmonitor);
lua_entry!(luaopen_process, process::luaopen_process);
lua_entry!(luaopen_utf8extra, utf8::luaopen_utf8extra);
lua_entry!(luaopen_system, system::luaopen_system);

// The probe is only reachable through `require`, so it keeps the attribute (module builds only).
#[cfg(all(feature = "probe", feature = "module"))]
mod probe;
