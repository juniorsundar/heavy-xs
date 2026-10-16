//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>` so the
//! existing `src/api/api.c` registration table keeps working unchanged.

mod custom_events;
mod ft;
mod renderer;
mod renderer_api;
mod rencache;
mod renwindow;
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

#[cfg(feature = "probe")]
mod probe;
#[cfg(feature = "test_hooks")]
mod test_hooks;

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

#[mlua::lua_module(name = "system")]
fn entry_system(lua: &Lua) -> LuaResult<LuaTable> {
    system::luaopen_system(lua)
}
