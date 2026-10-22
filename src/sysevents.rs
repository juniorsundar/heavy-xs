//! `system.poll_event` and `system.wait_event`, ported from `src/api/system.c`.
//!
//! Lua only ever sees owned [`Input`] records. `next_input` is the native producer: it takes one
//! SDL event and converts it into a record, after which no SDL memory is referenced. Records are
//! taken from SDL one per poll, so SDL keeps its queue and its poll-cycle and wait semantics;
//! the owned queue only holds what coalescing pulled forward. Custom events pushed from other
//! threads travel through SDL's queue as a plain integer token naming a Rust-owned record in
//! `PENDING`, so they keep their order relative to native events and still wake `SDL_WaitEvent`.
//!
//! Both are raw `lua_CFunction`s rather than mlua callbacks: `poll_event` returns 0 to 6 values
//! depending on the event, and custom-event callbacks push straight onto the Lua stack. Nothing
//! here holds a Rust destructor across a call that can raise a Lua error.
//!
//! Deviations from the C, all on paths where it dereferences NULL or overflows:
//!   * a mouse or touch event for a window that is no longer registered is dropped, where the C
//!     reads `window_renderer->scale_x` through NULL;
//!   * key names are built as Rust strings (see `keynames.rs`).

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::Mutex;

use mlua::ffi::{
    lua_State, lua_gettop, lua_pushboolean, lua_pushinteger, lua_pushlstring, lua_pushnumber,
    lua_pushstring, luaL_checknumber,
};
use sdl3_sys::everything as sdl;

use crate::custom_events::{get_custom_event_callback_by_type, FIRST_TYPE};
use crate::platform::{self, Custom, FingerKind, Input};

extern "C" {
    fn ren_find_window_from_id(id: sdl::SDL_WindowID) -> *mut c_void;
    fn ren_resize_window(window: *mut c_void);
    fn ren_get_scale_x(window: *mut c_void) -> f32;
    fn ren_get_scale_y(window: *mut c_void) -> f32;
    fn ren_get_sdl_window(window: *mut c_void) -> *mut sdl::SDL_Window;
    fn ren_refresh_after_foreground();
    fn rencache_invalidate();
}

// ------------------------------------------------------------- custom-event transport

/// Custom records waiting in SDL's queue, by token. A token is removed exactly once: by
/// `convert` when SDL hands it back, or by `post_custom` when SDL refused it.
static PENDING: Mutex<Option<(u64, HashMap<u64, Custom>)>> = Mutex::new(None);

/// Queues a custom record from any thread. False (record dropped) when SDL's queue refused it.
pub fn post_custom(custom: Custom) -> bool {
    let event_type = custom.event_type;
    let token = {
        let mut guard = PENDING.lock().unwrap();
        let (next, map) = guard.get_or_insert_with(|| (1, HashMap::new()));
        let token = *next;
        *next += 1;
        map.insert(token, custom);
        token
    };
    let mut e: sdl::SDL_Event = unsafe { std::mem::zeroed() };
    e.user.r#type = event_type;
    e.user.data1 = token as usize as *mut c_void;
    if unsafe { sdl::SDL_PushEvent(&mut e) } {
        return true;
    }
    take_pending(token);
    false
}

fn take_pending(token: u64) -> Option<Custom> {
    PENDING.lock().unwrap().as_mut()?.1.remove(&token)
}

pub fn sdl_error() -> String {
    let message = sdl::SDL_GetError();
    if message.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned()
}

// --------------------------------------------------------------- the SDL producer

fn bytes(text: *const c_char) -> Vec<u8> {
    if text.is_null() {
        return Vec::new();
    }
    unsafe { CStr::from_ptr(text) }.to_bytes().to_vec()
}

/// One SDL event as an owned record.
unsafe fn convert(e: &sdl::SDL_Event) -> Input {
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
            // Ours carry a token; anything else in the user range (a forged or foreign event)
            // is an empty custom record of that type, as SDL delivered it.
            let token = e.user.data1 as usize as u64;
            take_pending(token).filter(|c| c.event_type == e.r#type).unwrap_or(Custom {
                event_type: e.r#type,
                code: e.user.code,
                data1: e.user.data1 as usize,
                payload: None,
            })
            .into()
        }
        _ => Input::Other,
    }
}

impl From<Custom> for Input {
    fn from(custom: Custom) -> Self {
        Input::Custom(custom)
    }
}

/// The next record: one already pulled forward, else one SDL event (SDL frees its strings).
fn next_input() -> Option<Input> {
    platform::pop().or_else(|| unsafe {
        let mut e: sdl::SDL_Event = std::mem::zeroed();
        sdl::SDL_PollEvent(&mut e).then(|| convert(&e))
    })
}

/// Pulls every SDL event of `kind` forward into the owned queue in order, leaving the rest in
/// SDL's queue (`SDL_PumpEvents` then `SDL_PeepEvents`).
fn pull_forward(kind: sdl::SDL_EventType) {
    unsafe {
        sdl::SDL_PumpEvents();
        let mut e: sdl::SDL_Event = std::mem::zeroed();
        while sdl::SDL_PeepEvents(&mut e, 1, sdl::SDL_EventAction::GETEVENT, kind.0, kind.0) > 0 {
            platform::push(convert(&e));
        }
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
    ren_find_window_from_id(sdl::SDL_WindowID(id))
}

/// `f_poll_event`.
pub unsafe extern "C-unwind" fn poll_event(l: *mut lua_State) -> c_int {
    // `top:` in the C: events that produce nothing loop to the next one.
    loop {
        let Some(mut input) = next_input() else {
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
                sdl::SDL_FlushEvent(sdl::SDL_EventType::KEY_DOWN.0);
                platform::remove_all(|i| matches!(i, Input::Key { down: true, .. }));
            }

            Input::FileDropped { window, ref path } => {
                let window = window_of(window);
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
                if button == 1 {
                    sdl::SDL_CaptureMouse(down);
                }
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
                pull_forward(sdl::SDL_EventType::MOUSE_MOTION);
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
                    pull_forward(sdl::SDL_EventType::FINGER_MOTION);
                    platform::coalesce(&mut input);
                }
                let Input::Finger { window, x, y, dx, dy, finger, .. } = input else { unreachable!() };
                let window = window_of(window);
                if window.is_null() {
                    continue;
                }
                let (mut w, mut h) = (0, 0);
                sdl::SDL_GetWindowSize(ren_get_sdl_window(window), &mut w, &mut h);
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
    // Only coalescing pulls records forward, and it consumes them at once, so this is SDL's
    // own readiness.
    let ready = !platform::is_empty()
        || match timeout {
            Some(ms) => sdl::SDL_WaitEventTimeout(std::ptr::null_mut(), ms),
            None => sdl::SDL_WaitEvent(std::ptr::null_mut()),
        };
    lua_pushboolean(l, ready as c_int);
    1
}
