//! Lua event packing from the shared owned-input queue. Native callbacks never enter Lua.
use std::ffi::{c_char, c_int, c_void, CStr};
use std::time::Duration;
use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushlstring, lua_pushnumber,
    lua_pushstring, luaL_checknumber,
};
#[cfg(feature = "test_hooks")]
use sdl3_sys::everything as sdl;
use crate::custom_events::get_custom_event_callback_by_type;
#[cfg(feature = "test_hooks")]
use crate::custom_events::FIRST_TYPE;
use crate::platform::{self, Custom, FingerKind, Input};

extern "C" {
    fn ren_find_window_from_id(id: u32) -> *mut c_void;
    fn ren_resize_window(window: *mut c_void);
    fn ren_get_scale_x(window: *mut c_void) -> f32;
    fn ren_get_scale_y(window: *mut c_void) -> f32;
    fn ren_get_window_id(window: *mut c_void) -> u32;
    fn ren_refresh_after_foreground();
    fn rencache_invalidate();
}

pub fn post_custom(custom: Custom) -> bool { platform::post(Input::Custom(custom)) }

unsafe fn native_error(l: *mut lua_State, error: String) -> c_int {
    lua_pushlstring(l, error.as_ptr() as *const c_char, error.len());
    drop(error);
    mlua::ffi::lua_error(l)
}

// Historical numeric injection fixtures are decoded only in hook-enabled builds. No SDL pump.
#[cfg(feature = "test_hooks")]
fn bytes(text: *const c_char) -> Vec<u8> {
    if text.is_null() {
        return Vec::new();
    }
    unsafe { CStr::from_ptr(text) }.to_bytes().to_vec()
}

/// One SDL event as an owned record.
#[cfg(feature = "test_hooks")]
pub(crate) unsafe fn injected_input(e: &sdl::SDL_Event) -> Input {
    use sdl::SDL_EventType as T;
    let kind = T(e.r#type);
    match kind {
        T::QUIT => Input::Quit,
        T::WINDOW_RESIZED => {
            Input::Resized { window: e.window.windowID.0, w: e.window.data1, h: e.window.data2 }
        }
        T::WINDOW_DISPLAY_SCALE_CHANGED | T::WINDOW_PIXEL_SIZE_CHANGED => {
            Input::Rescaled { window: e.window.windowID.0 }
        }
        T::WINDOW_EXPOSED => Input::Exposed,
        T::WINDOW_MINIMIZED => Input::Minimized,
        T::WINDOW_MAXIMIZED => Input::Maximized,
        T::WINDOW_RESTORED => Input::Restored,
        T::WINDOW_MOUSE_LEAVE => Input::MouseLeft,
        T::WINDOW_FOCUS_LOST => Input::FocusLost,
        T::WINDOW_FOCUS_GAINED => Input::FocusGained,
        T::DROP_FILE => Input::FileDropped { window: e.drop.windowID.0, path: bytes(e.drop.data) },
        T::KEY_DOWN | T::KEY_UP => Input::Key {
            down: kind == T::KEY_DOWN,
            scancode: e.key.scancode.0,
            key: e.key.key.0,
            numlock: e.key.r#mod.0 & sdl::SDL_Keymod::NUM.0 != 0,
        },
        T::TEXT_INPUT => Input::TextInput(bytes(e.text.text)),
        T::TEXT_EDITING => Input::TextEditing {
            text: bytes(e.edit.text),
            start: e.edit.start,
            length: e.edit.length,
        },
        T::MOUSE_BUTTON_DOWN | T::MOUSE_BUTTON_UP => Input::MouseButton {
            window: e.button.windowID.0,
            down: kind == T::MOUSE_BUTTON_DOWN,
            button: e.button.button,
            x: e.button.x,
            y: e.button.y,
            clicks: e.button.clicks,
        },
        T::MOUSE_MOTION => Input::MouseMotion {
            window: e.motion.windowID.0,
            x: e.motion.x,
            y: e.motion.y,
            xrel: e.motion.xrel,
            yrel: e.motion.yrel,
        },
        T::MOUSE_WHEEL => Input::MouseWheel { x: e.wheel.x, y: e.wheel.y },
        T::FINGER_DOWN | T::FINGER_UP | T::FINGER_MOTION => Input::Finger {
            window: e.tfinger.windowID.0,
            kind: match kind {
                T::FINGER_DOWN => FingerKind::Down,
                T::FINGER_UP => FingerKind::Up,
                _ => FingerKind::Motion,
            },
            x: e.tfinger.x,
            y: e.tfinger.y,
            dx: e.tfinger.dx,
            dy: e.tfinger.dy,
            finger: e.tfinger.fingerID.0,
        },
        T::WILL_ENTER_FOREGROUND => Input::EnteringForeground,
        T::DID_ENTER_FOREGROUND => Input::EnteredForeground,
        T::WILL_ENTER_BACKGROUND => Input::EnteringBackground,
        T::DID_ENTER_BACKGROUND => Input::EnteredBackground,
        _ if e.r#type >= FIRST_TYPE => {
            Input::Custom(Custom {
                event_type: e.r#type, code: e.user.code,
                data1: e.user.data1 as usize, payload: None,
            })
        }
        _ => Input::Other,
    }
}


// --------------------------------------------------------------- the Lua side

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

unsafe fn push_bytes(l: *mut lua_State, s: &[u8]) {
    // As lua_pushstring of the C string: an embedded NUL ends it.
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    lua_pushlstring(l, s.as_ptr() as *const c_char, end);
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

unsafe fn window_of(id: u32) -> *mut c_void {
    ren_find_window_from_id(id)
}

/// `f_poll_event`.
pub unsafe extern "C-unwind" fn poll_event(l: *mut lua_State) -> c_int {
    if crate::native::initialized() {
        if let Err(error) = crate::native::pump(Some(Duration::ZERO)) { return native_error(l, error); }
    }
    if let Some(error) = platform::take_error() { return native_error(l, error); }
    // `top:` in the C: events that produce nothing loop to the next one.
    loop {
        let Some(mut input) = platform::pop() else {
            return 0;
        };

        match input {
            Input::Quit => event!(l, "quit"),

            Input::Resized { window, w, h } => {
                let window = window_of(window);
                if !window.is_null() {
                    ren_resize_window(window);
                }
                // The size is in points.
                event!(l, "resized", lua_pushinteger(l, w as i64), lua_pushinteger(l, h as i64))
            }
            Input::Rescaled { window } => {
                let window = window_of(window);
                if !window.is_null() {
                    ren_resize_window(window);
                }
            }

            Input::Exposed => {
                rencache_invalidate();
                event!(l, "exposed")
            }
            Input::Minimized => event!(l, "minimized"),
            Input::Maximized => event!(l, "maximized"),
            Input::Restored => event!(l, "restored"),
            Input::MouseLeft => event!(l, "mouseleft"),
            Input::FocusLost => event!(l, "focuslost"),

            Input::FocusGained => {
                // Alt-tabbing can queue several `tab` key presses; discard them.
                platform::remove_all(|i| matches!(i, Input::Key { down: true, .. }));
            }

            Input::FileDropped { window, ref path } => {
                let window = window_of(window);
                let (mx, my) = if window.is_null() { (0.0, 0.0) } else { crate::native::pointer(ren_get_window_id(window)) };
                // A drag into the dock can arrive before a window exists.
                let (sx, sy) = if window.is_null() {
                    (0.0, 0.0)
                } else {
                    (ren_get_scale_x(window), ren_get_scale_y(window))
                };
                event!(
                    l,
                    "filedropped",
                    push_bytes(l, path),
                    lua_pushinteger(l, (mx * sx) as i64),
                    lua_pushinteger(l, (my * sy) as i64),
                )
            }

            Input::Key { down, scancode, key, numlock } => {
                let name = crate::keynames::key_name(scancode, key, numlock);
                if down {
                    event!(l, "keypressed", push_bytes(l, name.as_bytes()))
                }
                event!(l, "keyreleased", push_bytes(l, name.as_bytes()))
            }

            Input::TextInput(ref text) => event!(l, "textinput", push_bytes(l, text)),
            Input::TextEditing { ref text, start, length } => event!(
                l,
                "textediting",
                push_bytes(l, text),
                lua_pushinteger(l, start as i64),
                lua_pushinteger(l, length as i64),
            ),

            Input::MouseButton { window, down, button, x, y, clicks } => {
                let window = window_of(window);
                if window.is_null() {
                    continue;
                }
                let (sx, sy) = (ren_get_scale_x(window), ren_get_scale_y(window));
                let name = button_name(button).as_ptr();
                if down {
                    event!(
                        l,
                        "mousepressed",
                        lua_pushstring(l, name),
                        lua_pushinteger(l, (x * sx) as i64),
                        lua_pushinteger(l, (y * sy) as i64),
                        lua_pushinteger(l, clicks as i64),
                    )
                }
                event!(
                    l,
                    "mousereleased",
                    lua_pushstring(l, name),
                    lua_pushinteger(l, (x * sx) as i64),
                    lua_pushinteger(l, (y * sy) as i64),
                )
            }

            Input::MouseMotion { .. } => {
                // Coalesce queued motion: keep the last position, sum the deltas.
                platform::coalesce(&mut input);
                let Input::MouseMotion { window, x, y, xrel, yrel } = input else { unreachable!() };
                let window = window_of(window);
                if window.is_null() {
                    continue;
                }
                let (sx, sy) = (ren_get_scale_x(window), ren_get_scale_y(window));
                event!(
                    l,
                    "mousemoved",
                    lua_pushinteger(l, (x * sx) as i64),
                    lua_pushinteger(l, (y * sy) as i64),
                    lua_pushinteger(l, (xrel * sx) as i64),
                    lua_pushinteger(l, (yrel * sy) as i64),
                )
            }

            Input::MouseWheel { x, y } => event!(
                l,
                "mousewheel",
                lua_pushnumber(l, y as f64),
                // -x keeps horizontal scrolling consistent with vertical (shift+scroll).
                lua_pushnumber(l, -(x as f64)),
            ),

            Input::Finger { kind, .. } => {
                if kind == FingerKind::Motion {
                    platform::coalesce(&mut input);
                }
                let Input::Finger { window, x, y, dx, dy, finger, .. } = input else { unreachable!() };
                let window = window_of(window);
                if window.is_null() {
                    continue;
                }
                let (w, h) = crate::native::logical_size(ren_get_window_id(window));
                let (w, h) = (w as f32, h as f32);
                match kind {
                    FingerKind::Down | FingerKind::Up => {
                        let name: &CStr =
                            if kind == FingerKind::Down { c"touchpressed" } else { c"touchreleased" };
                        lua_pushstring(l, name.as_ptr());
                        lua_pushinteger(l, (x * w) as i64);
                        lua_pushinteger(l, (y * h) as i64);
                        lua_pushinteger(l, finger as i64);
                        return 4;
                    }
                    FingerKind::Motion => event!(
                        l,
                        "touchmoved",
                        lua_pushinteger(l, (x * w) as i64),
                        lua_pushinteger(l, (y * h) as i64),
                        lua_pushinteger(l, (dx * w) as i64),
                        lua_pushinteger(l, (dy * h) as i64),
                        lua_pushinteger(l, finger as i64),
                    ),
                }
            }

            Input::EnteringForeground => {
                ren_refresh_after_foreground();
                event!(l, "enteringforeground")
            }
            Input::EnteredForeground => {
                ren_refresh_after_foreground();
                event!(l, "enteredforeground")
            }
            Input::EnteringBackground => event!(l, "enteringbackground"),
            Input::EnteredBackground => event!(l, "enteredbackground"),

            Input::Custom(ref custom) => {
                if let Some(callback) = get_custom_event_callback_by_type(custom.event_type) {
                    let pushed = callback(l, custom);
                    // A callback that returned nothing means "next event".
                    if pushed != 0 {
                        return pushed;
                    }
                }
            }

            Input::Other => {}
        }
    }
}

/// `f_wait_event`: true if an event arrived, false on timeout. Readiness only: the event stays
/// queued for `poll_event`.
pub unsafe extern "C-unwind" fn wait_event(l: *mut lua_State) -> c_int {
    let timeout = if lua_gettop(l) >= 1 {
        let seconds = luaL_checknumber(l, 1);
        Some((seconds.max(0.0) * 1000.0) as i32)
    } else {
        None
    };
    let ready = match platform::wait(timeout.map(|ms| Duration::from_millis(ms.max(0) as u64))) {
        Ok(ready) => ready,
        Err(error) => return native_error(l, error),
    };
    if let Some(error) = platform::take_error() { return native_error(l, error); }
    lua_pushboolean(l, ready as c_int);
    1
}
