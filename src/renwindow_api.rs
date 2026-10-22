//! `src/api/renwindow.c`: the `renwindow` Lua module (`RenWindow` userdata).
//!
//! The userdata holds one `RenWindow *`. One window may be marked persistent so it survives
//! the Lua state being recreated on restart (`core.restart_request`): `__gc` skips destroying
//! it and `_restore` hands it to the new state.
//!
//! The native owner creates the winit window; this module only asks for it and wraps the
//! returned registry id in a `RenWindow`.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use mlua::ffi::{
    luaL_checkstring, luaL_checkudata, luaL_error, luaL_newmetatable, luaL_optnumber,
    luaL_setmetatable, lua_CFunction, lua_State, lua_newuserdata, lua_pushlstring, lua_pushnil,
    lua_pushnumber, lua_pushvalue, lua_setfield, lua_tostring,
};

use crate::native;
use crate::renwindow::{ren_create, ren_destroy, ren_get_size, RenWindow};

const RENWINDOW_TYPE: &CStr = c"RenWindow";

/// Only touched from the Lua/main thread.
static mut PERSISTENT_WINDOW: *mut RenWindow = ptr::null_mut();

unsafe fn check_window(l: *mut lua_State) -> *mut RenWindow {
    *(luaL_checkudata(l, 1, RENWINDOW_TYPE.as_ptr()) as *mut *mut RenWindow)
}

unsafe fn push_window(l: *mut lua_State, window: *mut RenWindow) {
    let slot = lua_newuserdata(l, std::mem::size_of::<*mut RenWindow>()) as *mut *mut RenWindow;
    luaL_setmetatable(l, RENWINDOW_TYPE.as_ptr());
    *slot = window;
}

unsafe extern "C-unwind" fn f_renwin_create(l: *mut lua_State) -> c_int {
    // luaL_checkstring is first-NUL: the native title takes the same bytes as SDL did.
    let title = luaL_checkstring(l, 1);
    let width = luaL_optnumber(l, 2, 0.0) as f32;
    let height = luaL_optnumber(l, 3, 0.0) as f32;
    let title = CStr::from_ptr(title).to_string_lossy();

    let created = native::create(&title, width, height);
    drop(title);
    match created {
        Ok(id) => {
            push_window(l, ren_create(id));
            1
        }
        Err(message) => {
            // luaL_error longjmps, which would skip `message`'s destructor: park the text on
            // the Lua stack first, then drop the Rust allocation before throwing.
            let (ptr, len) = (message.as_ptr() as *const c_char, message.len());
            lua_pushlstring(l, ptr, len);
            drop(message);
            luaL_error(
                l,
                c"Error creating lite-xl window: %s".as_ptr(),
                lua_tostring(l, -1),
            )
        }
    }
}

unsafe extern "C-unwind" fn f_renwin_gc(l: *mut lua_State) -> c_int {
    let window = check_window(l);
    if window != PERSISTENT_WINDOW {
        ren_destroy(window);
    }
    0
}

unsafe extern "C-unwind" fn f_renwin_get_size(l: *mut lua_State) -> c_int {
    let window = check_window(l);
    let (mut w, mut h): (c_int, c_int) = (0, 0);
    ren_get_size(window, &mut w, &mut h);
    lua_pushnumber(l, w as f64);
    lua_pushnumber(l, h as f64);
    2
}

unsafe extern "C-unwind" fn f_renwin_persist(l: *mut lua_State) -> c_int {
    PERSISTENT_WINDOW = check_window(l);
    0
}

unsafe extern "C-unwind" fn f_renwin_restore(l: *mut lua_State) -> c_int {
    if PERSISTENT_WINDOW.is_null() {
        lua_pushnil(l);
    } else {
        push_window(l, PERSISTENT_WINDOW);
    }
    1
}

/// `luaopen_renwindow`.
///
/// # Safety
/// `l` is a live Lua state.
pub unsafe extern "C-unwind" fn luaopen_renwindow(l: *mut lua_State) -> c_int {
    let lib: [(&CStr, lua_CFunction); 5] = [
        (c"create", f_renwin_create),
        (c"__gc", f_renwin_gc),
        (c"get_size", f_renwin_get_size),
        (c"_persist", f_renwin_persist),
        (c"_restore", f_renwin_restore),
    ];
    luaL_newmetatable(l, RENWINDOW_TYPE.as_ptr());
    for (name, func) in lib {
        mlua::ffi::lua_pushcfunction(l, func);
        lua_setfield(l, -2, name.as_ptr());
    }
    lua_pushvalue(l, -1);
    lua_setfield(l, -2, c"__index".as_ptr());
    1
}
