//! `src/api/renderer.c`: the `renderer` Lua module and the `Font` userdata.
//!
//! Line-by-line port. Fonts cross the Lua boundary as `Font` userdata holding a single
//! `RenFont *` (`sizeof(RenFont*)` bytes, exactly as the C), and a "font group" is a plain
//! Lua table of such userdata that carries the same `Font` metatable. The metatable's methods
//! are raw `lua_CFunction`s like `sysevents.rs`/`sysplugin.rs`; argument checking uses the real
//! `luaL_check*`/`luaL_typeerror` so errors read exactly as before.
//!
//! Two deliberate deviations from the C are noted below: the unknown-hinting error formats
//! the option string (the C passes the address of the enum to `%s`), and the group-table readers
//! reject an empty/mutated group with a Lua error instead of nil-dereferencing.

use std::ffi::{c_char, c_int, CStr};
use std::ptr;

use mlua::ffi::{
    luaL_argerror, luaL_checkany, luaL_checklstring, luaL_checknumber, luaL_checkstring,
    luaL_checktype, luaL_checkudata, luaL_error, luaL_getmetafield, luaL_len, luaL_newmetatable,
    luaL_ref, luaL_setmetatable, lua_CFunction, lua_Integer, lua_Number, lua_State,
    lua_createtable, lua_getfield, lua_gettop, lua_isnoneornil, lua_isnumber, lua_isstring,
    lua_istable, lua_newtable, lua_newuserdata, lua_pop, lua_pushboolean, lua_pushcfunction,
    lua_pushfstring, lua_pushnumber, lua_pushstring, lua_pushvalue, lua_rawgeti, lua_rawlen,
    lua_rawset, lua_rawseti, lua_setfield, lua_toboolean, lua_tonumber, lua_tostring, lua_type,
    lua_typename, LUA_REGISTRYINDEX, LUA_TLIGHTUSERDATA, LUA_TNIL, LUA_TNUMBER, LUA_TSTRING,
    LUA_TTABLE, LUA_TUSERDATA,
};
use sdl3_sys::everything as sdl;

use crate::rencache::{
    rencache_begin_frame, rencache_draw_rect, rencache_draw_text, rencache_end_frame,
    rencache_set_clip_rect, rencache_show_debug, RenColor, RenTab, FONT_FALLBACK_MAX,
};
#[cfg(feature = "sdl_renderer")]
use crate::renderer::update_font_scale;
use crate::renderer::{
    ren_font_copy, ren_font_free, ren_font_get_path, ren_font_group_get_height,
    ren_font_group_get_size, ren_font_group_get_width, ren_font_group_set_size,
    ren_font_group_set_tab_size, ren_font_load, RenFont,
};
#[cfg(feature = "sdl_renderer")]
use crate::renwindow::renwin_get_surface;
use crate::renwindow::{
    ren_get_size, ren_get_target_window, ren_set_target_window, RenRect, RenWindow,
};

/// `LUA_NOREF`; mlua-sys does not export the macro.
const LUA_NOREF: c_int = -2;

const FONT_TYPE: &CStr = c"Font";
/// `API_TYPE_FONT "(userdata)"`, the name `f_font_group` reports for a bad element.
const FONT_TYPE_USERDATA: &CStr = c"Font(userdata)";
const RENWINDOW_TYPE: &CStr = c"RenWindow";

// `ERenFontHinting` / `ERenFontAntialiasing` / `ERenFontStyle` from `renderer.h`.
const FONT_HINTING_NONE: c_int = 0;
const FONT_HINTING_SLIGHT: c_int = 1;
const FONT_HINTING_FULL: c_int = 2;

const FONT_ANTIALIASING_NONE: c_int = 0;
const FONT_ANTIALIASING_GRAYSCALE: c_int = 1;
const FONT_ANTIALIASING_SUBPIXEL: c_int = 2;

const FONT_STYLE_BOLD: c_int = 1;
const FONT_STYLE_ITALIC: c_int = 2;
const FONT_STYLE_UNDERLINE: c_int = 4;
const FONT_STYLE_SMOOTH: c_int = 8;
const FONT_STYLE_STRIKETHROUGH: c_int = 16;

// `luaL_typeerror`: declared here because mlua-sys leaves it out of its lua54 bindings.
extern "C-unwind" {
    fn luaL_typeerror(L: *mut lua_State, arg: c_int, tname: *const c_char) -> c_int;
}

/// A reference index to a table that stores the fonts, anchored for one frame.
static mut RENDERER_FONT_REF: c_int = LUA_NOREF;

// ------------------------------------------------------------- font options

/// `font_get_options`.
unsafe fn font_get_options(
    l: *mut lua_State,
    antialiasing: &mut c_int,
    hinting: &mut c_int,
    style: &mut c_int,
) -> c_int {
    if lua_gettop(l) > 2 && lua_istable(l, 3) != 0 {
        lua_getfield(l, 3, c"antialiasing".as_ptr());
        if lua_isstring(l, -1) != 0 {
            let s = lua_tostring(l, -1);
            if !s.is_null() {
                if CStr::from_ptr(s).to_bytes() == b"none" {
                    *antialiasing = FONT_ANTIALIASING_NONE;
                } else if CStr::from_ptr(s).to_bytes() == b"grayscale" {
                    *antialiasing = FONT_ANTIALIASING_GRAYSCALE;
                } else if CStr::from_ptr(s).to_bytes() == b"subpixel" {
                    *antialiasing = FONT_ANTIALIASING_SUBPIXEL;
                } else {
                    return luaL_error(
                        l,
                        c"error in font options, unknown antialiasing option: \"%s\"".as_ptr(),
                        s,
                    );
                }
            }
        }
        lua_getfield(l, 3, c"hinting".as_ptr());
        if lua_isstring(l, -1) != 0 {
            let s = lua_tostring(l, -1);
            if !s.is_null() {
                if CStr::from_ptr(s).to_bytes() == b"slight" {
                    *hinting = FONT_HINTING_SLIGHT;
                } else if CStr::from_ptr(s).to_bytes() == b"none" {
                    *hinting = FONT_HINTING_NONE;
                } else if CStr::from_ptr(s).to_bytes() == b"full" {
                    *hinting = FONT_HINTING_FULL;
                } else {
                    // The C formats the address of the enum here; format the option instead.
                    return luaL_error(
                        l,
                        c"error in font options, unknown hinting option: \"%s\"".as_ptr(),
                        s,
                    );
                }
            }
        }
        let mut style_local = 0;
        lua_getfield(l, 3, c"italic".as_ptr());
        if lua_toboolean(l, -1) != 0 {
            style_local |= FONT_STYLE_ITALIC;
        }
        lua_getfield(l, 3, c"bold".as_ptr());
        if lua_toboolean(l, -1) != 0 {
            style_local |= FONT_STYLE_BOLD;
        }
        lua_getfield(l, 3, c"underline".as_ptr());
        if lua_toboolean(l, -1) != 0 {
            style_local |= FONT_STYLE_UNDERLINE;
        }
        lua_getfield(l, 3, c"smoothing".as_ptr());
        if lua_toboolean(l, -1) != 0 {
            style_local |= FONT_STYLE_SMOOTH;
        }
        lua_getfield(l, 3, c"strikethrough".as_ptr());
        if lua_toboolean(l, -1) != 0 {
            style_local |= FONT_STYLE_STRIKETHROUGH;
        }

        // As C: the five style fields come off, antialiasing and hinting stay (discarded at return).
        lua_pop(l, 5);

        if style_local != 0 {
            *style = style_local;
        }
    }

    0
}

// --------------------------------------------------------------- font lib

/// `f_font_load`.
unsafe extern "C-unwind" fn f_font_load(l: *mut lua_State) -> c_int {
    let filename = luaL_checkstring(l, 1);
    let size = luaL_checknumber(l, 2) as f32;
    let mut style: c_int = 0;
    let mut hinting: c_int = FONT_HINTING_SLIGHT;
    let mut antialiasing: c_int = FONT_ANTIALIASING_SUBPIXEL;

    let ret_code = font_get_options(l, &mut antialiasing, &mut hinting, &mut style);
    if ret_code > 0 {
        return ret_code;
    }

    let font = lua_newuserdata(l, std::mem::size_of::<*mut RenFont>()) as *mut *mut RenFont;
    *font = ren_font_load(filename, size, antialiasing, hinting, style as u8);
    if (*font).is_null() {
        return luaL_error(l, c"failed to load font: %s".as_ptr(), sdl::SDL_GetError());
    }
    luaL_setmetatable(l, FONT_TYPE.as_ptr());
    1
}

/// `font_retrieve`: fill `fonts` with up to `FONT_FALLBACK_MAX` pointers, NULL-terminated.
/// Returns whether the argument was a table (a font group).
unsafe fn font_retrieve(
    l: *mut lua_State,
    fonts: &mut [*mut RenFont; FONT_FALLBACK_MAX],
    idx: c_int,
) -> bool {
    fonts.fill(ptr::null_mut());
    let is_table;
    if lua_type(l, idx) != LUA_TTABLE {
        fonts[0] = *(luaL_checkudata(l, idx, FONT_TYPE.as_ptr()) as *mut *mut RenFont);
        is_table = false;
    } else {
        is_table = true;
        let len = luaL_len(l, idx) as usize;
        let len = if len > FONT_FALLBACK_MAX {
            FONT_FALLBACK_MAX
        } else {
            len
        };
        for i in 0..len {
            lua_rawgeti(l, idx, (i + 1) as lua_Integer);
            fonts[i] = *(luaL_checkudata(l, -1, FONT_TYPE.as_ptr()) as *mut *mut RenFont);
            lua_pop(l, 1);
        }
        // An empty or mutated (hollowed-out) group would leave every slot NULL and the callers
        // below dereference `fonts[0]`; reject it as a Lua error rather than crash. `luaL_error`
        // does not return.
        if fonts[0].is_null() {
            luaL_error(l, c"failed to use font group: table is empty".as_ptr());
        }
    }
    #[cfg(feature = "sdl_renderer")]
    update_font_scale(ren_get_target_window(), fonts.as_mut_ptr());
    is_table
}

/// `f_font_copy`.
unsafe extern "C-unwind" fn f_font_copy(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    let table = font_retrieve(l, &mut fonts, 1);
    let size = if lua_gettop(l) >= 2 {
        luaL_checknumber(l, 2) as f32
    } else {
        ren_font_group_get_height(fonts.as_mut_ptr()) as f32
    };
    let mut style: c_int = -1;
    let mut hinting: c_int = -1;
    let mut antialiasing: c_int = -1;

    let ret_code = font_get_options(l, &mut antialiasing, &mut hinting, &mut style);
    if ret_code > 0 {
        return ret_code;
    }

    if table {
        lua_newtable(l);
        luaL_setmetatable(l, FONT_TYPE.as_ptr());
    }
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !fonts[i].is_null() {
        let font = lua_newuserdata(l, std::mem::size_of::<*mut RenFont>()) as *mut *mut RenFont;
        *font = ren_font_copy(fonts[i], size, antialiasing, hinting, style);
        if (*font).is_null() {
            return luaL_error(l, c"failed to copy font: %s".as_ptr(), sdl::SDL_GetError());
        }
        luaL_setmetatable(l, FONT_TYPE.as_ptr());
        if table {
            lua_rawseti(l, -2, (i + 1) as lua_Integer);
        }
        i += 1;
    }
    1
}

/// `f_font_group`.
unsafe extern "C-unwind" fn f_font_group(l: *mut lua_State) -> c_int {
    luaL_checktype(l, 1, LUA_TTABLE);

    let table_size = lua_rawlen(l, 1) as c_int;
    if table_size <= 0 {
        return luaL_error(l, c"failed to create font group: table is empty".as_ptr());
    }
    if table_size > FONT_FALLBACK_MAX as c_int {
        return luaL_error(
            l,
            c"failed to create font group: table size too large".as_ptr(),
        );
    }

    // Ensure there are no font groups inside it.
    for i in 1..=table_size {
        if lua_rawgeti(l, 1, i as lua_Integer) != LUA_TUSERDATA {
            return luaL_typeerror(l, -1, FONT_TYPE_USERDATA.as_ptr());
        }
        lua_pop(l, 1);
    }

    luaL_setmetatable(l, FONT_TYPE.as_ptr());
    1
}

/// `f_font_get_path`.
unsafe extern "C-unwind" fn f_font_get_path(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    let table = font_retrieve(l, &mut fonts, 1);

    if table {
        lua_newtable(l);
    }
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !fonts[i].is_null() {
        lua_pushstring(l, ren_font_get_path(fonts[i]));
        if table {
            lua_rawseti(l, -2, (i + 1) as lua_Integer);
        }
        i += 1;
    }
    1
}

/// `f_font_set_tab_size`.
unsafe extern "C-unwind" fn f_font_set_tab_size(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);
    let n = luaL_checknumber(l, 2) as c_int;
    ren_font_group_set_tab_size(fonts.as_mut_ptr(), n);
    0
}

/// `f_font_gc`.
unsafe extern "C-unwind" fn f_font_gc(l: *mut lua_State) -> c_int {
    if lua_istable(l, 1) != 0 {
        return 0; // do not run if it is a FontGroup
    }
    let self_ = luaL_checkudata(l, 1, FONT_TYPE.as_ptr()) as *mut *mut RenFont;
    ren_font_free(*self_);
    0
}

// ------------------------------------------------------------- font metrics

/// `checktab`.
unsafe fn checktab(l: *mut lua_State, idx: c_int) -> RenTab {
    let mut tab = RenTab { offset: f64::NAN };
    if lua_isnoneornil(l, idx) != 0 {
        return tab;
    }
    luaL_checktype(l, idx, LUA_TTABLE);
    if lua_getfield(l, idx, c"tab_offset".as_ptr()) == LUA_TNIL {
        return tab;
    }
    tab.offset = luaL_checknumber(l, -1) as f64;
    tab
}

/// `f_font_get_width`.
unsafe extern "C-unwind" fn f_font_get_width(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);
    let mut len: usize = 0;
    let text = luaL_checklstring(l, 2, &mut len);
    let tab = checktab(l, 3);

    lua_pushnumber(
        l,
        ren_font_group_get_width(fonts.as_mut_ptr(), text, len, tab, ptr::null_mut()),
    );
    1
}

/// `f_font_get_height`.
unsafe extern "C-unwind" fn f_font_get_height(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);
    lua_pushnumber(l, ren_font_group_get_height(fonts.as_mut_ptr()) as f64);
    1
}

/// `f_font_get_size`.
unsafe extern "C-unwind" fn f_font_get_size(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);
    lua_pushnumber(l, ren_font_group_get_size(fonts.as_mut_ptr()) as f64);
    1
}

/// `f_font_set_size`.
unsafe extern "C-unwind" fn f_font_set_size(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);
    let size = luaL_checknumber(l, 2) as f32;
    let scale: c_int;
    #[cfg(feature = "sdl_renderer")]
    {
        let window = ren_get_target_window();
        scale = if window.is_null() {
            1
        } else {
            renwin_get_surface(window).scale
        };
    }
    #[cfg(not(feature = "sdl_renderer"))]
    {
        scale = 1;
    }
    ren_font_group_set_size(fonts.as_mut_ptr(), size, scale);
    0
}

// ----------------------------------------------------------------- colours

/// `color_value_error`.
unsafe fn color_value_error(l: *mut lua_State, idx: c_int, table_idx: c_int) -> c_int {
    let type_name: *const c_char = if luaL_getmetafield(l, -1, c"__name".as_ptr()) == LUA_TSTRING {
        lua_tostring(l, -1) // metatable name
    } else if lua_type(l, -1) == LUA_TLIGHTUSERDATA {
        c"light userdata".as_ptr() // special name for light userdata
    } else {
        lua_typename(l, lua_type(l, -1)) // default name
    };
    let msg = lua_pushfstring(
        l,
        c"table[%d]: %s expected, got %s".as_ptr(),
        table_idx,
        lua_typename(l, LUA_TNUMBER),
        type_name,
    );
    luaL_argerror(l, idx, msg)
}

/// `get_color_value`.
unsafe fn get_color_value(l: *mut lua_State, idx: c_int, table_idx: c_int) -> c_int {
    lua_rawgeti(l, idx, table_idx as lua_Integer);
    if lua_isnumber(l, -1) != 0 {
        lua_tonumber(l, -1) as c_int
    } else {
        color_value_error(l, idx, table_idx)
    }
}

/// `get_color_value_opt`.
unsafe fn get_color_value_opt(
    l: *mut lua_State,
    idx: c_int,
    table_idx: c_int,
    default_value: c_int,
) -> c_int {
    lua_rawgeti(l, idx, table_idx as lua_Integer);
    if lua_isnoneornil(l, -1) != 0 {
        default_value
    } else if lua_isnumber(l, -1) != 0 {
        lua_tonumber(l, -1) as c_int
    } else {
        color_value_error(l, idx, table_idx)
    }
}

/// `checkcolor`.
unsafe fn checkcolor(l: *mut lua_State, idx: c_int, def: c_int) -> RenColor {
    if lua_isnoneornil(l, idx) != 0 {
        return RenColor {
            b: def as u8,
            g: def as u8,
            r: def as u8,
            a: 255,
        };
    }
    luaL_checktype(l, idx, LUA_TTABLE);
    let color = RenColor {
        r: get_color_value(l, idx, 1) as u8,
        g: get_color_value(l, idx, 2) as u8,
        b: get_color_value(l, idx, 3) as u8,
        a: get_color_value_opt(l, idx, 4, 255) as u8,
    };
    lua_pop(l, 4);
    color
}

// ------------------------------------------------------------- frame / draw

/// `f_show_debug`.
unsafe extern "C-unwind" fn f_show_debug(l: *mut lua_State) -> c_int {
    luaL_checkany(l, 1);
    rencache_show_debug(lua_toboolean(l, 1) != 0);
    0
}

/// `f_get_size`.
unsafe extern "C-unwind" fn f_get_size(l: *mut lua_State) -> c_int {
    let mut w: c_int = 0;
    let mut h: c_int = 0;
    let window = ren_get_target_window();
    if !window.is_null() {
        ren_get_size(window, &mut w, &mut h);
    }
    lua_pushnumber(l, w as f64);
    lua_pushnumber(l, h as f64);
    2
}

/// `f_begin_frame`.
unsafe extern "C-unwind" fn f_begin_frame(l: *mut lua_State) -> c_int {
    assert!(ren_get_target_window().is_null());
    let window = *(luaL_checkudata(l, 1, RENWINDOW_TYPE.as_ptr()) as *mut *mut RenWindow);
    ren_set_target_window(window);
    rencache_begin_frame(window);
    0
}

/// `f_end_frame`.
unsafe extern "C-unwind" fn f_end_frame(l: *mut lua_State) -> c_int {
    let window = ren_get_target_window();
    assert!(!window.is_null());
    rencache_end_frame(window);
    ren_set_target_window(ptr::null_mut());
    // PROTOTYPE: expose the present time as a global for the frame-timing probe.
    lua_pushnumber(
        l,
        crate::rencache::LAST_PRESENT_NS.load(std::sync::atomic::Ordering::Relaxed) as lua_Number
            / 1e9,
    );
    mlua::ffi::lua_setglobal(l, c"LXL_LAST_PRESENT_S".as_ptr());
    // Clear the font reference table.
    lua_newtable(l);
    lua_rawseti(l, LUA_REGISTRYINDEX, RENDERER_FONT_REF as lua_Integer);
    0
}

/// `rect_to_grid`.
unsafe fn rect_to_grid(x: lua_Number, y: lua_Number, w: lua_Number, h: lua_Number) -> RenRect {
    let x1 = (x + 0.5) as c_int;
    let y1 = (y + 0.5) as c_int;
    let x2 = (x + w + 0.5) as c_int;
    let y2 = (y + h + 0.5) as c_int;
    RenRect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    }
}

/// `f_set_clip_rect`.
unsafe extern "C-unwind" fn f_set_clip_rect(l: *mut lua_State) -> c_int {
    let x = luaL_checknumber(l, 1);
    let y = luaL_checknumber(l, 2);
    let w = luaL_checknumber(l, 3);
    let h = luaL_checknumber(l, 4);
    let rect = rect_to_grid(x, y, w, h);
    rencache_set_clip_rect(ren_get_target_window(), rect);
    0
}

/// `f_draw_rect`.
unsafe extern "C-unwind" fn f_draw_rect(l: *mut lua_State) -> c_int {
    let x = luaL_checknumber(l, 1);
    let y = luaL_checknumber(l, 2);
    let w = luaL_checknumber(l, 3);
    let h = luaL_checknumber(l, 4);
    let rect = rect_to_grid(x, y, w, h);
    let color = checkcolor(l, 5, 255);
    rencache_draw_rect(ren_get_target_window(), rect, color);
    0
}

/// `f_draw_text`.
unsafe extern "C-unwind" fn f_draw_text(l: *mut lua_State) -> c_int {
    let mut fonts = [ptr::null_mut(); FONT_FALLBACK_MAX];
    font_retrieve(l, &mut fonts, 1);

    // Stores a reference to this font in the reference table.
    lua_rawgeti(l, LUA_REGISTRYINDEX, RENDERER_FONT_REF as lua_Integer);
    if lua_istable(l, -1) != 0 {
        lua_pushvalue(l, 1);
        lua_pushboolean(l, 1);
        lua_rawset(l, -3);
    } else {
        eprintln!("warning: failed to reference count fonts");
    }
    lua_pop(l, 1);

    let mut len: usize = 0;
    let text = luaL_checklstring(l, 2, &mut len);
    let mut x = luaL_checknumber(l, 3);
    let y = luaL_checknumber(l, 4) as c_int;
    let color = checkcolor(l, 5, 255);
    let tab = checktab(l, 6);
    x = rencache_draw_text(
        ren_get_target_window(),
        fonts.as_mut_ptr(),
        text,
        len,
        x,
        y,
        color,
        tab,
    );
    lua_pushnumber(l, x);
    1
}

// ------------------------------------------------------------------ open

/// `luaopen_renderer`.
#[no_mangle]
pub unsafe extern "C-unwind" fn luaopen_renderer(l: *mut lua_State) -> c_int {
    // Get a reference in the registry to store the font anchors for one frame.
    lua_newtable(l);
    RENDERER_FONT_REF = luaL_ref(l, LUA_REGISTRYINDEX);

    // `luaL_newlib` is a macro mlua-sys does not export; build the table by hand.
    let lib: [(&CStr, lua_CFunction); 7] = [
        (c"show_debug", f_show_debug),
        (c"get_size", f_get_size),
        (c"begin_frame", f_begin_frame),
        (c"end_frame", f_end_frame),
        (c"set_clip_rect", f_set_clip_rect),
        (c"draw_rect", f_draw_rect),
        (c"draw_text", f_draw_text),
    ];
    lua_createtable(l, 0, lib.len() as c_int);
    for (name, func) in lib {
        lua_pushcfunction(l, func);
        lua_setfield(l, -2, name.as_ptr());
    }

    let font_lib: [(&CStr, lua_CFunction); 10] = [
        (c"__gc", f_font_gc),
        (c"load", f_font_load),
        (c"copy", f_font_copy),
        (c"group", f_font_group),
        (c"set_tab_size", f_font_set_tab_size),
        (c"get_width", f_font_get_width),
        (c"get_height", f_font_get_height),
        (c"get_size", f_font_get_size),
        (c"set_size", f_font_set_size),
        (c"get_path", f_font_get_path),
    ];
    luaL_newmetatable(l, FONT_TYPE.as_ptr());
    for (name, func) in font_lib {
        lua_pushcfunction(l, func);
        lua_setfield(l, -2, name.as_ptr());
    }
    lua_pushvalue(l, -1);
    lua_setfield(l, -2, c"__index".as_ptr());
    lua_setfield(l, -2, c"font".as_ptr());
    1
}
