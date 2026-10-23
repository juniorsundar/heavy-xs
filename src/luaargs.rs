//! Conventions shared by the Lua-facing modules: how a bad argument is reported, how a string
//! argument is read, and what a C string makes of a Lua string. Number arguments are not here:
//! `system`, `process` and `utf8` each coerce them differently, and each is pinned to that.

use mlua::{Error as LuaError, Lua, LuaString, Result as LuaResult, Value};

/// `luaL_argerror`'s message for a wrong type.
pub(crate) fn argerror(fname: &str, arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!(
        "bad argument #{} to '{}' ({} expected, got {})",
        arg, fname, expected, got
    ))
}

/// `luaL_checklstring` for argument number `arg`: strings and numbers (coerced) only. `None`
/// is an absent argument.
pub(crate) fn string_arg(lua: &Lua, v: Option<&Value>, arg: usize, fname: &str) -> LuaResult<LuaString> {
    match v {
        Some(v) => lua
            .coerce_string(v.clone())?
            .ok_or_else(|| argerror(fname, arg, "string", v.type_name())),
        None => Err(argerror(fname, arg, "string", "no value")),
    }
}

/// `string_arg` for a value that is present: `arg` is its 1-based argument number.
pub(crate) fn coerce_string(lua: &Lua, v: &Value, arg: usize, fname: &str) -> LuaResult<LuaString> {
    string_arg(lua, Some(v), arg, fname)
}

/// `string_arg` for `args[i]`, reporting it as argument `i + 1`.
pub(crate) fn check_string(lua: &Lua, args: &[Value], i: usize, fname: &str) -> LuaResult<LuaString> {
    string_arg(lua, args.get(i), i + 1, fname)
}

/// The bytes a `const char *` would see: everything before the first NUL.
pub(crate) fn nul_cut(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_cut_stops_at_the_first_nul() {
        assert_eq!(nul_cut(b"abc"), b"abc");
        assert_eq!(nul_cut(b"a\0b\0c"), b"a");
        assert_eq!(nul_cut(b"\0abc"), b"");
        assert_eq!(nul_cut(b""), b"");
    }

    #[test]
    fn string_arguments_coerce_numbers_and_name_the_failure() {
        let lua = Lua::new();
        let text = string_arg(&lua, Some(&Value::Integer(42)), 1, "f").unwrap();
        assert_eq!(&*text.as_bytes(), b"42");

        let wrong = string_arg(&lua, Some(&Value::Boolean(true)), 2, "f").unwrap_err().to_string();
        assert!(wrong.contains("bad argument #2 to 'f' (string expected, got boolean)"), "{wrong}");

        let absent = check_string(&lua, &[], 0, "g").unwrap_err().to_string();
        assert!(absent.contains("bad argument #1 to 'g' (string expected, got no value)"), "{absent}");
    }
}
