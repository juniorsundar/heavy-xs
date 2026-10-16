//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>` so the
//! existing `src/api/api.c` registration table keeps working unchanged.

mod pcre;

#[cfg(feature = "probe")]
mod probe;

mod regex;

use mlua::prelude::*;

#[mlua::lua_module(name = "regex")]
fn entry_regex(lua: &Lua) -> LuaResult<LuaTable> {
    regex::luaopen_regex(lua)
}
