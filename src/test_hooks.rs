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

use std::ffi::{c_int, c_void, CString};

use mlua::{AnyUserData, Error as LuaError, Lua, Result as LuaResult, Table, Value};
use sdl3_sys::everything as sdl;

use crate::custom_events::{
    get_custom_event_callback_by_name, get_custom_event_name, push_custom_event, register_custom_event,
    CustomEvent, CustomEventCallback, FIRST_TYPE,
};
use crate::platform::{Custom, Payload};
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

/// The SDL window id of a `renwindow` userdata (it holds a `RenWindow **`).
fn window_id(t: &Table) -> LuaResult<sdl::SDL_WindowID> {
    match t.raw_get::<Value>("window")? {
        Value::UserData(ud) => {
            let ud: AnyUserData = ud;
            let slot = ud.to_pointer() as *const *mut c_void;
            let window = unsafe { *slot };
            Ok(sdl::SDL_WindowID(unsafe { ren_get_window_id(window) }))
        }
        _ => Ok(sdl::SDL_WindowID(0)),
    }
}

fn push_event(_: &Lua, spec: Table) -> LuaResult<bool> {
    use sdl::SDL_EventType as T;

    let kind: String = spec.raw_get("type")?;
    let mut e: sdl::SDL_Event = unsafe { std::mem::zeroed() };
    let text = spec.raw_get::<Option<mlua::LuaString>>("text")?.map(|s| s.as_bytes().to_vec()).unwrap_or_default();
    let text = CString::new(text.split(|&b| b == 0).next().unwrap_or_default()).unwrap();
    let wid = window_id(&spec)?;
    // The decoder copies borrowed fixture text before this callback returns.
    let owned_text = text.as_ptr();

    unsafe {
        match kind.as_str() {
            "quit" => e.r#type = T::QUIT.0,
            "key_down" | "key_up" => {
                e.key.r#type = if kind == "key_down" { T::KEY_DOWN } else { T::KEY_UP };
                e.key.scancode = sdl::SDL_Scancode(num(&spec, "scancode")? as i32);
                e.key.key = sdl::SDL_Keycode(num(&spec, "key")? as u32);
                e.key.r#mod = sdl::SDL_Keymod(num(&spec, "mod")? as u16);
                e.key.down = kind == "key_down";
            }
            "text_input" => {
                e.text.r#type = T::TEXT_INPUT;
                e.text.text = owned_text;
            }
            "text_editing" => {
                e.edit.r#type = T::TEXT_EDITING;
                e.edit.text = owned_text;
                e.edit.start = num(&spec, "start")? as i32;
                e.edit.length = num(&spec, "length")? as i32;
            }
            "mouse_down" | "mouse_up" => {
                e.button.r#type = if kind == "mouse_down" { T::MOUSE_BUTTON_DOWN } else { T::MOUSE_BUTTON_UP };
                e.button.windowID = wid;
                e.button.button = num(&spec, "button")? as u8;
                e.button.clicks = num(&spec, "clicks")? as u8;
                e.button.down = kind == "mouse_down";
                e.button.x = num(&spec, "x")? as f32;
                e.button.y = num(&spec, "y")? as f32;
            }
            "mouse_motion" => {
                e.motion.r#type = T::MOUSE_MOTION;
                e.motion.windowID = wid;
                e.motion.x = num(&spec, "x")? as f32;
                e.motion.y = num(&spec, "y")? as f32;
                e.motion.xrel = num(&spec, "xrel")? as f32;
                e.motion.yrel = num(&spec, "yrel")? as f32;
            }
            "mouse_wheel" => {
                e.wheel.r#type = T::MOUSE_WHEEL;
                e.wheel.windowID = wid;
                e.wheel.x = num(&spec, "x")? as f32;
                e.wheel.y = num(&spec, "y")? as f32;
            }
            "finger_down" | "finger_up" | "finger_motion" => {
                e.tfinger.r#type = match kind.as_str() {
                    "finger_down" => T::FINGER_DOWN,
                    "finger_up" => T::FINGER_UP,
                    _ => T::FINGER_MOTION,
                };
                e.tfinger.windowID = wid;
                e.tfinger.fingerID = sdl::SDL_FingerID(num(&spec, "finger")? as u64);
                e.tfinger.x = num(&spec, "x")? as f32;
                e.tfinger.y = num(&spec, "y")? as f32;
                e.tfinger.dx = num(&spec, "dx")? as f32;
                e.tfinger.dy = num(&spec, "dy")? as f32;
            }
            "drop_file" => {
                e.drop.r#type = T::DROP_FILE;
                e.drop.windowID = wid;
                e.drop.data = owned_text;
            }
            "window" => {
                // Any SDL_EVENT_WINDOW_* by its numeric type, with data1/data2.
                e.window.r#type = T(num(&spec, "etype")? as u32);
                e.window.windowID = wid;
                e.window.data1 = num(&spec, "data1")? as i32;
                e.window.data2 = num(&spec, "data2")? as i32;
            }
            "app" => {
                // Foreground / background lifecycle events, by numeric type.
                e.common.r#type = num(&spec, "etype")? as u32;
            }
            "other" => e.common.r#type = num(&spec, "etype")? as u32,
            "dialog_finished" => {
                // A forged `dialogfinished` event: what dialog_callback pushes when a dialog
                // ends, with the same owned payload.
                let code = num(&spec, "code")? as i32;
                let paths: Vec<mlua::LuaString> =
                    spec.raw_get::<Option<Vec<mlua::LuaString>>>("paths")?.unwrap_or_default();
                let message = text.as_bytes().to_vec();
                let payload = if code == 1 && paths.is_empty() && message.is_empty() {
                    None // DIALOG_CANCEL carries no data
                } else if code == 0 {
                    Some(Payload::Paths(paths.iter().map(|p| p.as_bytes().to_vec()).collect()))
                } else {
                    Some(Payload::Message(message)) // DIALOG_ERROR message
                };
                let event = CustomEvent { code, data1: num(&spec, "id")? as usize, payload };
                return Ok(push_custom_event(b"dialogfinished", event).is_ok());
            }
            other => return Err(LuaError::runtime(format!("unknown test event type '{other}'"))),
        }
        let input = crate::sysevents::injected_input(&e);
        // SDL's producer rejected window notifications without a registered window.
        let pushed = if kind == "window" && wid.0 == 0 { true } else { crate::platform::post(input) };
        Ok(pushed)
    }
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
        ("EVENT_WINDOW_SHOWN", sdl::SDL_EventType::WINDOW_SHOWN),
        ("EVENT_WINDOW_EXPOSED", sdl::SDL_EventType::WINDOW_EXPOSED),
        ("EVENT_WINDOW_RESIZED", sdl::SDL_EventType::WINDOW_RESIZED),
        ("EVENT_WINDOW_MINIMIZED", sdl::SDL_EventType::WINDOW_MINIMIZED),
        ("EVENT_WINDOW_MAXIMIZED", sdl::SDL_EventType::WINDOW_MAXIMIZED),
        ("EVENT_WINDOW_RESTORED", sdl::SDL_EventType::WINDOW_RESTORED),
        ("EVENT_WINDOW_MOUSE_LEAVE", sdl::SDL_EventType::WINDOW_MOUSE_LEAVE),
        ("EVENT_WINDOW_FOCUS_GAINED", sdl::SDL_EventType::WINDOW_FOCUS_GAINED),
        ("EVENT_WINDOW_FOCUS_LOST", sdl::SDL_EventType::WINDOW_FOCUS_LOST),
        ("EVENT_WINDOW_PIXEL_SIZE_CHANGED", sdl::SDL_EventType::WINDOW_PIXEL_SIZE_CHANGED),
        ("EVENT_WINDOW_DISPLAY_SCALE_CHANGED", sdl::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED),
        ("EVENT_WILL_ENTER_BACKGROUND", sdl::SDL_EventType::WILL_ENTER_BACKGROUND),
        ("EVENT_DID_ENTER_BACKGROUND", sdl::SDL_EventType::DID_ENTER_BACKGROUND),
        ("EVENT_WILL_ENTER_FOREGROUND", sdl::SDL_EventType::WILL_ENTER_FOREGROUND),
        ("EVENT_DID_ENTER_FOREGROUND", sdl::SDL_EventType::DID_ENTER_FOREGROUND),
        ("EVENT_USER", sdl::SDL_EventType::USER),
        ("EVENT_TERMINATING", sdl::SDL_EventType::TERMINATING),
    ] {
        hooks.raw_set(name, ty.0 as i64)?;
    }
    lua.globals().set("lxl_test", hooks)
}
