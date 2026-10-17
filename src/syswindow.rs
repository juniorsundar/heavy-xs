//! Window, cursor, text-input and message-box functions from `src/api/system.c`.
//!
//! These take a `RenWindow` userdata, which in C is `RenWindow **` created by
//! `src/api/renwindow.c`. This file only ever needs the `SDL_Window *` inside it, which
//! `ren_get_sdl_window` returns, so the struct stays opaque to Rust.
//!
//! Raw `lua_CFunction`s, like `sysevents.rs`: argument checking uses the real
//! `luaL_check*` so type errors read exactly as before ("bad argument #1 to ...").

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_void};

use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushstring, lua_toboolean,
    luaL_checknumber, luaL_checkoption, luaL_checkstring, luaL_checkudata,
};
use sdl3_sys::everything as sdl;

use crate::hit;

extern "C" {
    fn ren_get_sdl_window(window: *mut c_void) -> *mut sdl::SDL_Window;
    fn ren_resize_window(window: *mut c_void);
}

/// `luaL_checkudata(L, 1, "RenWindow")`, dereferenced: the `RenWindow *`. The C does
/// not check it for NULL either, except `text_input`, which returns quietly.
unsafe fn check_window(l: *mut lua_State) -> *mut c_void {
    *(luaL_checkudata(l, 1, c"RenWindow".as_ptr()) as *mut *mut c_void)
}

unsafe fn check_sdl_window(l: *mut lua_State) -> *mut sdl::SDL_Window {
    ren_get_sdl_window(check_window(l))
}

// ----------------------------------------------------------------- cursor

/// One cache slot per system cursor, never freed (SDL owns them until exit).
struct CursorCache(UnsafeCell<[*mut sdl::SDL_Cursor; 6]>);
// Lua runs on one thread; the cache is only touched from `set_cursor`.
unsafe impl Sync for CursorCache {}
static CURSORS: CursorCache = CursorCache(UnsafeCell::new([std::ptr::null_mut(); 6]));

pub unsafe extern "C-unwind" fn set_cursor(l: *mut lua_State) -> c_int {
    const NAMES: [*const c_char; 6] = [
        c"arrow".as_ptr(),
        c"ibeam".as_ptr(),
        c"sizeh".as_ptr(),
        c"sizev".as_ptr(),
        c"hand".as_ptr(),
        std::ptr::null(),
    ];
    // Indexed by the option, like the C's cursor_enums.
    const CURSORS_BY_OPTION: [sdl::SDL_SystemCursor; 5] = [
        sdl::SDL_SystemCursor::DEFAULT,
        sdl::SDL_SystemCursor::TEXT,
        sdl::SDL_SystemCursor::EW_RESIZE,
        sdl::SDL_SystemCursor::NS_RESIZE,
        sdl::SDL_SystemCursor::POINTER,
    ];
    let option = luaL_checkoption(l, 1, c"arrow".as_ptr(), NAMES.as_ptr()) as usize;
    let kind = CURSORS_BY_OPTION[option];

    // The C indexes cursor_cache by the SDL enum value; this indexes by option, which is
    // the same one-to-one mapping and keeps the array at the five slots actually used.
    let cache = &mut *CURSORS.0.get();
    if cache[option].is_null() {
        cache[option] = sdl::SDL_CreateSystemCursor(kind);
    }
    sdl::SDL_SetCursor(cache[option]);
    0
}

// ----------------------------------------------------------------- window

pub unsafe extern "C-unwind" fn set_window_title(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    let title = luaL_checkstring(l, 2);
    sdl::SDL_SetWindowTitle(window, title);
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

    let window = check_sdl_window(l);
    let mode = luaL_checkoption(l, 2, c"normal".as_ptr(), NAMES.as_ptr());
    sdl::SDL_SetWindowFullscreen(window, mode == FULLSCREEN);
    if mode == NORMAL {
        sdl::SDL_RestoreWindow(window);
    }
    if mode == MAXIMIZED {
        sdl::SDL_MaximizeWindow(window);
    }
    if mode == MINIMIZED {
        sdl::SDL_MinimizeWindow(window);
    }
    0
}

pub unsafe extern "C-unwind" fn get_window_mode(l: *mut lua_State) -> c_int {
    let flags = sdl::SDL_GetWindowFlags(check_sdl_window(l));
    let name: &[u8] = if flags & sdl::SDL_WINDOW_FULLSCREEN != 0 {
        b"fullscreen\0"
    } else if flags & sdl::SDL_WINDOW_MINIMIZED != 0 {
        b"minimized\0"
    } else if flags & sdl::SDL_WINDOW_MAXIMIZED != 0 {
        b"maximized\0"
    } else {
        b"normal\0"
    };
    lua_pushstring(l, name.as_ptr() as *const c_char);
    1
}

pub unsafe extern "C-unwind" fn set_window_bordered(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    sdl::SDL_SetWindowBordered(window, lua_toboolean(l, 2) != 0);
    0
}

pub unsafe extern "C-unwind" fn get_window_size(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    let (mut w, mut h, mut x, mut y) = (0, 0, 0, 0);
    sdl::SDL_GetWindowSize(window, &mut w, &mut h);
    sdl::SDL_GetWindowPosition(window, &mut x, &mut y);
    lua_pushinteger(l, w as i64);
    lua_pushinteger(l, h as i64);
    lua_pushinteger(l, x as i64);
    lua_pushinteger(l, y as i64);
    4
}

pub unsafe extern "C-unwind" fn set_window_size(l: *mut lua_State) -> c_int {
    let renwindow = check_window(l);
    let window = ren_get_sdl_window(renwindow);
    // Doubles in the C, truncated by the int parameters of the SDL calls.
    let (w, h) = (luaL_checknumber(l, 2), luaL_checknumber(l, 3));
    let (x, y) = (luaL_checknumber(l, 4), luaL_checknumber(l, 5));
    sdl::SDL_SetWindowSize(window, w as c_int, h as c_int);
    sdl::SDL_SetWindowPosition(window, x as c_int, y as c_int);
    ren_resize_window(renwindow);
    0
}

pub unsafe extern "C-unwind" fn window_has_focus(l: *mut lua_State) -> c_int {
    let flags = sdl::SDL_GetWindowFlags(check_sdl_window(l));
    lua_pushboolean(l, (flags & sdl::SDL_WINDOW_INPUT_FOCUS != 0) as c_int);
    1
}

pub unsafe extern "C-unwind" fn raise_window(l: *mut lua_State) -> c_int {
    sdl::SDL_RaiseWindow(check_sdl_window(l));
    0
}

pub unsafe extern "C-unwind" fn set_window_opacity(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    let opacity = luaL_checknumber(l, 2);
    // shortcut: the C does `int r = SDL_SetWindowOpacity(...); push(r > -1)`, left over from
    // SDL2's 0/-1 return. SDL3 returns bool, so r is 0 or 1 and this is always true, even when
    // the call failed. Kept bug-compatible; flip to the real result once callers need it.
    sdl::SDL_SetWindowOpacity(window, opacity as f32);
    lua_pushboolean(l, 1);
    1
}

// ------------------------------------------------------------ hit testing

/// The three numbers `hit_test` reads. One global shared by every window: the C keeps a
/// single `window_hit_info[1]`, so the last `set_window_hit_test` wins.
struct HitInfo(UnsafeCell<[c_int; 3]>);
unsafe impl Sync for HitInfo {}
static HIT_INFO: HitInfo = HitInfo(UnsafeCell::new([0; 3]));

unsafe extern "C" fn hit_test(
    window: *mut sdl::SDL_Window,
    point: *const sdl::SDL_Point,
    data: *mut c_void,
) -> sdl::SDL_HitTestResult {
    let info = *(data as *const [c_int; 3]);
    let (mut w, mut h) = (0, 0);
    sdl::SDL_GetWindowSize(window, &mut w, &mut h);
    sdl::SDL_HitTestResult(hit::classify((*point).x, (*point).y, w, h, info[0], info[1], info[2]))
}

pub unsafe extern "C-unwind" fn set_window_hit_test(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    if lua_gettop(l) == 1 {
        sdl::SDL_SetWindowHitTest(window, None, std::ptr::null_mut());
        return 0;
    }
    let info = &mut *HIT_INFO.0.get();
    info[0] = luaL_checknumber(l, 2) as c_int;
    info[1] = luaL_checknumber(l, 3) as c_int;
    info[2] = luaL_checknumber(l, 4) as c_int;
    sdl::SDL_SetWindowHitTest(window, Some(hit_test), info.as_mut_ptr() as *mut c_void);
    0
}

// ------------------------------------------------------------- text input

pub unsafe extern "C-unwind" fn text_input(l: *mut lua_State) -> c_int {
    // The one window function that tolerates a NULL RenWindow.
    let renwindow = check_window(l);
    if renwindow.is_null() {
        return 0;
    }
    let window = ren_get_sdl_window(renwindow);
    if lua_toboolean(l, 2) != 0 {
        sdl::SDL_StartTextInput(window);
    } else {
        sdl::SDL_StopTextInput(window);
    }
    0
}

pub unsafe extern "C-unwind" fn set_text_input_rect(l: *mut lua_State) -> c_int {
    let window = check_sdl_window(l);
    let rect = sdl::SDL_Rect {
        x: luaL_checknumber(l, 2) as c_int,
        y: luaL_checknumber(l, 3) as c_int,
        w: luaL_checknumber(l, 4) as c_int,
        h: luaL_checknumber(l, 5) as c_int,
    };
    sdl::SDL_SetTextInputArea(window, &rect, 0);
    0
}

pub unsafe extern "C-unwind" fn clear_ime(l: *mut lua_State) -> c_int {
    sdl::SDL_ClearComposition(check_sdl_window(l));
    0
}

// ------------------------------------------------------------- message box

pub unsafe extern "C-unwind" fn show_fatal_error(l: *mut lua_State) -> c_int {
    let title = luaL_checkstring(l, 1);
    let message = luaL_checkstring(l, 2);
    sdl::SDL_ShowSimpleMessageBox(sdl::SDL_MESSAGEBOX_ERROR, title, message, std::ptr::null_mut());
    0
}
