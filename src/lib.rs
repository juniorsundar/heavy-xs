//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>`; `app::api_load_libs`
//! registers them, and `app::run` is the editor's `main` (`src/main.rs`).
//! Linux only: Cargo builds Lua and links the system SDL3; fonts use the Rust-native backend.

#[cfg(not(target_os = "linux"))]
compile_error!("Lite XL is Linux-only");

pub mod app;
mod clipboard;
mod custom_events;
mod dialogs;
mod fatal;
#[cfg(test)]
#[path = "../tests/contract/portal.rs"]
mod portal_fixture;
mod native;
mod native_input;
mod services;
mod font;
mod renderer;
mod renderer_api;
mod rencache;
mod renwindow;
mod renwindow_api;
mod dirmonitor;
mod hit;
mod clicks;
mod keymap;
mod keynames;
mod platform;
mod plugin_syms;
mod process;
mod pathmatch;
mod system;
mod syswindow;
mod sysevents;
mod sysplugin;
mod unidata;
mod utf8;

#[cfg(feature = "test_hooks")]
mod test_hooks;

mod regex;

use mlua::prelude::*;

/// Defines `luaopen_<name>` for a module written against mlua. This is what
/// `#[mlua::lua_module]` generates, but that macro only compiles with mlua's `module` feature
/// and this build links a vendored Lua instead.
macro_rules! lua_entry {
    ($open:ident, $module:path) => {
        pub(crate) unsafe extern "C-unwind" fn $open(state: *mut mlua::lua_State) -> std::os::raw::c_int {
            mlua::Lua::entrypoint1(state, |lua: &Lua| $module(lua))
        }
    };
}

lua_entry!(luaopen_regex, regex::luaopen_regex);
lua_entry!(luaopen_dirmonitor, dirmonitor::luaopen_dirmonitor);
lua_entry!(luaopen_process, process::luaopen_process);
lua_entry!(luaopen_utf8extra, utf8::luaopen_utf8extra);
lua_entry!(luaopen_system, system::luaopen_system);
