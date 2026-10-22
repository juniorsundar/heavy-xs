//! Window, cursor, text-input and message-box functions from `src/api/system.c`.
//!
//! These take a `RenWindow` userdata, which in C is `RenWindow **` created by
//! `src/api/renwindow.c`. Window state now lives in the native winit owner: a `RenWindow`
//! carries only the registry id, and `crate::native` resolves it to the live window.
//!
//! Raw `lua_CFunction`s, like `sysevents.rs`: argument checking uses the real
//! `luaL_check*` so type errors read exactly as before ("bad argument #1 to ...").

use std::ffi::{c_char, c_int, CStr};

use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushstring, lua_toboolean,
    luaL_checknumber, luaL_checkoption, luaL_checkstring, luaL_checkudata,
};
use sdl3_sys::everything as sdl;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{CursorIcon, Fullscreen};

use crate::native;
use crate::renwindow::{ren_get_window_id, ren_resize_window, RenWindow};

/// `luaL_checkudata(L, 1, "RenWindow")`, dereferenced: the `RenWindow *`. The C does
/// not check it for NULL either, except `text_input`, which returns quietly.
unsafe fn check_window(l: *mut lua_State) -> *mut RenWindow {
    *(luaL_checkudata(l, 1, c"RenWindow".as_ptr()) as *mut *mut RenWindow)
}

unsafe fn window_id(l: *mut lua_State) -> u32 {
    ren_get_window_id(check_window(l))
}

// ----------------------------------------------------------------- cursor

pub unsafe extern "C-unwind" fn set_cursor(l: *mut lua_State) -> c_int {
    const NAMES: [*const c_char; 6] = [
        c"arrow".as_ptr(),
        c"ibeam".as_ptr(),
        c"sizeh".as_ptr(),
        c"sizev".as_ptr(),
        c"hand".as_ptr(),
        std::ptr::null(),
    ];
    const ICONS: [CursorIcon; 5] = [
        CursorIcon::Default,
        CursorIcon::Text,
        CursorIcon::EwResize,
        CursorIcon::NsResize,
        CursorIcon::Pointer,
    ];
    let option = luaL_checkoption(l, 1, c"arrow".as_ptr(), NAMES.as_ptr()) as usize;
    native::set_cursor(ICONS[option]);
    0
}

// ----------------------------------------------------------------- window

pub unsafe extern "C-unwind" fn set_window_title(l: *mut lua_State) -> c_int {
    let id = window_id(l);
    let title = CStr::from_ptr(luaL_checkstring(l, 2)).to_string_lossy();
    let window = native::window(id);
    if let Some(window) = window {
        window.set_title(&title);
    }
    0
}

pub unsafe extern "C-unwind" fn set_window_mode(l: *mut lua_State) -> c_int {
    const NAMES: [*const c_char; 5] = [
        c"normal".as_ptr(),
        c"minimized".as_ptr(),
        c"maximized".as_ptr(),
        c"fullscreen".as_ptr(),
        std::ptr::null(),
    ];
    const NORMAL: c_int = 0;
    const MINIMIZED: c_int = 1;
    const MAXIMIZED: c_int = 2;
    const FULLSCREEN: c_int = 3;

    let id = window_id(l);
    let mode = luaL_checkoption(l, 2, c"normal".as_ptr(), NAMES.as_ptr());
    let window = native::window(id);
    let Some(window) = window else { return 0 };
    window.set_fullscreen(if mode == FULLSCREEN {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    });
    if mode == NORMAL {
        window.set_minimized(false);
        window.set_maximized(false);
    }
    if mode == MAXIMIZED {
        window.set_maximized(true);
    }
    if mode == MINIMIZED {
        window.set_minimized(true);
    }
    0
}

pub unsafe extern "C-unwind" fn get_window_mode(l: *mut lua_State) -> c_int {
    let Some(window) = native::window(window_id(l)) else {
        lua_pushstring(l, c"normal".as_ptr());
        return 1;
    };
    let name = if window.fullscreen().is_some() {
        c"fullscreen"
    } else if window.is_minimized() == Some(true) {
        c"minimized"
    } else if window.is_maximized() {
        c"maximized"
    } else {
        c"normal"
    };
    drop(window);
    lua_pushstring(l, name.as_ptr());
    1
}

pub unsafe extern "C-unwind" fn set_window_bordered(l: *mut lua_State) -> c_int {
    let window = native::window(window_id(l));
    let bordered = lua_toboolean(l, 2) != 0;
    if let Some(window) = window {
        window.set_decorations(bordered);
    }
    0
}

pub unsafe extern "C-unwind" fn get_window_size(l: *mut lua_State) -> c_int {
    let id = window_id(l);
    let (w, h) = native::logical_size(id);
    // winit reports the position in physical pixels; SDL's API is in window points.
    let (x, y) = native::window(id)
        .and_then(|window| {
            let scale = window.scale_factor();
            window.inner_position().ok().map(|pos| {
                (
                    (pos.x as f64 / scale).round() as c_int,
                    (pos.y as f64 / scale).round() as c_int,
                )
            })
        })
        .unwrap_or((0, 0));
    lua_pushinteger(l, w as i64);
    lua_pushinteger(l, h as i64);
    lua_pushinteger(l, x as i64);
    lua_pushinteger(l, y as i64);
    4
}

pub unsafe extern "C-unwind" fn set_window_size(l: *mut lua_State) -> c_int {
    let renwindow = check_window(l);
    // Doubles in the C, truncated to int for the SDL calls; winit wants logical points.
    let (w, h) = (luaL_checknumber(l, 2) as c_int, luaL_checknumber(l, 3) as c_int);
    let (x, y) = (luaL_checknumber(l, 4) as c_int, luaL_checknumber(l, 5) as c_int);
    if let Some(window) = native::window(ren_get_window_id(renwindow)) {
        let _ = window.request_inner_size(LogicalSize::new(w as f64, h as f64));
        window.set_outer_position(LogicalPosition::new(x as f64, y as f64));
        ren_resize_window(renwindow);
    }
    0
}

pub unsafe extern "C-unwind" fn window_has_focus(l: *mut lua_State) -> c_int {
    let focused = native::window(window_id(l)).is_some_and(|window| window.has_focus());
    lua_pushboolean(l, focused as c_int);
    1
}

pub unsafe extern "C-unwind" fn raise_window(l: *mut lua_State) -> c_int {
    if let Some(window) = native::window(window_id(l)) {
        window.focus_window();
    }
    0
}

pub unsafe extern "C-unwind" fn set_window_opacity(l: *mut lua_State) -> c_int {
    // Whole-window opacity is an intentional no-op with the historical true return.
    // Keep checking both arguments so Lua errors and call sites behave unchanged.
    let _ = native::window(window_id(l));
    let _ = luaL_checknumber(l, 2);
    lua_pushboolean(l, 1);
    1
}

// ------------------------------------------------------------ hit testing

pub unsafe extern "C-unwind" fn set_window_hit_test(l: *mut lua_State) -> c_int {
    let id = window_id(l);
    let info = if lua_gettop(l) == 1 {
        None
    } else {
        Some([
            luaL_checknumber(l, 2) as c_int,
            luaL_checknumber(l, 3) as c_int,
            luaL_checknumber(l, 4) as c_int,
        ])
    };
    native::set_hit_test(id, info);
    0
}

// ------------------------------------------------------------- text input

pub unsafe extern "C-unwind" fn text_input(l: *mut lua_State) -> c_int {
    // The one window function that tolerates a NULL RenWindow.
    let renwindow = check_window(l);
    if renwindow.is_null() {
        return 0;
    }
    native::set_text_input(ren_get_window_id(renwindow), lua_toboolean(l, 2) != 0);
    0
}

pub unsafe extern "C-unwind" fn set_text_input_rect(l: *mut lua_State) -> c_int {
    let id = window_id(l);
    let rect = [
        luaL_checknumber(l, 2) as c_int,
        luaL_checknumber(l, 3) as c_int,
        luaL_checknumber(l, 4) as c_int,
        luaL_checknumber(l, 5) as c_int,
    ];
    native::set_ime_rect(id, rect);
    0
}

pub unsafe extern "C-unwind" fn clear_ime(l: *mut lua_State) -> c_int {
    native::clear_ime(window_id(l));
    0
}

// ------------------------------------------------------------- message box

pub unsafe extern "C-unwind" fn show_fatal_error(l: *mut lua_State) -> c_int {
    let title = luaL_checkstring(l, 1);
    let message = luaL_checkstring(l, 2);
    sdl::SDL_ShowSimpleMessageBox(sdl::SDL_MESSAGEBOX_ERROR, title, message, std::ptr::null_mut());
    0
}
