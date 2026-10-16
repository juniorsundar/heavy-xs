//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>` so the
//! existing `src/api/api.c` registration table keeps working unchanged.

mod pcre;
mod process;
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

#[mlua::lua_module(name = "process")]
fn entry_process(lua: &Lua) -> LuaResult<LuaTable> {
    process::luaopen_process(lua)
}

#[mlua::lua_module(name = "utf8extra")]
fn entry_utf8(lua: &Lua) -> LuaResult<LuaTable> {
    utf8::luaopen_utf8extra(lua)
}
