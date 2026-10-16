//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>` so the
//! existing `src/api/api.c` registration table keeps working unchanged.

#[cfg(target_os = "linux")]
mod dirmonitor;
mod pcre;
mod process;
mod system;
mod unidata;
mod utf8;

#[cfg(feature = "probe")]
mod probe;

mod regex;

use mlua::prelude::*;

#[mlua::lua_module(name = "regex")]
fn entry_regex(lua: &Lua) -> LuaResult<LuaTable> {
    regex::luaopen_regex(lua)
}

#[cfg(target_os = "linux")]
#[mlua::lua_module(name = "dirmonitor")]
fn entry_dirmonitor(lua: &Lua) -> LuaResult<LuaTable> {
    dirmonitor::luaopen_dirmonitor(lua)
}

#[mlua::lua_module(name = "process")]
fn entry_process(lua: &Lua) -> LuaResult<LuaTable> {
    process::luaopen_process(lua)
}

#[mlua::lua_module(name = "utf8extra")]
fn entry_utf8(lua: &Lua) -> LuaResult<LuaTable> {
    utf8::luaopen_utf8extra(lua)
}

/// Called by `api_load_libs` once the C `luaopen_system` has run and `system` is
/// a global: replaces the functions that have been ported (see `system.rs`).
///
/// # Safety
/// `state` must be the live Lua state `api_load_libs` is loading libraries into.
#[no_mangle]
pub unsafe extern "C" fn lxl_rust_system_overlay(state: *mut mlua::lua_State) {
    let lua = Lua::get_or_init_from_ptr(state);
    if let Err(error) = system::install(lua) {
        eprintln!("lite-xl: cannot install Rust system overlay: {error}");
    }
}
