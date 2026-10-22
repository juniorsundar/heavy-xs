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
    luaL_checkudata, luaL_error, luaL_len, luaL_optlstring, luaL_setmetatable, LUA_TNIL, LUA_TTABLE,
};
use sdl3_sys::everything as sdl;
use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};

use crate::custom_events::{push_custom_event, CustomEvent};
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

/// Everything the SDL callback needs once the dialog finishes (possibly on another thread).
struct DialogData {
    id: usize,
    filters: Vec<sdl::SDL_DialogFileFilter>,
}

impl Drop for DialogData {
    fn drop(&mut self) {
        for filter in &self.filters {
            unsafe {
                sdl::SDL_free(filter.name as *mut c_void);
                sdl::SDL_free(filter.pattern as *mut c_void);
            }
        }
    }
}

extern "C" {
    fn ren_get_sdl_window(window: *mut c_void) -> *mut sdl::SDL_Window;
}

/// `dialog_callback`: SDL calls this (on whichever thread finished the dialog) with the
/// chosen paths. It never touches Lua; the result becomes a `dialogfinished` event that the
/// main thread's `poll_event` turns into Lua values via `dialogfinished_callback`.
unsafe extern "C" fn dialog_callback(userdata: *mut c_void, filelist: *const *const c_char, _filter: c_int) {
    let data = Box::from_raw(userdata as *mut DialogData);
    let data1 = data.id;
    drop(data); // the filters had to outlive the dialog; they no longer do

    let (code, payload) = if filelist.is_null() {
        let message = CStr::from_ptr(sdl::SDL_GetError()).to_bytes().to_vec();
        (DIALOG_ERROR, Some(Payload::Message(message)))
    } else if (*filelist).is_null() {
        (DIALOG_CANCEL, None)
    } else {
        // Owned copies: SDL frees `filelist` when this callback returns.
        let mut paths = Vec::new();
        let mut i = 0;
        while !(*filelist.add(i)).is_null() {
            paths.push(CStr::from_ptr(*filelist.add(i)).to_bytes().to_vec());
            i += 1;
        }
        (DIALOG_OK, Some(Payload::Paths(paths)))
    };

    let _ = push_custom_event(b"dialogfinished", CustomEvent { code, data1, payload });
}

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
                    let len = message.iter().position(|&b| b == 0).unwrap_or(message.len());
                    lua_pushlstring(l, message.as_ptr() as *const c_char, len);
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

/// An owned copy of an optional string option: `NULL` when the field is nil.
unsafe fn optional_string(l: *mut lua_State, index: c_int, field: &CStr) -> Option<CString> {
    lua_getfield(l, index, field.as_ptr());
    let mut length = 0usize;
    let text = luaL_optlstring(l, -1, std::ptr::null(), &mut length);
    let owned = if text.is_null() {
        None
    } else {
        Some(CString::new(std::slice::from_raw_parts(text as *const u8, length)).unwrap_or_default())
    };
    owned
}

/// `get_dialog_filters`: `{ {name=, pattern=}, ... }` -> SDL filters, strings owned by SDL's heap.
unsafe fn dialog_filters(l: *mut lua_State, index: c_int) -> Vec<sdl::SDL_DialogFileFilter> {
    let index = if index < 0 { index + lua_gettop(l) + 1 } else { index };
    if lua_type(l, index) <= LUA_TNIL {
        return Vec::new();
    }
    let count = luaL_len(l, index);
    let mut filters = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        mlua::ffi::lua_geti(l, index, i + 1);
        let mut length = 0usize;
        lua_getfield(l, -1, c"name".as_ptr());
        let name = luaL_checklstring(l, -1, &mut length);
        let name = sdl::SDL_strndup(name, length);
        lua_getfield(l, -2, c"pattern".as_ptr());
        let pattern = luaL_checklstring(l, -1, &mut length);
        let pattern = sdl::SDL_strndup(pattern, length);
        lua_settop(l, lua_gettop(l) - 3);
        filters.push(sdl::SDL_DialogFileFilter { name, pattern });
    }
    filters
}

unsafe fn open_dialog(l: *mut lua_State, kind: sdl::SDL_FileDialogType) -> c_int {
    let renwindow = *(luaL_checkudata(l, 1, c"RenWindow".as_ptr()) as *mut *mut c_void);
    let id = luaL_checkinteger(l, 2) as usize;

    let (mut default_location, mut title, mut accept, mut cancel) = (None, None, None, None);
    let mut allow_many = false;
    let mut filters = Vec::new();

    if lua_type(l, 3) > LUA_TNIL {
        mlua::ffi::luaL_checktype(l, 3, LUA_TTABLE);
        default_location = optional_string(l, 3, c"default_location");
        title = optional_string(l, 3, c"title");
        accept = optional_string(l, 3, c"accept_label");
        cancel = optional_string(l, 3, c"cancel_label");
        lua_settop(l, lua_gettop(l) - 4);

        if kind == sdl::SDL_FileDialogType::OPENFILE || kind == sdl::SDL_FileDialogType::OPENFOLDER {
            lua_getfield(l, 3, c"allow_many".as_ptr());
            allow_many = lua_type(l, -1) > LUA_TNIL && lua_toboolean(l, -1) != 0;
            lua_settop(l, lua_gettop(l) - 1);
        }
        if kind == sdl::SDL_FileDialogType::OPENFILE || kind == sdl::SDL_FileDialogType::SAVEFILE {
            lua_getfield(l, 3, c"filters".as_ptr());
            filters = dialog_filters(l, -1);
            lua_settop(l, lua_gettop(l) - 1);
        }
    }

    let props = sdl::SDL_CreateProperties();
    if props.0 == 0 {
        return luaL_error(l, c"Error while creating SDL Property: %s".as_ptr(), sdl::SDL_GetError());
    }

    // SDL reads the filter array until the callback runs, so it lives in the Box that
    // `dialog_callback` takes back and drops.
    let data = Box::new(DialogData { id, filters });
    let filter_ptr = data.filters.as_ptr();
    let filter_count = data.filters.len();
    let data = Box::into_raw(data);

    let opt = |value: &Option<CString>| value.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    sdl::SDL_SetPointerProperty(props, sdl::SDL_PROP_FILE_DIALOG_FILTERS_POINTER, filter_ptr as *mut c_void);
    sdl::SDL_SetNumberProperty(props, sdl::SDL_PROP_FILE_DIALOG_NFILTERS_NUMBER, filter_count as i64);
    sdl::SDL_SetPointerProperty(props, sdl::SDL_PROP_FILE_DIALOG_WINDOW_POINTER, ren_get_sdl_window(renwindow) as *mut c_void);
    sdl::SDL_SetStringProperty(props, sdl::SDL_PROP_FILE_DIALOG_LOCATION_STRING, opt(&default_location));
    sdl::SDL_SetBooleanProperty(props, sdl::SDL_PROP_FILE_DIALOG_MANY_BOOLEAN, allow_many);
    sdl::SDL_SetStringProperty(props, sdl::SDL_PROP_FILE_DIALOG_TITLE_STRING, opt(&title));
    sdl::SDL_SetStringProperty(props, sdl::SDL_PROP_FILE_DIALOG_ACCEPT_STRING, opt(&accept));
    sdl::SDL_SetStringProperty(props, sdl::SDL_PROP_FILE_DIALOG_CANCEL_STRING, opt(&cancel));

    sdl::SDL_ShowFileDialogWithProperties(kind, Some(dialog_callback), data as *mut c_void, props);
    sdl::SDL_DestroyProperties(props);
    0
}

pub unsafe extern "C-unwind" fn open_file_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, sdl::SDL_FileDialogType::OPENFILE)
}
pub unsafe extern "C-unwind" fn save_file_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, sdl::SDL_FileDialogType::SAVEFILE)
}
pub unsafe extern "C-unwind" fn open_directory_dialog(l: *mut lua_State) -> c_int {
    open_dialog(l, sdl::SDL_FileDialogType::OPENFOLDER)
}

