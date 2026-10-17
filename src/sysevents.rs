//! `system.poll_event` and `system.wait_event`, ported from `src/api/system.c`.
//!
//! Both are raw `lua_CFunction`s rather than mlua callbacks: `poll_event` returns
//! 0 to 6 values depending on the event, and the custom-event callbacks it
//! dispatches to (`dialogfinished_callback`, in `sysplugin.rs`) push their results straight
//! onto the Lua stack. Nothing in here holds a Rust destructor across a call that
//! can raise a Lua error, so the `longjmp`s Lua uses are safe to cross.
//!
//! `RenWindow` stays opaque: scale and the `SDL_Window` come through the C
//! accessors added to `src/renwindow.c`.
//!
//! Deviations from the C, all on paths where it dereferences NULL or overflows:
//!   * a mouse or touch event for a window that is no longer registered is
//!     dropped, where the C reads `window_renderer->scale_x` through NULL;
//!   * key names are built as Rust strings, where the C `strcpy`s into a
//!     `char buf[16]` that several scancode names ("Keypad Memory Store") exceed.

use std::ffi::{c_char, c_int, c_void, CStr};

use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushnumber, lua_pushstring,
    luaL_checknumber,
};
use sdl3_sys::everything as sdl;

use crate::custom_events::get_custom_event_callback_by_type;

extern "C" {
    fn ren_find_window_from_id(id: sdl::SDL_WindowID) -> *mut c_void;
    fn ren_resize_window(window: *mut c_void);
    fn ren_get_scale_x(window: *mut c_void) -> f32;
    fn ren_get_scale_y(window: *mut c_void) -> f32;
    fn ren_get_sdl_window(window: *mut c_void) -> *mut sdl::SDL_Window;
    fn ren_refresh_after_foreground();
    fn rencache_invalidate();
}

const NUMPAD: [&str; 11] = [
    "end", "down", "pagedown", "left", "", "right", "home", "up", "pageup", "ins", "delete",
];

/// `button_name`.
fn button_name(button: u8) -> &'static CStr {
    match button as i32 {
        1 => c"left",
        2 => c"middle",
        3 => c"right",
        4 => c"x",
        5 => c"y",
        _ => c"?",
    }
}

/// `get_key_name`: layout-aware names for Latin keys and special keys, the
/// QWERTY scancode name otherwise; keypad keys become their navigation name
/// while NumLock is off. Always lower case (ASCII only, like `tolower` in "C").
fn key_name(key: &sdl::SDL_KeyboardEvent) -> String {
    let scancode = key.scancode.0;
    let kp1 = sdl::SDL_Scancode::KP_1.0;
    if scancode >= kp1 && scancode <= kp1 + 10 && key.r#mod.0 & sdl::SDL_Keymod::NUM.0 == 0 {
        return NUMPAD[(scancode - kp1) as usize].to_string();
    }
    let name = unsafe {
        if key.key.0 < 128 || key.key.0 & sdl::SDL_Keycode::SCANCODE_MASK.0 != 0 {
            CStr::from_ptr(sdl::SDL_GetKeyName(key.key))
        } else {
            CStr::from_ptr(sdl::SDL_GetScancodeName(key.scancode))
        }
    };
    name.to_string_lossy().to_ascii_lowercase()
}

unsafe fn push_str(l: *mut lua_State, s: &str) {
    // lua_pushstring wants a NUL-terminated copy; key names never contain NUL.
    let c = std::ffi::CString::new(s).unwrap_or_default();
    lua_pushstring(l, c.as_ptr());
}

unsafe fn push_cstr(l: *mut lua_State, s: *const c_char) {
    lua_pushstring(l, s);
}

/// Pushes `name` plus the values and returns how many were pushed in total.
macro_rules! event {
    ($l:expr, $name:literal $(, $push:expr)* $(,)?) => {{
        lua_pushstring($l, concat!($name, "\0").as_ptr() as *const c_char);
        #[allow(unused_mut)]
        let mut n: c_int = 1;
        $( { $push; n += 1; } )*
        return n;
    }};
}

/// `f_poll_event`.
pub unsafe extern "C-unwind" fn poll_event(l: *mut lua_State) -> c_int {
    let mut e: sdl::SDL_Event = std::mem::zeroed();

    // `top:` in the C: events that produce nothing loop to the next one.
    loop {
        if !sdl::SDL_PollEvent(&mut e) {
            return 0;
        }

        match sdl::SDL_EventType(e.r#type) {
            sdl::SDL_EventType::QUIT => event!(l, "quit"),

            sdl::SDL_EventType::WINDOW_RESIZED => {
                let window = ren_find_window_from_id(e.window.windowID);
                if !window.is_null() {
                    ren_resize_window(window);
                }
                // The size is in points.
                event!(
                    l,
                    "resized",
                    lua_pushinteger(l, e.window.data1 as i64),
                    lua_pushinteger(l, e.window.data2 as i64),
                )
            }

            sdl::SDL_EventType::WINDOW_EXPOSED => {
                rencache_invalidate();
                event!(l, "exposed")
            }
            sdl::SDL_EventType::WINDOW_MINIMIZED => event!(l, "minimized"),
            sdl::SDL_EventType::WINDOW_MAXIMIZED => event!(l, "maximized"),
            sdl::SDL_EventType::WINDOW_RESTORED => event!(l, "restored"),
            sdl::SDL_EventType::WINDOW_MOUSE_LEAVE => event!(l, "mouseleft"),
            sdl::SDL_EventType::WINDOW_FOCUS_LOST => event!(l, "focuslost"),

            sdl::SDL_EventType::WINDOW_FOCUS_GAINED => {
                // Alt-tabbing can queue several `tab` key presses; discard them.
                sdl::SDL_FlushEvent(sdl::SDL_EventType::KEY_DOWN.0);
                continue;
            }

            sdl::SDL_EventType::DROP_FILE => {
                let window = ren_find_window_from_id(e.drop.windowID);
                let (mut mx, mut my) = (0f32, 0f32);
                sdl::SDL_GetMouseState(&mut mx, &mut my);
                // A drag into the dock can arrive before a window exists.
                let (sx, sy) = if window.is_null() {
                    (0.0, 0.0)
                } else {
                    (ren_get_scale_x(window), ren_get_scale_y(window))
                };
                event!(
                    l,
                    "filedropped",
                    push_cstr(l, e.drop.data),
                    lua_pushinteger(l, (mx * sx) as i64),
                    lua_pushinteger(l, (my * sy) as i64),
                )
            }

            sdl::SDL_EventType::KEY_DOWN => {
                event!(l, "keypressed", push_str(l, &key_name(&e.key)))
            }
            sdl::SDL_EventType::KEY_UP => {
                event!(l, "keyreleased", push_str(l, &key_name(&e.key)))
            }

            sdl::SDL_EventType::TEXT_INPUT => event!(l, "textinput", push_cstr(l, e.text.text)),
            sdl::SDL_EventType::TEXT_EDITING => event!(
                l,
                "textediting",
                push_cstr(l, e.edit.text),
                lua_pushinteger(l, e.edit.start as i64),
                lua_pushinteger(l, e.edit.length as i64),
            ),

            sdl::SDL_EventType::MOUSE_BUTTON_DOWN => {
                if e.button.button == 1 {
                    sdl::SDL_CaptureMouse(true);
                }
                let window = ren_find_window_from_id(e.button.windowID);
                if window.is_null() {
                    continue;
                }
                let (sx, sy) = (ren_get_scale_x(window), ren_get_scale_y(window));
                event!(
                    l,
                    "mousepressed",
                    push_cstr(l, button_name(e.button.button).as_ptr()),
                    lua_pushinteger(l, (e.button.x * sx) as i64),
                    lua_pushinteger(l, (e.button.y * sy) as i64),
                    lua_pushinteger(l, e.button.clicks as i64),
                )
            }

            sdl::SDL_EventType::MOUSE_BUTTON_UP => {
                if e.button.button == 1 {
                    sdl::SDL_CaptureMouse(false);
                }
                let window = ren_find_window_from_id(e.button.windowID);
                if window.is_null() {
                    continue;
                }
                let (sx, sy) = (ren_get_scale_x(window), ren_get_scale_y(window));
                event!(
                    l,
                    "mousereleased",
                    push_cstr(l, button_name(e.button.button).as_ptr()),
                    lua_pushinteger(l, (e.button.x * sx) as i64),
                    lua_pushinteger(l, (e.button.y * sy) as i64),
                )
            }

            sdl::SDL_EventType::MOUSE_MOTION => {
                // Coalesce queued motion: keep the last position, sum the deltas.
                sdl::SDL_PumpEvents();
                let mut next: sdl::SDL_Event = std::mem::zeroed();
                while sdl::SDL_PeepEvents(
                    &mut next,
                    1,
                    sdl::SDL_EventAction::GETEVENT,
                    sdl::SDL_EventType::MOUSE_MOTION.0,
                    sdl::SDL_EventType::MOUSE_MOTION.0,
                ) > 0
                {
                    e.motion.x = next.motion.x;
                    e.motion.y = next.motion.y;
                    e.motion.xrel += next.motion.xrel;
                    e.motion.yrel += next.motion.yrel;
                }
                let window = ren_find_window_from_id(e.motion.windowID);
                if window.is_null() {
                    continue;
                }
                let (sx, sy) = (ren_get_scale_x(window), ren_get_scale_y(window));
                event!(
                    l,
                    "mousemoved",
                    lua_pushinteger(l, (e.motion.x * sx) as i64),
                    lua_pushinteger(l, (e.motion.y * sy) as i64),
                    lua_pushinteger(l, (e.motion.xrel * sx) as i64),
                    lua_pushinteger(l, (e.motion.yrel * sy) as i64),
                )
            }

            sdl::SDL_EventType::MOUSE_WHEEL => event!(
                l,
                "mousewheel",
                lua_pushnumber(l, e.wheel.y as f64),
                // -x keeps horizontal scrolling consistent with vertical (shift+scroll).
                lua_pushnumber(l, -(e.wheel.x as f64)),
            ),

            sdl::SDL_EventType::FINGER_DOWN | sdl::SDL_EventType::FINGER_UP => {
                let window = ren_find_window_from_id(e.tfinger.windowID);
                if window.is_null() {
                    continue;
                }
                let (mut w, mut h) = (0, 0);
                sdl::SDL_GetWindowSize(ren_get_sdl_window(window), &mut w, &mut h);
                let name: &CStr = if e.r#type == sdl::SDL_EventType::FINGER_DOWN.0 {
                    c"touchpressed"
                } else {
                    c"touchreleased"
                };
                lua_pushstring(l, name.as_ptr());
                lua_pushinteger(l, (e.tfinger.x * w as f32) as i64);
                lua_pushinteger(l, (e.tfinger.y * h as f32) as i64);
                lua_pushinteger(l, e.tfinger.fingerID.0 as i64);
                return 4;
            }

            sdl::SDL_EventType::FINGER_MOTION => {
                sdl::SDL_PumpEvents();
                let mut next: sdl::SDL_Event = std::mem::zeroed();
                while sdl::SDL_PeepEvents(
                    &mut next,
                    1,
                    sdl::SDL_EventAction::GETEVENT,
                    sdl::SDL_EventType::FINGER_MOTION.0,
                    sdl::SDL_EventType::FINGER_MOTION.0,
                ) > 0
                {
                    e.tfinger.x = next.tfinger.x;
                    e.tfinger.y = next.tfinger.y;
                    e.tfinger.dx += next.tfinger.dx;
                    e.tfinger.dy += next.tfinger.dy;
                }
                let window = ren_find_window_from_id(e.tfinger.windowID);
                if window.is_null() {
                    continue;
                }
                let (mut w, mut h) = (0, 0);
                sdl::SDL_GetWindowSize(ren_get_sdl_window(window), &mut w, &mut h);
                event!(
                    l,
                    "touchmoved",
                    lua_pushinteger(l, (e.tfinger.x * w as f32) as i64),
                    lua_pushinteger(l, (e.tfinger.y * h as f32) as i64),
                    lua_pushinteger(l, (e.tfinger.dx * w as f32) as i64),
                    lua_pushinteger(l, (e.tfinger.dy * h as f32) as i64),
                    lua_pushinteger(l, e.tfinger.fingerID.0 as i64),
                )
            }

            sdl::SDL_EventType::WILL_ENTER_FOREGROUND | sdl::SDL_EventType::DID_ENTER_FOREGROUND => {
                ren_refresh_after_foreground();
                let name: &CStr = if e.r#type == sdl::SDL_EventType::WILL_ENTER_FOREGROUND.0 {
                    c"enteringforeground"
                } else {
                    c"enteredforeground"
                };
                lua_pushstring(l, name.as_ptr());
                return 1;
            }
            sdl::SDL_EventType::WILL_ENTER_BACKGROUND => event!(l, "enteringbackground"),
            sdl::SDL_EventType::DID_ENTER_BACKGROUND => event!(l, "enteredbackground"),

            kind => {
                // The two events below resize the window, then fall into the default arm.
                if kind == sdl::SDL_EventType::WINDOW_DISPLAY_SCALE_CHANGED
                    || kind == sdl::SDL_EventType::WINDOW_PIXEL_SIZE_CHANGED
                {
                    let window = ren_find_window_from_id(e.window.windowID);
                    if !window.is_null() {
                        ren_resize_window(window);
                    }
                }
                // Custom event types start at SDL_EVENT_USER.
                if e.r#type >= sdl::SDL_EventType::USER.0 {
                    if let Some(callback) = get_custom_event_callback_by_type(e.r#type) {
                        let pushed = callback(l, &mut e);
                        // A callback that returned nothing means "next event".
                        if pushed != 0 {
                            return pushed;
                        }
                    }
                }
            }
        }
    }
}

/// `f_wait_event`: true if an event arrived, false on timeout.
pub unsafe extern "C-unwind" fn wait_event(l: *mut lua_State) -> c_int {
    if lua_gettop(l) >= 1 {
        let mut seconds = luaL_checknumber(l, 1);
        if seconds < 0.0 {
            seconds = 0.0;
        }
        lua_pushboolean(l, sdl::SDL_WaitEventTimeout(std::ptr::null_mut(), (seconds * 1000.0) as i32) as c_int);
    } else {
        lua_pushboolean(l, sdl::SDL_WaitEvent(std::ptr::null_mut()) as c_int);
    }
    1
}
