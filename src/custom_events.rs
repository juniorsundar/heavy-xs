//! Custom events, ported from `src/custom_events.c`.
//!
//! A custom event is a name plus an optional callback. Registering a name assigns it the next
//! type number; `push_custom_event(name, ...)` queues an event of that type with an owned
//! payload, and `poll_event` later asks `get_custom_event_callback_by_type` what Lua should see.
//! Worker threads (the dirmonitor watcher, a finished file dialog) push, the main thread
//! polls, so the registry sits behind a `Mutex`.
//!
//! Type numbers start at `FIRST_TYPE` (SDL's `SDL_EVENT_USER`, so they keep their historical
//! values) and are never reused. The C kept this in an SDL property bag; two ordinary maps give
//! the same observable behaviour (see `tests/contract/cases/customevents.lua`).

use std::collections::HashMap;
use std::sync::Mutex;

use mlua::ffi::lua_State;

use crate::platform::{Custom, Payload};

/// The first custom type number (`SDL_EVENT_USER`).
pub const FIRST_TYPE: u32 = 0x8000;
/// One past the last (`SDL_EVENT_LAST` is 0xFFFF).
const END_TYPE: u32 = 0x10000;

/// What a pusher hands over; the payload moves into the queued record.
pub struct CustomEvent {
    pub code: i32,
    pub data1: usize,
    pub payload: Option<Payload>,
}

/// Pushes this event's Lua values and returns how many; 0 means "no event, poll on".
pub type CustomEventCallback = Option<unsafe fn(*mut lua_State, &Custom) -> i32>;

#[derive(Clone, Copy)]
struct Entry {
    event_type: u32,
    callback: CustomEventCallback,
}

#[derive(Default)]
struct Registry {
    by_name: HashMap<Vec<u8>, Entry>,
    by_type: HashMap<u32, Vec<u8>>,
    next_type: u32,
    /// False before `init_custom_events` and after `free_custom_events`.
    live: bool,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// The callback a registered event without one gets: it pushes nothing.
unsafe fn noop(_: *mut lua_State, _: &Custom) -> i32 {
    0
}

/// `init_custom_events`. Type numbers continue across a Lua restart, as SDL's did: they are
/// process-wide and never reused.
pub fn init_custom_events() {
    let mut guard = REGISTRY.lock().unwrap();
    let next_type = guard.as_ref().map_or(FIRST_TYPE, |r| r.next_type);
    *guard = Some(Registry { live: true, next_type, ..Registry::default() });
}

/// `free_custom_events`: forget names and callbacks, keep the type counter.
pub fn free_custom_events() {
    if let Some(registry) = REGISTRY.lock().unwrap().as_mut() {
        registry.by_name.clear();
        registry.by_type.clear();
        registry.live = false;
    }
}

/// `register_custom_event`. Registering an existing name replaces its callback and keeps its
/// type. The error is the C's `SDL_SetError` text.
pub fn register_custom_event(name: &[u8], callback: CustomEventCallback) -> Result<(), String> {
    if name.is_empty() {
        return Err("Invalid event name".into());
    }
    if name[0] == b'#' {
        return Err("Invalid event name, can't start with #".into());
    }

    let mut guard = REGISTRY.lock().unwrap();
    // The C would crash here (NULL property bag); a clean refusal costs nothing.
    let Some(registry) = guard.as_mut().filter(|r| r.live) else {
        return Err("Custom events are not initialised".into());
    };

    if let Some(entry) = registry.by_name.get_mut(name) {
        entry.callback = callback;
        return Ok(());
    }
    if registry.next_type >= END_TYPE {
        // SDL_RegisterEvents returned 0 without an error message.
        return Err(String::new());
    }
    let event_type = registry.next_type;
    registry.next_type += 1;
    registry.by_name.insert(name.to_vec(), Entry { event_type, callback });
    registry.by_type.insert(event_type, name.to_vec());
    Ok(())
}

/// `push_custom_event`: may be called from any thread. On failure the event (and its payload)
/// is dropped here.
pub fn push_custom_event(name: &[u8], event: CustomEvent) -> Result<(), String> {
    if name.is_empty() || name[0] == b'#' {
        return Err("Invalid event name".into());
    }
    let event_type = REGISTRY
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|r| r.by_name.get(name))
        .map(|entry| entry.event_type)
        .ok_or_else(|| format!("Unknown custom event {}", String::from_utf8_lossy(name)))?;
    let custom = Custom { event_type, code: event.code, data1: event.data1, payload: event.payload };
    if crate::sysevents::post_custom(custom) {
        Ok(())
    } else {
        Err(crate::sysevents::sdl_error())
    }
}

fn callback_of(entry: &Entry) -> CustomEventCallback {
    // A registered event without a callback gets a no-op, never NULL.
    entry.callback.or(Some(noop))
}

/// `get_custom_event_callback_by_name`: `None` only when the event does not exist.
#[cfg(feature = "test_hooks")]
pub fn get_custom_event_callback_by_name(name: &[u8]) -> CustomEventCallback {
    if name.is_empty() || name[0] == b'#' {
        return None;
    }
    REGISTRY.lock().unwrap().as_ref().and_then(|r| r.by_name.get(name)).and_then(callback_of)
}

/// `get_custom_event_callback_by_type`: `None` only when no event has this type.
pub fn get_custom_event_callback_by_type(event_type: u32) -> CustomEventCallback {
    let guard = REGISTRY.lock().unwrap();
    let registry = guard.as_ref()?;
    let name = registry.by_type.get(&event_type)?;
    registry.by_name.get(name).and_then(callback_of)
}

/// `get_custom_event_name`, owned.
#[cfg(feature = "test_hooks")]
pub fn get_custom_event_name(event_type: u32) -> Option<Vec<u8>> {
    REGISTRY.lock().unwrap().as_ref()?.by_type.get(&event_type).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_numbers_refusals_and_restart() {
        init_custom_events();
        let a = |name: &[u8]| register_custom_event(name, None);
        assert_eq!(a(b""), Err("Invalid event name".into()));
        assert_eq!(a(b"#x"), Err("Invalid event name, can't start with #".into()));
        a(b"unit_one").unwrap();
        let one = REGISTRY.lock().unwrap().as_ref().unwrap().by_name[&b"unit_one"[..]].event_type;
        // Re-registering keeps the type; types are never reused, even across a Lua restart.
        a(b"unit_one").unwrap();
        free_custom_events();
        assert_eq!(a(b"late"), Err("Custom events are not initialised".into()));
        init_custom_events();
        a(b"unit_two").unwrap();
        let two = REGISTRY.lock().unwrap().as_ref().unwrap().by_name[&b"unit_two"[..]].event_type;
        assert_eq!(two, one + 1);
        assert!(get_custom_event_callback_by_type(two).is_some());
        assert!(get_custom_event_callback_by_type(one).is_none());
        let err = push_custom_event(b"never", CustomEvent { code: 0, data1: 0, payload: None });
        assert_eq!(err, Err("Unknown custom event never".into()));
    }
}
