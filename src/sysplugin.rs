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

use crate::custom_events::{push_custom_event, CustomEvent};
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

/// `f_library_gc`: the `NativePlugin` metatable's `__gc`.
pub unsafe extern "C-unwind" fn library_gc(l: *mut lua_State) -> c_int {
    lua_getfield(l, 1, c"handle".as_ptr());
    let handle = lua_touserdata(l, -1) as *mut sdl::SDL_SharedObject;
    sdl::SDL_UnloadObject(handle);
    0
}

/// `f_load_native_plugin`.
pub unsafe extern "C-unwind" fn load_native_plugin(l: *mut lua_State) -> c_int {
    let name = luaL_checkstring(l, 1);
    let path = luaL_checkstring(l, 2);
    let library = sdl::SDL_LoadObject(path);
    if library.is_null() {
        lua_pushstring(l, sdl::SDL_GetError());
        mlua::ffi::lua_error(l)
    }

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
    let xl_name = CString::new([b"luaopen_lite_xl_", base].concat()).unwrap();
    let xl_entry = sdl::SDL_LoadFunction(library, xl_name.as_ptr());

    let result = if let Some(entry) = xl_entry {
        let entry: XlEntry = std::mem::transmute(entry);
        entry(l, api_require)
    } else {
        let plain_name = CString::new([b"luaopen_", base].concat()).unwrap();
        match sdl::SDL_LoadFunction(library, plain_name.as_ptr()) {
            Some(entry) => {
                let entry: PlainEntry = std::mem::transmute(entry);
                entry(l)
            }
            None => {
                return luaL_error(
                    l,
                    c"Unable to load %s: Can't find %s(lua_State *L, void *XL)".as_ptr(),
                    name,
                    plain_name.as_ptr(),
                )
            }
        }
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
    let mut event = CustomEvent {
        window_id: 0,
        code: DIALOG_OK,
        data1: data.id as *mut c_void,
        data2: std::ptr::null_mut(),
    };
    drop(data); // the filters had to outlive the dialog; they no longer do

    if filelist.is_null() {
        event.code = DIALOG_ERROR;
        event.data2 = sdl::SDL_strdup(sdl::SDL_GetError()) as *mut c_void;
    } else if (*filelist).is_null() {
        event.code = DIALOG_CANCEL;
    } else {
        // "path1\0path2\0...pathN\0\0"
        let mut bytes = 0usize;
        let mut i = 0;
        while !(*filelist.add(i)).is_null() {
            bytes += CStr::from_ptr(*filelist.add(i)).to_bytes().len() + 1;
            i += 1;
        }
        let buffer = sdl::SDL_malloc(bytes + 1) as *mut u8;
        if buffer.is_null() {
            event.code = DIALOG_ERROR;
        } else {
            let mut at = buffer;
            let mut i = 0;
            while !(*filelist.add(i)).is_null() {
                let entry = CStr::from_ptr(*filelist.add(i)).to_bytes_with_nul();
                std::ptr::copy_nonoverlapping(entry.as_ptr(), at, entry.len());
                at = at.add(entry.len());
                i += 1;
            }
            *at = 0;
            event.data2 = buffer as *mut c_void;
        }
    }

    if !push_custom_event(c"dialogfinished".as_ptr(), &mut event) {
        sdl::SDL_free(event.data2);
    }
}

/// `dialogfinished_callback`: runs on the main thread inside `poll_event` and pushes
/// `("dialogfinished", id, "accept"|"cancel"|"error", list_or_message?)`.
pub unsafe extern "C" fn dialogfinished_callback(l: *mut lua_State, e: *mut sdl::SDL_Event) -> c_int {
    lua_pushstring(l, c"dialogfinished".as_ptr());
    lua_pushinteger(l, (*e).user.data1 as usize as i64);
    match (*e).user.code {
        DIALOG_OK => {
            lua_pushstring(l, c"accept".as_ptr());
            let mut at = (*e).user.data2 as *const c_char;
            lua_createtable(l, 0, 0);
            let mut index = 1;
            while !at.is_null() && *at != 0 {
                let entry = CStr::from_ptr(at);
                lua_pushlstring(l, at, entry.to_bytes().len());
                lua_rawseti(l, -2, index);
                at = at.add(entry.to_bytes().len() + 1);
                index += 1;
            }
            sdl::SDL_free((*e).user.data2);
            4
        }
        DIALOG_CANCEL => {
            lua_pushstring(l, c"cancel".as_ptr());
            3
        }
        DIALOG_ERROR => {
            lua_pushstring(l, c"error".as_ptr());
            lua_pushstring(l, (*e).user.data2 as *const c_char);
            sdl::SDL_free((*e).user.data2);
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

