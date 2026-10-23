//! `dirmonitor`: file-change notification, ported from `src/api/dirmonitor.c`
//! plus the Linux inotify backend `src/api/dirmonitor/inotify.c`.
//!
//! One userdata owns an inotify instance, a self-pipe and, once the first watch
//! exists, a worker thread. The thread blocks in `poll`, stores the raw inotify
//! bytes for `check` to translate, and wakes the SDL event loop by pushing the
//! "dirmonitor" custom event (see `custom_events.rs`).
//! The thread never touches Lua.
//!
//! Linux only (the other platforms' backends were deleted in W10).
//!
//! Deviations from the C:
//!   * the C walks inotify events in 16-byte steps instead of 16 + `len`, so a
//!     file name makes it report garbage watch ids made of name bytes;
//!     `data/core/dirwatch.lua` ignores unknown ids, and this port walks the
//!     events correctly;
//!   * shutdown wakes the worker through the self-pipe, joins it, and only then
//!     closes the fds, where the C closes them under the worker's feet;
//!   * `check` runs the Lua callbacks after releasing the mutex, so a callback
//!     may call back into the monitor.

use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::fd::OwnedFd;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::custom_events::{push_custom_event, register_custom_event, CustomEvent};
use mlua::{
    Error as LuaError, Function, Lua, MultiValue, Result as LuaResult, Table, UserData,
    UserDataMethods, Value, Variadic,
};

/// `char buffer[64512]` in the C struct.
const BUFFER_SIZE: usize = 64512;
const API_TYPE_DIRMONITOR: &str = "Dirmonitor";
const INOTIFY_HEADER: usize = 16; // sizeof(struct inotify_event) without the name

// ------------------------------------------------------------ custom events

fn wake_event_loop() {
    let _ = push_custom_event(b"dirmonitor", CustomEvent { code: 0, data1: 0, payload: None });
}

// ------------------------------------------------------------ the backend

/// Everything the worker needs. Immutable once created. The fds close when the last
/// reference goes, which is after the worker has been joined (`deinit_dirmonitor`).
struct Backend {
    /// `None` if `inotify_init1` failed: every watch then fails, as on the C's -1 fd.
    inotify: Option<OwnedFd>,
    /// Self-pipe: a byte written to the write end wakes a worker blocked in `poll`.
    sig: Option<(OwnedFd, OwnedFd)>,
}

impl Backend {
    /// `init_dirmonitor`.
    fn new() -> Backend {
        use rustix::fs::inotify::{init, CreateFlags};
        use rustix::pipe::{pipe_with, PipeFlags};
        Backend {
            inotify: init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC).ok(),
            sig: pipe_with(PipeFlags::CLOEXEC).ok(),
        }
    }

    /// `add_dirmonitor`: a watch id, or -1.
    fn add(&self, path: &CStr) -> i32 {
        use rustix::fs::inotify::{add_watch, WatchFlags};
        let mask = WatchFlags::CREATE | WatchFlags::DELETE | WatchFlags::MOVED_FROM | WatchFlags::MODIFY | WatchFlags::MOVED_TO;
        self.inotify.as_ref().and_then(|fd| add_watch(fd, path, mask).ok()).unwrap_or(-1)
    }

    /// `remove_dirmonitor`.
    fn remove(&self, id: i32) {
        if let Some(fd) = &self.inotify {
            let _ = rustix::fs::inotify::remove_watch(fd, id);
        }
    }

    /// `get_changes_dirmonitor`: blocks until inotify has data (returns its
    /// length), the monitor is being shut down, or reading fails (both -1).
    fn get_changes(&self, buffer: &mut [u8]) -> i32 {
        use rustix::event::{poll, PollFd, PollFlags};
        use rustix::io::Errno;
        let events = PollFlags::IN | PollFlags::ERR;
        loop {
            // poll(2) skips the C's -1 fds; leaving a missing one out is the same.
            let mut fds: Vec<PollFd> = Vec::with_capacity(2);
            if let Some((reader, _)) = &self.sig {
                fds.push(PollFd::new(reader, events));
            }
            if let Some(inotify) = &self.inotify {
                fds.push(PollFd::new(inotify, events));
            }
            match poll(&mut fds, None) {
                Ok(_) => {}
                Err(Errno::INTR) => continue,
                Err(_) => return -1,
            }
            if self.sig.is_some() && !fds[0].revents().is_empty() {
                return -1;
            }
            drop(fds);
            let Some(inotify) = &self.inotify else { return -1 };
            match rustix::io::read(inotify, &mut *buffer) {
                Ok(n) => return n as i32,
                Err(Errno::AGAIN) | Err(Errno::INTR) => continue,
                Err(_) => return -1,
            }
        }
    }

    /// Wake a worker blocked in `poll` (see `get_changes`).
    fn wake(&self) {
        if let Some((_, writer)) = &self.sig {
            let _ = rustix::io::write(writer, &[1u8]);
        }
    }
}

/// `translate_changes_dirmonitor`: the watch id of every event except bare
/// `IN_IGNORED`. Walks `struct inotify_event` by its real size.
fn translate_changes(buffer: &[u8]) -> Vec<i32> {
    let mut ids = Vec::new();
    let mut at = 0;
    while at + INOTIFY_HEADER <= buffer.len() {
        let wd = i32::from_ne_bytes(buffer[at..at + 4].try_into().unwrap());
        let mask = u32::from_ne_bytes(buffer[at + 4..at + 8].try_into().unwrap());
        let name_len = u32::from_ne_bytes(buffer[at + 12..at + 16].try_into().unwrap()) as usize;
        if mask & !libc::IN_IGNORED > 0 {
            ids.push(wd);
        }
        at += INOTIFY_HEADER + name_len;
    }
    ids
}

// -------------------------------------------------------------- the monitor

/// `volatile int length` and the buffer it describes. `length` is 0 when empty,
/// positive when `buffer` holds that many unread bytes, negative once shut down.
struct State {
    length: i32,
    buffer: Vec<u8>,
}

struct Shared {
    state: Mutex<State>,
    backend: Backend,
}

struct DirMonitor {
    shared: Arc<Shared>,
    /// The worker, started by the first `watch`. A `Mutex` rather than `&mut self`: `watch` must
    /// stay a shared-borrow method, because `dirwatch.lua` calls `unwatch`/`watch` on the monitor
    /// from inside the `check` callback, and an exclusive borrow would fail while `check` holds
    /// its shared one ("error mutably borrowing userdata").
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// `dirmonitor_check_thread`.
fn check_thread(shared: Arc<Shared>) {
    let mut local = vec![0u8; BUFFER_SIZE];
    loop {
        let pending = {
            let state = shared.state.lock().unwrap();
            if state.length < 0 {
                return;
            }
            state.length == 0
        };
        if pending {
            let result = shared.backend.get_changes(&mut local);
            let mut state = shared.state.lock().unwrap();
            if state.length == 0 {
                state.length = result;
                if result > 0 {
                    state.buffer.clear();
                    state.buffer.extend_from_slice(&local[..result as usize]);
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
        wake_event_loop();
    }
}

impl DirMonitor {
    fn new() -> DirMonitor {
        DirMonitor {
            shared: Arc::new(Shared {
                state: Mutex::new(State { length: 0, buffer: Vec::new() }),
                backend: Backend::new(),
            }),
            thread: Mutex::new(None),
        }
    }
}

impl Drop for DirMonitor {
    /// `f_dirmonitor_gc`: mark the monitor dead, wake the worker, join it, and
    /// only then release the fds it was using.
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().length = -1;
        self.shared.backend.wake();
        if let Some(thread) = self.thread.get_mut().unwrap().take() {
            let _ = thread.join();
        }
        // The worker held the only other reference and has exited, so dropping `shared`
        // with `self` closes the fds.
    }
}

// --------------------------------------------------------------- Lua side

fn argerror(fname: &str, arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!(
        "bad argument #{} to '{}' ({} expected, got {})",
        arg, fname, expected, got
    ))
}

/// The message the C's `lua_pcall` message handler would have been given.
fn error_text(error: &LuaError) -> String {
    match error {
        LuaError::RuntimeError(message) => message.clone(),
        LuaError::CallbackError { cause, .. } => error_text(cause),
        other => other.to_string(),
    }
}

impl UserData for DirMonitor {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__name", API_TYPE_DIRMONITOR);
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("watch", |lua, this, args: Variadic<Value>| {
            let path = match args.first() {
                Some(v) => lua
                    .coerce_string(v.clone())?
                    .ok_or_else(|| argerror("watch", 2, "string", v.type_name()))?,
                None => return Err(argerror("watch", 2, "string", "no value")),
            };
            let bytes = path.as_bytes();
            let bytes: &[u8] = &bytes;
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let path = CString::new(&bytes[..end]).unwrap();

            let id = this.shared.backend.add(&path);

            // The worker starts on the first watch, whether or not it succeeded.
            let mut thread = this.thread.lock().unwrap();
            if thread.is_none() {
                let shared = Arc::clone(&this.shared);
                *thread = thread::Builder::new()
                    .name("dirmonitor_check_thread".into())
                    .spawn(move || check_thread(shared))
                    .ok();
            }
            Ok(Value::Number(id as f64))
        });

        methods.add_method("unwatch", |_, this, args: Variadic<Value>| {
            // lua_tonumber: anything that is not a number is 0.
            let id = match args.first() {
                Some(Value::Integer(i)) => *i as i32,
                Some(Value::Number(n)) => *n as i32,
                _ => 0,
            };
            this.shared.backend.remove(id);
            Ok(MultiValue::new())
        });

        methods.add_method(
            "check",
            |_, this, (callback, error_callback): (Function, Option<Function>)| {
                // Take the pending bytes under the lock, run Lua without it.
                let ids = {
                    let mut state = this.shared.state.lock().unwrap();
                    if state.length < 0 {
                        return Ok(Value::Nil);
                    }
                    if state.length == 0 {
                        return Ok(Value::Boolean(false));
                    }
                    let ids = translate_changes(&state.buffer[..state.length as usize]);
                    state.length = 0;
                    ids
                };

                // Notify each watch once per check.
                let mut notified: HashSet<i32> = HashSet::new();
                for id in ids {
                    if !notified.insert(id) {
                        continue;
                    }
                    if let Err(error) = callback.call::<Value>(Value::Number(id as f64)) {
                        if let Some(handler) = &error_callback {
                            let _ = handler.call::<Value>(error_text(&error));
                        }
                    }
                }
                Ok(Value::Boolean(true))
            },
        );

        methods.add_method("mode", |_, _, ()| Ok("multiple"));
    }
}

/// `luaopen_dirmonitor`. The C returns the metatable itself as the module, with
/// `new` and `mode` (and the methods) on it; `data/` only uses `new`, `mode`
/// and the methods of the instances.
pub fn luaopen_dirmonitor(lua: &Lua) -> LuaResult<Table> {
    if let Err(reason) = register_custom_event(b"dirmonitor", None) {
        return Err(LuaError::runtime(format!(
            "Unable to register custom dirmonitor event: {}",
            reason
        )));
    }

    let module = lua.create_table()?;
    module.set(
        "new",
        lua.create_function(|lua, _: Variadic<Value>| lua.create_userdata(DirMonitor::new()))?,
    )?;
    module.set("mode", lua.create_function(|_, _: Variadic<Value>| Ok("multiple"))?)?;
    Ok(module)
}
