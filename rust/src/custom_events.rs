//! Custom SDL events, ported from `src/custom_events.c`.
//!
//! A custom event is a name plus an optional callback. Registering a name reserves one SDL
//! event type with `SDL_RegisterEvents`; `push_custom_event(name, ...)` pushes an event of that
//! type, and `poll_event` later asks `get_custom_event_callback_by_type` what Lua should see.
//! Worker threads (the dirmonitor watcher, a finished file dialog) push, the main thread
//! polls, so the registry sits behind a `Mutex`.
//!
//! The C keeps this in an SDL property bag, with `#<base36 type>` keys mapping a type back to
//! its name. That layout is an implementation detail with no reader outside the file, so this
//! uses two ordinary maps; the observable behaviour is identical (see `tests/contract/cases/
//! customevents.lua`).
//!
//! The C-visible functions keep their exact names and signatures, so the remaining C that
//! calls them (`main.c`, until W8) links unchanged.

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::Mutex;

use mlua::ffi::lua_State;
use sdl3_sys::everything as sdl;

/// `CustomEvent` in `custom_events.h`.
#[repr(C)]
pub struct CustomEvent {
    pub window_id: u32, // SDL_WindowID
    pub code: i32,
    pub data1: *mut c_void,
    pub data2: *mut c_void,
}

pub type CustomEventCallback = Option<unsafe extern "C" fn(*mut lua_State, *mut sdl::SDL_Event) -> c_int>;

#[derive(Clone, Copy)]
struct Entry {
    event_type: u32,
    callback: CustomEventCallback,
}

#[derive(Default)]
struct Registry {
    by_name: HashMap<Vec<u8>, Entry>,
    by_type: HashMap<u32, Vec<u8>>,
    /// False before `init_custom_events` and after `free_custom_events`.
    live: bool,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// The callback a registered event without one gets: it pushes nothing.
unsafe extern "C" fn noop(_: *mut lua_State, _: *mut sdl::SDL_Event) -> c_int {
    0
}

unsafe fn set_error(message: &CStr) {
    sdl::SDL_SetError(c"%s".as_ptr(), message.as_ptr());
}

/// `init_custom_events`.
#[no_mangle]
pub extern "C" fn init_custom_events() -> bool {
    *REGISTRY.lock().unwrap() = Some(Registry { live: true, ..Registry::default() });
    true
}

/// `free_custom_events`. Callers must not hold callbacks or names returned earlier.
#[no_mangle]
pub extern "C" fn free_custom_events() {
    *REGISTRY.lock().unwrap() = None;
}

/// `register_custom_event`. Registering an existing name replaces its callback and keeps its
/// type.
///
/// # Safety
/// `name` is NULL or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn register_custom_event(name: *const c_char, callback: CustomEventCallback) -> bool {
    if name.is_null() || *name == 0 {
        set_error(c"Invalid event name");
        return false;
    }
    let name = CStr::from_ptr(name).to_bytes();
    if name[0] == b'#' {
        set_error(c"Invalid event name, can't start with #");
        return false;
    }

    let mut guard = REGISTRY.lock().unwrap();
    // The C would crash here (NULL property bag); a clean refusal costs nothing.
    let Some(registry) = guard.as_mut().filter(|r| r.live) else {
        set_error(c"Custom events are not initialised");
        return false;
    };

    if let Some(entry) = registry.by_name.get_mut(name) {
        entry.callback = callback;
        return true;
    }
    let event_type = sdl::SDL_RegisterEvents(1);
    if event_type == 0 {
        return false;
    }
    registry.by_name.insert(name.to_vec(), Entry { event_type, callback });
    registry.by_type.insert(event_type, name.to_vec());
    true
}

/// `push_custom_event`: may be called from any thread.
///
/// # Safety
/// `name` is NULL or a NUL-terminated string; `event` points to a valid `CustomEvent`.
#[no_mangle]
pub unsafe extern "C" fn push_custom_event(name: *const c_char, event: *mut CustomEvent) -> bool {
    if name.is_null() || *name == 0 || *name as u8 == b'#' {
        set_error(c"Invalid event name");
        return false;
    }
    let bytes = CStr::from_ptr(name).to_bytes();
    let event_type = {
        let guard = REGISTRY.lock().unwrap();
        match guard.as_ref().and_then(|r| r.by_name.get(bytes)) {
            Some(entry) => entry.event_type,
            None => {
                drop(guard);
                let message = CString::new(format!("Unknown custom event {}", String::from_utf8_lossy(bytes))).unwrap();
                set_error(&message);
                return false;
            }
        }
    };

    let mut sdl_event: sdl::SDL_Event = std::mem::zeroed();
    sdl_event.user.r#type = event_type;
    sdl_event.user.windowID = sdl::SDL_WindowID((*event).window_id);
    sdl_event.user.code = (*event).code;
    sdl_event.user.data1 = (*event).data1;
    sdl_event.user.data2 = (*event).data2;
    sdl::SDL_PushEvent(&mut sdl_event)
}

fn callback_of(entry: &Entry) -> CustomEventCallback {
    // A registered event without a callback gets a no-op, never NULL.
    entry.callback.or(Some(noop))
}

/// `get_custom_event_callback_by_name`: NULL only when the event does not exist.
///
/// # Safety
/// `name` is NULL or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn get_custom_event_callback_by_name(name: *const c_char) -> CustomEventCallback {
    if name.is_null() || *name == 0 || *name as u8 == b'#' {
        set_error(c"Invalid event name");
        return None;
    }
    let guard = REGISTRY.lock().unwrap();
    guard
        .as_ref()
        .and_then(|r| r.by_name.get(CStr::from_ptr(name).to_bytes()))
        .and_then(callback_of)
}

/// `get_custom_event_callback_by_type`: NULL only when no event has this type.
#[no_mangle]
pub extern "C" fn get_custom_event_callback_by_type(event_type: u32) -> CustomEventCallback {
    let guard = REGISTRY.lock().unwrap();
    let registry = guard.as_ref()?;
    let name = registry.by_type.get(&event_type)?;
    registry.by_name.get(name).and_then(callback_of)
}

/// `get_custom_event_name`: a NUL-terminated name that stays valid until `free_custom_events`
/// (names are never removed, and a `Vec`'s heap buffer does not move when the map grows).
#[no_mangle]
pub extern "C" fn get_custom_event_name(event_type: u32) -> *const c_char {
    let guard = REGISTRY.lock().unwrap();
    let Some(registry) = guard.as_ref() else {
        return std::ptr::null();
    };
    match registry.by_type.get(&event_type) {
        // Stored without a terminator, so hand out a leaked, terminated copy once per type.
        Some(name) => leaked_name(event_type, name),
        None => std::ptr::null(),
    }
}

/// One terminated copy per event type, kept for the life of the process: the C API returns a
/// bare `const char *`, so the string has to outlive the call. There are a handful of event
/// types, registered once at startup.
fn leaked_name(event_type: u32, name: &[u8]) -> *const c_char {
    static NAMES: Mutex<Option<HashMap<u32, usize>>> = Mutex::new(None);
    let mut guard = NAMES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    let address = *map.entry(event_type).or_insert_with(|| {
        let owned = CString::new(name).unwrap_or_default();
        Box::leak(owned.into_boxed_c_str()).as_ptr() as usize
    });
    address as *const c_char
}

