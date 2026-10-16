//! `src/api/renwindow.c`: the `renwindow` Lua module (`RenWindow` userdata).
//!
//! The userdata holds one `RenWindow *`. One window may be marked persistent so it survives
//! the Lua state being recreated on restart (`core.restart_request`): `__gc` skips destroying
//! it and `_restore` hands it to the new state.

use std::ffi::{c_int, CStr};
use std::ptr;

use mlua::ffi::{
    luaL_checkstring, luaL_checkudata, luaL_error, luaL_newmetatable, luaL_optnumber,
    luaL_setmetatable, lua_CFunction, lua_State, lua_newuserdata, lua_pushnil, lua_pushnumber,
    lua_pushvalue, lua_setfield,
};
use sdl3_sys::everything as sdl;

use crate::renderer::video_init;
use crate::renwindow::{ren_create, ren_destroy, ren_get_size, RenWindow};

const RENWINDOW_TYPE: &CStr = c"RenWindow";

/// Only touched from the Lua/main thread.
static mut PERSISTENT_WINDOW: *mut RenWindow = ptr::null_mut();

/// 64x64 RGBA, extracted from `resources/icons/icon.inl` by build.rs.
static ICON_RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.rgba"));

unsafe fn init_window_icon(window: *mut sdl::SDL_Window) {
    // The icon is skipped on Windows (resource file) and macOS (bundle), as in the C.
    if cfg!(any(windows, target_os = "macos")) {
        return;
    }
    let format =
        sdl::SDL_GetPixelFormatForMasks(32, 0x0000_00ff, 0x0000_ff00, 0x00ff_0000, 0xff00_0000);
    let surface = sdl::SDL_CreateSurfaceFrom(64, 64, format, ICON_RGBA.as_ptr() as *mut _, 64 * 4);
    sdl::SDL_SetWindowIcon(window, surface);
    sdl::SDL_DestroySurface(surface);
}

unsafe fn check_window(l: *mut lua_State) -> *mut RenWindow {
    *(luaL_checkudata(l, 1, RENWINDOW_TYPE.as_ptr()) as *mut *mut RenWindow)
}

unsafe fn push_window(l: *mut lua_State, window: *mut RenWindow) {
    let slot = lua_newuserdata(l, std::mem::size_of::<*mut RenWindow>()) as *mut *mut RenWindow;
    luaL_setmetatable(l, RENWINDOW_TYPE.as_ptr());
    *slot = window;
}

unsafe extern "C-unwind" fn f_renwin_create(l: *mut lua_State) -> c_int {
    let title = luaL_checkstring(l, 1);
    let mut width = luaL_optnumber(l, 2, 0.0) as f32;
    let mut height = luaL_optnumber(l, 3, 0.0) as f32;

    if video_init() != 0 {
        return luaL_error(
            l,
            c"Error creating lite-xl window: %s".as_ptr(),
            sdl::SDL_GetError(),
        );
    }

    if width < 1.0 || height < 1.0 {
        let mode = sdl::SDL_GetCurrentDisplayMode(sdl::SDL_GetPrimaryDisplay());
        // The C dereferences this unchecked; a missing display mode is an error here.
        if mode.is_null() {
            return luaL_error(
                l,
                c"Error creating lite-xl window: %s".as_ptr(),
                sdl::SDL_GetError(),
            );
        }
        if width < 1.0 {
            width = (*mode).w as f32 * 0.8;
        }
        if height < 1.0 {
            height = (*mode).h as f32 * 0.8;
        }
    }

    let window = sdl::SDL_CreateWindow(
        title,
        width as c_int,
        height as c_int,
        sdl::SDL_WINDOW_RESIZABLE | sdl::SDL_WINDOW_HIGH_PIXEL_DENSITY | sdl::SDL_WINDOW_HIDDEN,
    );
    if window.is_null() {
        return luaL_error(
            l,
            c"Error creating lite-xl window: %s".as_ptr(),
            sdl::SDL_GetError(),
        );
    }
    init_window_icon(window);
    push_window(l, ren_create(window));
    1
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
