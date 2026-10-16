//! Test-only: lets the contract suite inject synthetic SDL events.
//!
//! Compiled only with `-Dcore_features=test_hooks`; shipping builds have no
//! injection surface (a plugin could otherwise forge input). The C baseline for
//! the `sysevents` suite is recorded with the same feature, so both
//! implementations receive identical events.
//!
//! `lxl_test.push_event{ type = "key_down", ... }` pushes one event through
//! `SDL_PushEvent` and returns whether SDL accepted it.

use std::ffi::{c_void, CString};

use mlua::{AnyUserData, Error as LuaError, Lua, Result as LuaResult, Table, Value};
use sdl3_sys::everything as sdl;

use crate::custom_events::{
    get_custom_event_callback_by_name, get_custom_event_name, push_custom_event, register_custom_event,
    CustomEvent, CustomEventCallback,
};

extern "C" {
    fn ren_get_sdl_window(window: *mut c_void) -> *mut sdl::SDL_Window;
}

/// A callback that pushes ("custom", code, data1 as integer) so a test can see it ran.
unsafe extern "C" fn tracer(l: *mut mlua::lua_State, e: *mut sdl::SDL_Event) -> std::ffi::c_int {
    mlua::ffi::lua_pushstring(l, c"custom".as_ptr());
    mlua::ffi::lua_pushinteger(l, (*e).user.code as i64);
    mlua::ffi::lua_pushinteger(l, (*e).user.data1 as usize as i64);
    3
}

/// A callback that returns 0 values, which poll_event must treat as "next event".
unsafe extern "C" fn silent(_l: *mut mlua::lua_State, _e: *mut sdl::SDL_Event) -> std::ffi::c_int {
    0
}

fn custom_register(_: &Lua, (name, kind): (String, String)) -> LuaResult<bool> {
    let name = CString::new(name).unwrap_or_default();
    let callback: CustomEventCallback = match kind.as_str() {
        "tracer" => Some(tracer),
        "silent" => Some(silent),
        _ => None,
    };
    Ok(unsafe { register_custom_event(name.as_ptr(), callback) })
}

fn custom_push(_: &Lua, (name, code, data1): (String, i32, i64)) -> LuaResult<bool> {
    let name = CString::new(name).unwrap_or_default();
    let mut event = CustomEvent {
        window_id: 0,
        code,
        data1: data1 as usize as *mut c_void,
        data2: std::ptr::null_mut(),
    };
    Ok(unsafe { push_custom_event(name.as_ptr(), &mut event) })
}

/// The type number of a registered name (the first SDL user event is 0x8000), or nil.
fn custom_type(_: &Lua, name: String) -> LuaResult<Option<i64>> {
    // Walk the user range: the API only maps type -> name, so invert it.
    for ty in 0x8000u32..0x8040 {
        let found = get_custom_event_name(ty);
        if !found.is_null() && unsafe { std::ffi::CStr::from_ptr(found) }.to_bytes() == name.as_bytes() {
            return Ok(Some((ty - 0x8000) as i64));
        }
    }
    Ok(None)
}

fn custom_name(_: &Lua, offset: i64) -> LuaResult<Option<String>> {
    let found = get_custom_event_name(0x8000u32.wrapping_add(offset as u32));
    Ok(if found.is_null() {
        None
    } else {
        Some(unsafe { std::ffi::CStr::from_ptr(found) }.to_string_lossy().into_owned())
    })
}

fn custom_has_callback(_: &Lua, name: String) -> LuaResult<bool> {
    let name = CString::new(name).unwrap_or_default();
    Ok(unsafe { get_custom_event_callback_by_name(name.as_ptr()) }.is_some())
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
            Ok(unsafe { sdl::SDL_GetWindowID(ren_get_sdl_window(window)) })
        }
        _ => Ok(sdl::SDL_WindowID(0)),
    }
}

fn push_event(_: &Lua, spec: Table) -> LuaResult<bool> {
    use sdl::SDL_EventType as T;

    let kind: String = spec.raw_get("type")?;
    let mut e: sdl::SDL_Event = unsafe { std::mem::zeroed() };
    let text = CString::new(spec.raw_get::<Option<String>>("text")?.unwrap_or_default()).unwrap();
    let wid = window_id(&spec)?;
    // SDL queues a copy of the event but not the strings it points to. SDL's own
    // producers SDL_strdup them and SDL frees them in SDL_CleanupEvent, so do the
    // same instead of pointing at `text`, which is gone when this function returns.
    let owned_text = unsafe { sdl::SDL_strdup(text.as_ptr()) };

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
                // ends. `paths` is a Lua list; data2 is the "a\0b\0\0" block the callback
                // expects (SDL_malloc'd, because the callback SDL_free()s it).
                let code = num(&spec, "code")? as i32;
                let paths: Vec<String> = spec.raw_get::<Option<Vec<String>>>("paths")?.unwrap_or_default();
                let mut block: Vec<u8> = Vec::new();
                for p in &paths {
                    block.extend_from_slice(p.as_bytes());
                    block.push(0);
                }
                block.push(0);
                let heap = if code == 1 && paths.is_empty() && text.as_bytes().is_empty() {
                    std::ptr::null_mut() // DIALOG_CANCEL carries no data
                } else if code == 0 {
                    let m = sdl::SDL_malloc(block.len()) as *mut u8;
                    std::ptr::copy_nonoverlapping(block.as_ptr(), m, block.len());
                    m as *mut c_void
                } else {
                    sdl::SDL_strdup(text.as_ptr()) as *mut c_void // DIALOG_ERROR message
                };
                e.user.r#type = sdl::SDL_EventType::USER.0; // "dialogfinished" is registered first
                e.user.code = code;
                e.user.data1 = num(&spec, "id")? as usize as *mut c_void;
                e.user.data2 = heap;
                let pushed = sdl::SDL_PushEvent(&mut e);
                if !pushed {
                    sdl::SDL_free(heap);
                }
                sdl::SDL_free(owned_text as *mut c_void);
                return Ok(pushed);
            }
            other => return Err(LuaError::runtime(format!("unknown test event type '{other}'"))),
        }
        let pushed = sdl::SDL_PushEvent(&mut e);
        // Only the three text-carrying kinds hand ownership of the copy to SDL.
        if !pushed || !matches!(kind.as_str(), "text_input" | "text_editing" | "drop_file") {
            sdl::SDL_free(owned_text as *mut c_void);
        }
        Ok(pushed)
    }
}

/// Installs the global `lxl_test` table.
pub fn install(lua: &Lua) -> LuaResult<()> {
    let hooks = lua.create_table()?;
    hooks.raw_set("push_event", lua.create_function(push_event)?)?;
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
