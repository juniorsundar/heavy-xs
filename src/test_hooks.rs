//! Test-only: inject owned records before Lua packing and read back the Rust draw target.
//!
//! Compiled only with the `test_hooks` cargo feature; shipping builds have no
//! injection surface (a plugin could otherwise forge input), which
//! `scripts/rust-port/normal-build-check.sh` checks. The event goldens are historical
//! C recordings: that implementation is gone and nothing in this binary selects it.
//!
//! `lxl_test.push_event{ type = "key_down", ... }` pushes one event through
//! the shared owned-input queue after decoding the historical numeric event fixtures.
//! Native capture is validated separately; this is not OS-input evidence.

use std::ffi::{c_int, c_void};

use mlua::{AnyUserData, Error as LuaError, Lua, Result as LuaResult, Table, Value};

use crate::custom_events::{
    get_custom_event_callback_by_name, get_custom_event_name, push_custom_event, register_custom_event,
    CustomEvent, CustomEventCallback, FIRST_TYPE,
};
use crate::platform::{Custom, FingerKind, Input, Payload};
use crate::renwindow::{renwin_get_surface, RenSurface, RenWindow};

extern "C" {
    fn ren_get_window_id(window: *mut c_void) -> u32;
}

/// A callback that pushes ("custom", code, data1 as integer) so a test can see it ran.
unsafe fn tracer(l: *mut mlua::lua_State, e: &Custom) -> i32 {
    mlua::ffi::lua_pushstring(l, c"custom".as_ptr());
    mlua::ffi::lua_pushinteger(l, e.code as i64);
    mlua::ffi::lua_pushinteger(l, e.data1 as i64);
    3
}

/// A callback that returns 0 values, which poll_event must treat as "next event".
unsafe fn silent(_l: *mut mlua::lua_State, _e: &Custom) -> i32 {
    0
}

#[repr(C)]
struct RenRectRaw {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
}

extern "C" {
    /// The dirty rects of the frame just ended. Implemented by rencache (Rust after W6; the C
    /// version that recorded the golden is gone).
    fn rencache_debug_rects(rects: *mut *mut RenRectRaw) -> usize;
}

/// "x,y,w,h" per dirty rect, in the order rencache_end_frame produced them. Deterministic for a
/// fixed scene, which is what makes the rencache contract testable at all.
fn dirty_rects(_: &Lua, _: ()) -> LuaResult<String> {
    let mut rects: *mut RenRectRaw = std::ptr::null_mut();
    let count = unsafe { rencache_debug_rects(&mut rects) };
    let mut out = Vec::new();
    for i in 0..count {
        let r = unsafe { &*rects.add(i) };
        out.push(format!("{},{},{},{}", r.x, r.y, r.width, r.height));
    }
    Ok(out.join(" "))
}

/// The logical r,g,b of one window-surface pixel (8 bits per channel, as `SDL_GetRGBA` read it),
/// or `None` out of bounds.
fn surface_rgb(rs: &RenSurface, x: c_int, y: c_int) -> Option<(u8, u8, u8)> {
    if x < 0 || y < 0 || x >= rs.w || y >= rs.h || rs.pixels.is_null() {
        return None;
    }
    let offset = y as usize * rs.pitch as usize + x as usize * rs.bytes_per_pixel as usize;
    let mut pixel: u32 = 0;
    unsafe {
        std::ptr::copy_nonoverlapping(
            rs.pixels.add(offset),
            &mut pixel as *mut u32 as *mut u8,
            (rs.bytes_per_pixel as usize).min(4),
        )
    };
    Some((rs.r.get(pixel) as u8, rs.g.get(pixel) as u8, rs.b.get(pixel) as u8))
}

/// The b,g,r,a bytes of one surface pixel of a window, or nil when it is out of bounds. Used to
/// check that a replayed dirty rect really painted: dirty rects alone would not catch a replay
/// that draws nothing.
fn pixel(_: &Lua, (window, x, y): (mlua::AnyUserData, c_int, c_int)) -> LuaResult<Option<String>> {
    // The userdata holds a RenWindow*, like the C's `RenWindow **`.
    let ren = unsafe { *(window.to_pointer() as *mut *mut RenWindow) };
    let rs = unsafe { renwin_get_surface(ren) };
    // Colour only: alpha is 0 in some surface formats and 255 in others for an untouched
    // pixel, which is a format artifact rather than cache behaviour.
    Ok(surface_rgb(&rs, x, y).map(|(r, g, b)| format!("{r:02x}{g:02x}{b:02x}")))
}

/// FNV-1a over the logical r,g,b of every pixel in a rectangle of the window surface, plus the
/// count of non-background pixels. One short string pins a whole rendered text run: a glyph that
/// is a pixel off, hinted differently or blended differently changes it. Alpha is excluded for
/// the same reason `pixel` excludes it.
fn surface_hash(
    _: &Lua,
    (window, rx, ry, rw, rh): (mlua::AnyUserData, c_int, c_int, c_int, c_int),
) -> LuaResult<String> {
    let ren = unsafe { *(window.to_pointer() as *mut *mut RenWindow) };
    let rs = unsafe { renwin_get_surface(ren) };
    let (mut hash, mut lit) = (2166136261u32, 0usize);
    for y in ry.max(0)..(ry + rh).min(rs.h) {
        for x in rx.max(0)..(rx + rw).min(rs.w) {
            let Some((r, g, b)) = surface_rgb(&rs, x, y) else { continue };
            for byte in [r, g, b] {
                hash = (hash ^ byte as u32).wrapping_mul(16777619);
            }
            if (r, g, b) != (0, 0, 0) {
                lit += 1;
            }
        }
    }
    Ok(format!("{hash:08x}/{lit}"))
}

/// The name up to its first NUL, as the C string the hooks used to pass.
fn c_name(name: &str) -> &[u8] {
    name.as_bytes().split(|&b| b == 0).next().unwrap_or_default()
}

fn custom_register(_: &Lua, (name, kind): (String, String)) -> LuaResult<bool> {
    let callback: CustomEventCallback = match kind.as_str() {
        "tracer" => Some(tracer),
        "silent" => Some(silent),
        _ => None,
    };
    Ok(register_custom_event(c_name(&name), callback).is_ok())
}

fn custom_push(_: &Lua, (name, code, data1): (String, i32, i64)) -> LuaResult<bool> {
    let event = CustomEvent { code, data1: data1 as usize, payload: None };
    Ok(push_custom_event(c_name(&name), event).is_ok())
}

/// The type number of a registered name, as an offset from the first custom type, or nil.
fn custom_type(_: &Lua, name: String) -> LuaResult<Option<i64>> {
    // Walk the user range: the API only maps type -> name, so invert it.
    for ty in FIRST_TYPE..FIRST_TYPE + 0x40 {
        if get_custom_event_name(ty).is_some_and(|found| found == c_name(&name)) {
            return Ok(Some((ty - FIRST_TYPE) as i64));
        }
    }
    Ok(None)
}

fn custom_name(_: &Lua, offset: i64) -> LuaResult<Option<String>> {
    let found = get_custom_event_name(FIRST_TYPE.wrapping_add(offset as u32));
    Ok(found.map(|name| String::from_utf8_lossy(&name).into_owned()))
}

fn custom_has_callback(_: &Lua, name: String) -> LuaResult<bool> {
    Ok(get_custom_event_callback_by_name(c_name(&name)).is_some())
}

fn num(t: &Table, key: &str) -> LuaResult<f64> {
    Ok(match t.raw_get::<Value>(key)? {
        Value::Integer(i) => i as f64,
        Value::Number(n) => n,
        _ => 0.0,
    })
}

/// The window id of a `renwindow` userdata (it holds a `RenWindow **`), or 0 without one.
fn window_id(t: &Table) -> LuaResult<u32> {
    match t.raw_get::<Value>("window")? {
        Value::UserData(ud) => {
            let ud: AnyUserData = ud;
            let slot = ud.to_pointer() as *const *mut c_void;
            let window = unsafe { *slot };
            Ok(unsafe { ren_get_window_id(window) })
        }
        _ => Ok(0),
    }
}

/// The historical fixtures' numeric event types: SDL 3's values, which the recorded goldens and
/// `lxl_test.EVENT_*` constants still use. Only the types a fixture can name are listed.
mod etype {
    pub const QUIT: u32 = 0x100;
    pub const TERMINATING: u32 = 0x101;
    pub const WILL_ENTER_BACKGROUND: u32 = 0x103;
    pub const DID_ENTER_BACKGROUND: u32 = 0x104;
    pub const WILL_ENTER_FOREGROUND: u32 = 0x105;
    pub const DID_ENTER_FOREGROUND: u32 = 0x106;
    pub const WINDOW_SHOWN: u32 = 0x202;
    pub const WINDOW_EXPOSED: u32 = 0x204;
    pub const WINDOW_RESIZED: u32 = 0x206;
    pub const WINDOW_PIXEL_SIZE_CHANGED: u32 = 0x207;
    pub const WINDOW_MINIMIZED: u32 = 0x209;
    pub const WINDOW_MAXIMIZED: u32 = 0x20A;
    pub const WINDOW_RESTORED: u32 = 0x20B;
    pub const WINDOW_MOUSE_LEAVE: u32 = 0x20D;
    pub const WINDOW_FOCUS_GAINED: u32 = 0x20E;
    pub const WINDOW_FOCUS_LOST: u32 = 0x20F;
    pub const WINDOW_DISPLAY_SCALE_CHANGED: u32 = 0x214;
}

/// SDL's NumLock bit in the fixtures' `mod` field.
const KMOD_NUM: u16 = 0x1000;

/// A window, lifecycle or other event named by its numeric type. Window events carry the window
/// and data1/data2; user-range types become an unregistered custom event with no data.
fn numbered(ty: u32, window: u32, data1: i32, data2: i32) -> Input {
    match ty {
        etype::QUIT => Input::Quit,
        etype::WINDOW_RESIZED => Input::Resized { window, w: data1, h: data2 },
        etype::WINDOW_DISPLAY_SCALE_CHANGED | etype::WINDOW_PIXEL_SIZE_CHANGED => Input::Rescaled { window },
        etype::WINDOW_EXPOSED => Input::Exposed,
        etype::WINDOW_MINIMIZED => Input::Minimized,
        etype::WINDOW_MAXIMIZED => Input::Maximized,
        etype::WINDOW_RESTORED => Input::Restored,
        etype::WINDOW_MOUSE_LEAVE => Input::MouseLeft,
        etype::WINDOW_FOCUS_LOST => Input::FocusLost,
        etype::WINDOW_FOCUS_GAINED => Input::FocusGained,
        etype::WILL_ENTER_FOREGROUND => Input::EnteringForeground,
        etype::DID_ENTER_FOREGROUND => Input::EnteredForeground,
        etype::WILL_ENTER_BACKGROUND => Input::EnteringBackground,
        etype::DID_ENTER_BACKGROUND => Input::EnteredBackground,
        _ if ty >= FIRST_TYPE => Input::Custom(Custom { event_type: ty, code: 0, data1: 0, payload: None }),
        _ => Input::Other,
    }
}

fn push_event(_: &Lua, spec: Table) -> LuaResult<bool> {
    let kind: String = spec.raw_get("type")?;
    let text = spec.raw_get::<Option<mlua::LuaString>>("text")?.map(|s| s.as_bytes().to_vec()).unwrap_or_default();
    // Fixture text was a C string: it ends at the first NUL.
    let text = text.split(|&b| b == 0).next().unwrap_or_default().to_vec();
    let window = window_id(&spec)?;

    let input = match kind.as_str() {
        "quit" => Input::Quit,
        "key_down" | "key_up" => Input::Key {
            down: kind == "key_down",
            scancode: num(&spec, "scancode")? as i32,
            key: num(&spec, "key")? as u32,
            numlock: num(&spec, "mod")? as u16 & KMOD_NUM != 0,
        },
        "text_input" => Input::TextInput(text),
        "text_editing" => Input::TextEditing {
            text,
            start: num(&spec, "start")? as i32,
            length: num(&spec, "length")? as i32,
        },
        "mouse_down" | "mouse_up" => Input::MouseButton {
            window,
            down: kind == "mouse_down",
            button: num(&spec, "button")? as u8,
            x: num(&spec, "x")? as f32,
            y: num(&spec, "y")? as f32,
            clicks: num(&spec, "clicks")? as u8,
        },
        "mouse_motion" => Input::MouseMotion {
            window,
            x: num(&spec, "x")? as f32,
            y: num(&spec, "y")? as f32,
            xrel: num(&spec, "xrel")? as f32,
            yrel: num(&spec, "yrel")? as f32,
        },
        "mouse_wheel" => Input::MouseWheel { x: num(&spec, "x")? as f32, y: num(&spec, "y")? as f32 },
        "finger_down" | "finger_up" | "finger_motion" => Input::Finger {
            window,
            kind: match kind.as_str() {
                "finger_down" => FingerKind::Down,
                "finger_up" => FingerKind::Up,
                _ => FingerKind::Motion,
            },
            x: num(&spec, "x")? as f32,
            y: num(&spec, "y")? as f32,
            dx: num(&spec, "dx")? as f32,
            dy: num(&spec, "dy")? as f32,
            finger: num(&spec, "finger")? as u64,
        },
        "drop_file" => Input::FileDropped { window, path: text },
        "window" => {
            // The historical producer rejected window notifications without a registered window.
            if window == 0 {
                return Ok(true);
            }
            let (data1, data2) = (num(&spec, "data1")? as i32, num(&spec, "data2")? as i32);
            numbered(num(&spec, "etype")? as u32, window, data1, data2)
        }
        // Foreground / background lifecycle events and anything else, by numeric type.
        "app" | "other" => numbered(num(&spec, "etype")? as u32, 0, 0, 0),
        "dialog_finished" => {
            // A forged `dialogfinished` event: what dialog_callback pushes when a dialog
            // ends, with the same owned payload.
            let code = num(&spec, "code")? as i32;
            let paths: Vec<mlua::LuaString> =
                spec.raw_get::<Option<Vec<mlua::LuaString>>>("paths")?.unwrap_or_default();
            let payload = if code == 1 && paths.is_empty() && text.is_empty() {
                None // DIALOG_CANCEL carries no data
            } else if code == 0 {
                Some(Payload::Paths(paths.iter().map(|p| p.as_bytes().to_vec()).collect()))
            } else {
                Some(Payload::Message(text)) // DIALOG_ERROR message
            };
            let event = CustomEvent { code, data1: num(&spec, "id")? as usize, payload };
            return Ok(push_custom_event(b"dialogfinished", event).is_ok());
        }
        other => return Err(LuaError::runtime(format!("unknown test event type '{other}'"))),
    };
    Ok(crate::platform::post(input))
}

/// Installs the global `lxl_test` table.
pub fn install(lua: &Lua) -> LuaResult<()> {
    let hooks = lua.create_table()?;
    hooks.raw_set("push_event", lua.create_function(push_event)?)?;
    hooks.raw_set("dirty_rects", lua.create_function(dirty_rects)?)?;
    hooks.raw_set("pixel", lua.create_function(pixel)?)?;
    hooks.raw_set("surface_hash", lua.create_function(surface_hash)?)?;
    hooks.raw_set("custom_register", lua.create_function(custom_register)?)?;
    hooks.raw_set("custom_push", lua.create_function(custom_push)?)?;
    hooks.raw_set("custom_type", lua.create_function(custom_type)?)?;
    hooks.raw_set("custom_name", lua.create_function(custom_name)?)?;
    hooks.raw_set("custom_has_callback", lua.create_function(custom_has_callback)?)?;
    for (name, ty) in [
        ("EVENT_WINDOW_SHOWN", etype::WINDOW_SHOWN),
        ("EVENT_WINDOW_EXPOSED", etype::WINDOW_EXPOSED),
        ("EVENT_WINDOW_RESIZED", etype::WINDOW_RESIZED),
        ("EVENT_WINDOW_MINIMIZED", etype::WINDOW_MINIMIZED),
        ("EVENT_WINDOW_MAXIMIZED", etype::WINDOW_MAXIMIZED),
        ("EVENT_WINDOW_RESTORED", etype::WINDOW_RESTORED),
        ("EVENT_WINDOW_MOUSE_LEAVE", etype::WINDOW_MOUSE_LEAVE),
        ("EVENT_WINDOW_FOCUS_GAINED", etype::WINDOW_FOCUS_GAINED),
        ("EVENT_WINDOW_FOCUS_LOST", etype::WINDOW_FOCUS_LOST),
        ("EVENT_WINDOW_PIXEL_SIZE_CHANGED", etype::WINDOW_PIXEL_SIZE_CHANGED),
        ("EVENT_WINDOW_DISPLAY_SCALE_CHANGED", etype::WINDOW_DISPLAY_SCALE_CHANGED),
        ("EVENT_WILL_ENTER_BACKGROUND", etype::WILL_ENTER_BACKGROUND),
        ("EVENT_DID_ENTER_BACKGROUND", etype::DID_ENTER_BACKGROUND),
        ("EVENT_WILL_ENTER_FOREGROUND", etype::WILL_ENTER_FOREGROUND),
        ("EVENT_DID_ENTER_FOREGROUND", etype::DID_ENTER_FOREGROUND),
        ("EVENT_USER", FIRST_TYPE),
        ("EVENT_TERMINATING", etype::TERMINATING),
    ] {
        hooks.raw_set(name, ty as i64)?;
    }
    lua.globals().set("lxl_test", hooks)
}
