//! Native plugin loading and the file dialogs, ported from `src/api/system.c`.
//!
//! * `load_native_plugin(name, path)` and the `NativePlugin` metatable's `__gc`.
//! * `api_require`, the `XL` argument of `luaopen_lite_xl_<name>(L, XL)`: a resolver that
//!   maps a Lua C-API symbol name to the address of that function in this executable. The
//!   table of names is generated from the plugin header (`plugin_syms.rs`).
//! * `open_file_dialog`, `save_file_dialog`, `open_directory_dialog` and the
//!   `dialogfinished` custom-event callback they feed.
//!
//! All raw `lua_CFunction`s: the entry points are `extern "C"` code that may `longjmp`
//! through Lua errors, so no Rust destructor is held across a call that can raise.

use std::ffi::{c_char, c_int, c_void, CStr, CString};

use mlua::ffi::{
    lua_State, lua_createtable, lua_getfield, lua_getglobal, lua_gettop, lua_pushinteger,
    lua_pushlightuserdata, lua_pushlstring, lua_pushstring, lua_rawseti, lua_setfield, lua_settop,
    lua_toboolean, lua_touserdata, lua_type, luaL_checkinteger, luaL_checklstring, luaL_checkstring,
    luaL_error, luaL_len, luaL_optlstring, luaL_setmetatable, LUA_TNIL, LUA_TTABLE,
};
use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};

use crate::platform::{Custom, Payload};
use crate::plugin_syms::SYMBOLS;

// ----------------------------------------------------------- api_require

/// `api_require`: the resolver handed to a plugin's `luaopen_lite_xl_<name>`.
/// Returns the address of the named function, or NULL if unknown.
pub unsafe extern "C" fn api_require(symbol: *const c_char) -> *mut c_void {
    if symbol.is_null() {
        return std::ptr::null_mut();
    }
    let wanted = CStr::from_ptr(symbol).to_bytes();
    for (name, address) in SYMBOLS.iter() {
        if name.as_bytes() == wanted {
            return *address as *mut c_void;
        }
    }
    std::ptr::null_mut()
}

// ------------------------------------------------------ load_native_plugin

type XlEntry = unsafe extern "C-unwind" fn(*mut lua_State, unsafe extern "C" fn(*const c_char) -> *mut c_void) -> c_int;
type PlainEntry = unsafe extern "C-unwind" fn(*mut lua_State) -> c_int;

/// `f_library_gc`: the `NativePlugin` metatable's `__gc`. The table's `handle` owns one
/// reference to the library; a missing handle is ignored, as `SDL_UnloadObject(NULL)` was.
pub unsafe extern "C-unwind" fn library_gc(l: *mut lua_State) -> c_int {
    lua_getfield(l, 1, c"handle".as_ptr());
    let handle = lua_touserdata(l, -1);
    if !handle.is_null() {
        drop(Library::from_raw(handle));
    }
    0
}

/// `dlerror`'s exact bytes, which libloading keeps private but debug-prints with
/// `escape_ascii`; undone here so a non-UTF-8 path survives in the message as it did through SDL.
fn dlerror_bytes(err: &libloading::Error) -> Vec<u8> {
    let unescape = |quoted: &str| -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut chars = quoted.strip_prefix('"')?.strip_suffix('"')?.bytes();
        while let Some(b) = chars.next() {
            if b != b'\\' {
                out.push(b);
                continue;
            }
            out.push(match chars.next()? {
                b't' => b'\t',
                b'r' => b'\r',
                b'n' => b'\n',
                b'x' => {
                    let hex = [chars.next()?, chars.next()?];
                    u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?
                }
                other => other,
            });
        }
        Some(out)
    };
    match err {
        libloading::Error::DlOpen { desc } => {
            unescape(&format!("{desc:?}")).unwrap_or_else(|| err.to_string().into_bytes())
        }
        _ => err.to_string().into_bytes(),
    }
}

/// `SDL_LoadFunction`: `dlsym`, then again with a leading underscore; a NULL address counts as
/// missing.
unsafe fn load_function(handle: *mut c_void, name: &[u8]) -> *mut c_void {
    // Borrowed, not owned: the plugin's table keeps the reference.
    let library = std::mem::ManuallyDrop::new(Library::from_raw(handle));
    for candidate in [name.to_vec(), [b"_", name].concat()] {
        if let Ok(symbol) = library.get::<*mut c_void>(&candidate) {
            if !symbol.is_null() {
                return *symbol;
            }
        }
    }
    std::ptr::null_mut()
}

/// `f_load_native_plugin`.
pub unsafe extern "C-unwind" fn load_native_plugin(l: *mut lua_State) -> c_int {
    use std::os::unix::ffi::OsStrExt;
    let name = luaL_checkstring(l, 1);
    let path = luaL_checkstring(l, 2);
    // SDL_LoadObject's flags: every symbol bound now, none made visible to later libraries.
    let opened = Library::open(Some(std::ffi::OsStr::from_bytes(CStr::from_ptr(path).to_bytes())), RTLD_NOW | RTLD_LOCAL);
    let library = match opened {
        Ok(library) => library.into_raw(),
        Err(err) => {
            // SDL_LoadObject's wording. Nothing owned may outlive this block: lua_error longjmps.
            {
                let message = [b"Failed loading ", CStr::from_ptr(path).to_bytes(), b": ", &dlerror_bytes(&err)].concat();
                lua_pushlstring(l, message.as_ptr() as *const c_char, message.len());
            }
            drop(err);
            mlua::ffi::lua_error(l)
        }
    };

    // package.native_plugins[name] = setmetatable({ handle = library }, NativePlugin)
    lua_getglobal(l, c"package".as_ptr());
    lua_getfield(l, -1, c"native_plugins".as_ptr());
    lua_createtable(l, 0, 1);
    lua_pushlightuserdata(l, library as *mut c_void);
    lua_setfield(l, -2, c"handle".as_ptr());
    luaL_setmetatable(l, c"NativePlugin".as_ptr());
    lua_setfield(l, -2, name);
    lua_settop(l, lua_gettop(l) - 2);

    // The entrypoint is named after the part of `name` after the last dot.
    let full = CStr::from_ptr(name).to_bytes();
    let base = match full.iter().rposition(|&b| b == b'.') {
        Some(dot) => &full[dot + 1..],
        None => full,
    };
    // The C formats into a 512-byte buffer and truncates; a name that long does not exist.
    let xl_entry = load_function(library, &[b"luaopen_lite_xl_", base].concat());

    let result = if !xl_entry.is_null() {
        let entry: XlEntry = std::mem::transmute(xl_entry);
        entry(l, api_require)
    } else {
        let plain_name = CString::new([b"luaopen_", base].concat()).unwrap();
        let plain_entry = load_function(library, plain_name.as_bytes());
        if plain_entry.is_null() {
            return luaL_error(
                l,
                c"Unable to load %s: Can't find %s(lua_State *L, void *XL)".as_ptr(),
                name,
                plain_name.as_ptr(),
            );
        }
        drop(plain_name);
        let entry: PlainEntry = std::mem::transmute(plain_entry);
        entry(l)
    };

    if result == 0 {
        return luaL_error(l, c"Unable to load %s: entrypoint must return a value".as_ptr(), name);
    }
    result
}

// ------------------------------------------------------------- the dialogs

const DIALOG_OK: i32 = 0;
const DIALOG_CANCEL: i32 = 1;
const DIALOG_ERROR: i32 = 2;

/// `dialogfinished_callback`: runs on the main thread inside `poll_event` and pushes
/// `("dialogfinished", id, "accept"|"cancel"|"error", list_or_message?)`.
pub unsafe fn dialogfinished_callback(l: *mut lua_State, e: &Custom) -> c_int {
    lua_pushstring(l, c"dialogfinished".as_ptr());
    lua_pushinteger(l, e.data1 as i64);
    match e.code {
        DIALOG_OK => {
            lua_pushstring(l, c"accept".as_ptr());
            lua_createtable(l, 0, 0);
            // The C's "a\0b\0\0" block, walked the same way: an empty entry ends the list.
            let mut block = Vec::new();
            if let Some(Payload::Paths(paths)) = &e.payload {
                for path in paths {
                    block.extend_from_slice(path);
                    block.push(0);
                }
            }
            let entries = block.split(|&b| b == 0).take_while(|entry| !entry.is_empty());
            for (index, entry) in entries.enumerate() {
                lua_pushlstring(l, entry.as_ptr() as *const c_char, entry.len());
                lua_rawseti(l, -2, index as i64 + 1);
            }
            drop(block);
            4
        }
        DIALOG_CANCEL => {
            lua_pushstring(l, c"cancel".as_ptr());
            3
        }
        DIALOG_ERROR => {
            lua_pushstring(l, c"error".as_ptr());
            match &e.payload {
                // NULL in the C pushes nil.
                Some(Payload::Message(message)) => {
                    let message = crate::luaargs::nul_cut(message);
                    lua_pushlstring(l, message.as_ptr() as *const c_char, message.len());
                }
                _ => mlua::ffi::lua_pushnil(l),
            }
            4
        }
        _ => {
            lua_pushstring(l, c"unknown".as_ptr());
            3
        }
    }
}

// Snapshot validated Lua values before allocating Rust owners: option metamethods may raise.
unsafe fn checked_options(l: *mut lua_State, kind: crate::dialogs::Kind) -> c_int {
    let has_options = lua_type(l, 3) > LUA_TNIL;
    if has_options { mlua::ffi::luaL_checktype(l, 3, LUA_TTABLE); }
    lua_createtable(l, 6, 0);
    let snapshot = lua_gettop(l);
    if !has_options { return snapshot; }
    for (i, field) in [c"default_location", c"title", c"accept_label", c"cancel_label"].iter().enumerate() {
        lua_getfield(l, 3, field.as_ptr());
        luaL_optlstring(l, -1, std::ptr::null(), std::ptr::null_mut());
        lua_rawseti(l, snapshot, i as i64 + 1);
    }
    if kind != crate::dialogs::Kind::Save {
        lua_getfield(l, 3, c"allow_many".as_ptr());
        let many = lua_toboolean(l, -1);
        lua_settop(l, lua_gettop(l) - 1);
        mlua::ffi::lua_pushboolean(l, many);
        lua_rawseti(l, snapshot, 5);
    }
    if kind != crate::dialogs::Kind::Directory {
        lua_getfield(l, 3, c"filters".as_ptr());
        let original = lua_gettop(l);
        let count = if lua_type(l, original) > LUA_TNIL { luaL_len(l, original) } else { 0 };
        lua_createtable(l, 0, 0);
        let flat = lua_gettop(l);
        for i in 0..count {
            mlua::ffi::lua_geti(l, original, i + 1);
            for (j, field) in [c"name", c"pattern"].iter().enumerate() {
                lua_getfield(l, -1, field.as_ptr());
                luaL_checklstring(l, -1, std::ptr::null_mut());
                lua_rawseti(l, flat, i * 2 + j as i64 + 1);
            }
            lua_settop(l, lua_gettop(l) - 1);
        }
        lua_rawseti(l, snapshot, 6);
        lua_settop(l, snapshot);
    }
    snapshot
}

unsafe fn copied_string(l: *mut lua_State, index: c_int, field: i64) -> Option<Vec<u8>> {
    mlua::ffi::lua_rawgeti(l, index, field);
    let ptr = mlua::ffi::lua_tolstring(l, -1, std::ptr::null_mut());
    let bytes = if ptr.is_null() { None } else { Some(CStr::from_ptr(ptr).to_bytes().to_vec()) };
    lua_settop(l, lua_gettop(l) - 1);
    bytes
}

unsafe fn open_dialog(l: *mut lua_State, kind: crate::dialogs::Kind) -> c_int {
    let window = crate::renwindow_api::window_id_at(l, 1);
    let id = luaL_checkinteger(l, 2) as usize;
    let snapshot = checked_options(l, kind);
    let location = copied_string(l, snapshot, 1);
    let title = copied_string(l, snapshot, 2);
    let accept = copied_string(l, snapshot, 3);
    mlua::ffi::lua_rawgeti(l, snapshot, 5);
    let many = lua_toboolean(l, -1) != 0;
    lua_settop(l, snapshot);
    mlua::ffi::lua_rawgeti(l, snapshot, 6);
    let flat = lua_gettop(l);
    let mut filters = Vec::new();
    if lua_type(l, flat) == LUA_TTABLE {
        let count = mlua::ffi::lua_rawlen(l, flat) / 2;
        for i in 0..count {
            let name = copied_string(l, flat, i as i64 * 2 + 1).unwrap();
            let pattern = copied_string(l, flat, i as i64 * 2 + 2).unwrap();
            filters.push((name, pattern));
        }
    }
    crate::dialogs::start(window, id, kind,
        crate::dialogs::Options { location, title, accept, many, filters });
    0
}

pub unsafe extern "C-unwind" fn open_file_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, crate::dialogs::Kind::Open)
}
pub unsafe extern "C-unwind" fn save_file_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, crate::dialogs::Kind::Save)
}
pub unsafe extern "C-unwind" fn open_directory_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, crate::dialogs::Kind::Directory)
}
