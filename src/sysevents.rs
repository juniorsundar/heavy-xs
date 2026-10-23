//! Lua event packing from the shared owned-input queue. Native callbacks never enter Lua.
use std::ffi::{c_char, c_int, CStr};
use std::time::Duration;
use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushlstring, lua_pushnumber,
    lua_pushstring, luaL_checknumber,
};
use crate::custom_events::get_custom_event_callback_by_type;
use crate::platform::{self, Custom, FingerKind, Input};

use crate::rencache::rencache_invalidate;
use crate::renwindow::{self, refresh_after_foreground};

pub fn post_custom(custom: Custom) -> bool { platform::post(Input::Custom(custom)) }

unsafe fn native_error(l: *mut lua_State, error: String) -> c_int {
    lua_pushlstring(l, error.as_ptr() as *const c_char, error.len());
    drop(error);
    mlua::ffi::lua_error(l)
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
    let s = crate::luaargs::nul_cut(s);
    lua_pushlstring(l, s.as_ptr() as *const c_char, s.len());
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

/// The point-to-pixel scale of the app-owned window with this native id, or `None` if there is
/// none. Read straight away: no window reference is held while values are pushed.
fn scale_of(id: u32) -> Option<(f32, f32)> {
    renwindow::find(id).map(|window| {
        let window = window.borrow();
        (window.scale_x(), window.scale_y())
    })
}

fn resize(id: u32) {
    if let Some(window) = renwindow::find(id) {
        window.borrow_mut().resize();
    }
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
                resize(window);
                // The size is in points.
                event!(l, "resized", lua_pushinteger(l, w as i64), lua_pushinteger(l, h as i64))
            }
            Input::Rescaled { window } => resize(window),

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
                // A drag into the dock can arrive before a window exists.
                let (sx, sy) = scale_of(window).unwrap_or((0.0, 0.0));
                let (mx, my) = if scale_of(window).is_some() { crate::native::pointer(window) } else { (0.0, 0.0) };
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
                let Some((sx, sy)) = scale_of(window) else {
                    continue;
                };
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
                let Some((sx, sy)) = scale_of(window) else {
                    continue;
                };
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
                if scale_of(window).is_none() {
                    continue;
                }
                let (w, h) = crate::native::logical_size(window);
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
                refresh_after_foreground();
                event!(l, "enteringforeground")
            }
            Input::EnteredForeground => {
                refresh_after_foreground();
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
