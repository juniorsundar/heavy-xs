use mlua::prelude::*;

#[mlua::lua_module(name = "rust_probe")]
fn rust_probe(lua: &Lua) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("ping", lua.create_function(|_, ()| Ok("pong"))?)?;
    Ok(t)
}
