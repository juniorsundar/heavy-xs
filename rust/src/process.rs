//! `process`: spawning and talking to child processes, ported from
//! `src/api/process.c` (POSIX path).
//!
//! Two pieces: the `process_t` userdata, whose `Drop` mirrors the C's `__gc`
//! (signal the child, then either queue it for the kill-list worker or wait
//! synchronously), and the kill-list worker itself, a process-wide thread that
//! retries SIGTERM and finally SIGKILL. The C keeps the worker in the Lua
//! registry as a userdata; here it is a `static`, which is equivalent because
//! the editor has exactly one Lua state. The worker never touches Lua.
//!
//! Deliberate deviations from the C, all on paths where it has undefined
//! behaviour or would crash (see the `shortcut:` comments):
//!   * file descriptors use -1 as the closed sentinel, not 0, which in C is a
//!     real fd, so writing to a closed stdin writes to fd 0 instead of failing;
//!   * a negative read size, an out-of-range stream index, a `REDIRECT_DEFAULT`
//!     stream value and an empty command all fail cleanly;
//!   * an env block without `=` stops instead of dereferencing NULL.

use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::os::unix::io::RawFd;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mlua::{
    Error as LuaError, Lua, LuaString, MetaMethod, MultiValue, Result as LuaResult, Table, UserData,
    UserDataMethods, Value, Variadic,
};

// Tunables and Lua-visible constants, from src/api/process.c:28-31 and :76-95.
const READ_BUF_SIZE: i64 = 2048;
const PROCESS_TERM_TRIES: i32 = 3;
const PROCESS_TERM_DELAY: i64 = 50; // milliseconds
/// Only a chunk size for reads: the loop keeps going until the caller's size is
/// used up, so its exact value is invisible in the result.
const LUAL_BUFFERSIZE: i64 = 8192;

const WAIT_NONE: i64 = 0;
const WAIT_DEADLINE: i64 = -1;
const WAIT_INFINITE: i64 = -2;

const STDIN_FD: i64 = 0;
const STDOUT_FD: i64 = 1;
const STDERR_FD: i64 = 2;
const REDIRECT_DEFAULT: i64 = -1;
const REDIRECT_DISCARD: i64 = -2;
const REDIRECT_PARENT: i64 = -3;

const API_TYPE_PROCESS: &str = "Process";

#[derive(Clone, Copy)]
enum Signal {
    Interrupt,
    Term,
    Kill,
}

impl Signal {
    fn number(self) -> libc::c_int {
        match self {
            Signal::Interrupt => libc::SIGINT,
            Signal::Term => libc::SIGTERM,
            Signal::Kill => libc::SIGKILL,
        }
    }
}

// ---------------------------------------------------------------- libc glue

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// `strerror`, as `process.strerror` exposes it.
fn strerror(err: i32) -> String {
    unsafe { CStr::from_ptr(libc::strerror(err)) }
        .to_string_lossy()
        .into_owned()
}

/// `push_error`: the C's `"%s: %s (%d)"` message shape.
fn error_message(extra: &str, err: i32) -> String {
    format!("{}: {} ({})", extra, strerror(err), err)
}

fn now_ms() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as i64
}

fn close_fd(fd: &mut RawFd) {
    if *fd >= 0 {
        unsafe { libc::close(*fd) };
        *fd = -1;
    }
}

/// `signal_process`: the child called `setpgid(0,0)` (or `setsid()`), so a
/// negative pid signals its whole process group.
fn signal_pid(pid: i64, sig: Signal) -> bool {
    unsafe { libc::kill(-(pid as libc::pid_t), sig.number()) == 0 }
}

/// `process_handle_is_running`: `None` while the child runs, otherwise its exit
/// status (reaping it as a side effect). Like the C this uses `WEXITSTATUS` on
/// the raw status, so a child killed by a signal reports 0.
fn try_wait(pid: i64) -> Option<i32> {
    let mut status: libc::c_int = 0;
    let waited = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) };
    if waited != 0 {
        Some((status >> 8) & 0xff)
    } else {
        None
    }
}

// ------------------------------------------------------------ the userdata

/// `process_t`. Only the parent side of each pipe is kept; the C keeps both and
/// closes the child's ends right after forking.
struct Process {
    running: bool,
    detached: bool,
    returncode: i32,
    deadline: i64,
    pid: i64,
    stdin_fd: RawFd,
    stdout_fd: RawFd,
    stderr_fd: RawFd,
}

impl Process {
    /// `poll_process`.
    fn poll(&mut self, timeout: i64) -> bool {
        if !self.running {
            return false;
        }
        let timeout = if timeout == WAIT_DEADLINE { self.deadline } else { timeout };
        let started = now_ms();
        loop {
            if let Some(status) = try_wait(self.pid) {
                self.running = false;
                self.returncode = status;
                break;
            }
            if timeout != 0 {
                if timeout >= 5 {
                    thread::sleep(Duration::from_millis(5));
                } else {
                    thread::yield_now();
                }
            }
            if !(timeout == WAIT_INFINITE || now_ms() - started < timeout) {
                break;
            }
        }
        self.running
    }

    /// `signal_process`: always reports success, as the C does.
    fn signal(&mut self, sig: Signal) -> bool {
        if signal_pid(self.pid, sig) {
            self.poll(WAIT_NONE);
        }
        true
    }

    fn stream_fd(&self, stream: i64) -> RawFd {
        match stream {
            STDIN_FD => self.stdin_fd,
            STDOUT_FD => self.stdout_fd,
            STDERR_FD => self.stderr_fd,
            _ => -1,
        }
    }
}

impl Drop for Process {
    /// `f_gc`: if the child is still running and not detached, ask it to stop
    /// and hand it to the kill-list worker, or wait for it here when there is
    /// no worker.
    fn drop(&mut self) {
        if self.poll(WAIT_NONE) && !self.detached {
            self.signal(Signal::Term);
            if !kill_list_push(self.pid) {
                if self.poll(PROCESS_TERM_DELAY) {
                    self.signal(Signal::Kill);
                    self.poll(PROCESS_TERM_DELAY);
                }
            }
        }
        close_fd(&mut self.stdin_fd);
        close_fd(&mut self.stdout_fd);
        close_fd(&mut self.stderr_fd);
    }
}

// ------------------------------------------------------------ kill-list

struct KillTask {
    tries: i32,
    start: Instant,
    pid: i64,
}

#[derive(Default)]
struct KillState {
    stop: bool,
    dead: bool,
    queue: VecDeque<KillTask>,
    handle: Option<JoinHandle<()>>,
}

struct KillList {
    state: Mutex<KillState>,
    has_work: Condvar,
    work_done: Condvar,
}

static KILL_LIST: OnceLock<Arc<KillList>> = OnceLock::new();

/// `kill_list_init`: start the worker on first use.
fn kill_list() -> &'static Arc<KillList> {
    KILL_LIST.get_or_init(|| {
        let list = Arc::new(KillList {
            state: Mutex::new(KillState::default()),
            has_work: Condvar::new(),
            work_done: Condvar::new(),
        });
        let spawned = thread::Builder::new()
            .name("process_kill".into())
            .spawn({
                let list = Arc::clone(&list);
                move || kill_worker(list)
            });
        {
            let mut state = list.state.lock().unwrap();
            match spawned {
                Ok(handle) => state.handle = Some(handle),
                // No worker: every destructor falls back to waiting itself.
                Err(_) => {
                    state.stop = true;
                    state.dead = true;
                }
            }
        }
        list
    })
}

/// `kill_list_push`. Returns false when there is no worker to take the task.
fn kill_list_push(pid: i64) -> bool {
    let list = kill_list();
    let mut state = list.state.lock().unwrap();
    if state.dead || state.handle.is_none() {
        return false;
    }
    state.queue.push_back(KillTask {
        tries: 1,
        start: Instant::now(),
        pid,
    });
    list.has_work.notify_all();
    true
}

/// `kill_list_wait_all` + `kill_list_free`: drain the queue, stop the worker,
/// join it. Called from the module table's `__gc`.
fn kill_list_shutdown() {
    let Some(list) = KILL_LIST.get() else {
        return;
    };
    let handle = {
        let mut state = list.state.lock().unwrap();
        while !state.queue.is_empty() {
            state = list.work_done.wait(state).unwrap();
        }
        state.stop = true;
        state.dead = true;
        list.has_work.notify_all();
        state.handle.take()
    };
    if let Some(handle) = handle {
        let _ = handle.join();
    }
}

/// `kill_list_worker`: SIGTERM while `tries < 3`, SIGKILL at 3, then drop it.
fn kill_worker(list: Arc<KillList>) {
    loop {
        let mut state = list.state.lock().unwrap();
        while state.queue.is_empty() && !state.stop {
            state = list.has_work.wait(state).unwrap();
        }
        if state.stop {
            return;
        }

        loop {
            let due = state
                .queue
                .front()
                .is_some_and(|task| task.start.elapsed() >= Duration::from_millis(PROCESS_TERM_DELAY as u64));
            if !due {
                break;
            }
            let mut task = state.queue.pop_front().unwrap();
            if try_wait(task.pid).is_none() {
                if task.tries < PROCESS_TERM_TRIES {
                    signal_pid(task.pid, Signal::Term);
                } else if task.tries == PROCESS_TERM_TRIES {
                    signal_pid(task.pid, Signal::Kill);
                }
                if task.tries <= PROCESS_TERM_TRIES {
                    task.tries += 1;
                    task.start = Instant::now();
                    state.queue.push_back(task);
                    continue;
                }
            }
            list.work_done.notify_all();
        }

        let delay = state
            .queue
            .front()
            .map(|task| PROCESS_TERM_DELAY - task.start.elapsed().as_millis() as i64)
            .unwrap_or(0);
        drop(state);
        if delay > 0 {
            thread::sleep(Duration::from_millis(delay as u64));
        }
    }
}

// ----------------------------------------------------------- Lua arguments

fn argerror(fname: &str, arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!(
        "bad argument #{} to '{}' ({} expected, got {})",
        arg, fname, expected, got
    ))
}

fn coerce_string(lua: &Lua, v: &Value, arg: usize, fname: &str) -> LuaResult<LuaString> {
    lua.coerce_string(v.clone())?
        .ok_or_else(|| argerror(fname, arg, "string", v.type_name()))
}

/// `luaL_checknumber` reduced to what the C actually reads back.
fn number_value(v: &Value, def: i64, arg: usize, fname: &str) -> LuaResult<i64> {
    match v {
        Value::Nil => Ok(def),
        Value::Integer(i) => Ok(*i),
        Value::Number(n) => Ok(*n as i64),
        other => Err(argerror(fname, arg, "number", other.type_name())),
    }
}

/// `luaL_optnumber(L, idx, def)` for a pushed-out-of-a-table field.
fn opt_field(table: &Table, key: &str, def: i64, arg: usize, fname: &str) -> LuaResult<i64> {
    number_value(&table.raw_get::<Value>(key)?, def, arg, fname)
}

fn truthy(v: &Value) -> bool {
    !matches!(v, Value::Nil | Value::Boolean(false))
}

fn arg_at<'a>(args: &'a [Value], i: usize) -> Option<&'a Value> {
    args.get(i).filter(|v| !matches!(v, Value::Nil))
}

/// A C string out of a Lua string, stopping at an embedded NUL the way
/// `execvp` would.
fn c_string(bytes: &[u8]) -> LuaResult<CString> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    CString::new(&bytes[..end]).map_err(|_| LuaError::runtime("invalid argument: embedded NUL"))
}

// ------------------------------------------------------------------ spawn

/// `process_start`. On success the parent owns only the parent ends of the
/// three pipes, which the returned userdata then closes.
fn process_start(lua: &Lua, args: Variadic<Value>) -> LuaResult<Value> {
    let command = match arg_at(&args, 0) {
        Some(Value::Table(t)) => t.clone(),
        Some(other) => return Err(argerror("process.start", 1, "table", other.type_name())),
        None => return Err(argerror("process.start", 1, "table", "no value")),
    };

    let argc = command.raw_len();
    let mut argv_owned: Vec<CString> = Vec::with_capacity(argc);
    for i in 1..=argc {
        let value: Value = command.raw_get(i)?;
        let string = coerce_string(lua, &value, 1, "process.start")?;
        let bytes = string.as_bytes();
        let bytes: &[u8] = &bytes;
        argv_owned.push(c_string(bytes)?);
    }
    if argv_owned.is_empty() {
        // shortcut: the C hands a NULL argv[0] to execvp, which kills the child
        // with a signal before it can report anything and makes `start` claim
        // success; report the failure it means instead.
        return Err(LuaError::runtime(format!(
            "Error creating child process: {}",
            strerror(libc::ENOENT)
        )));
    }
    let mut argv: Vec<*const libc::c_char> = argv_owned.iter().map(|a| a.as_ptr()).collect();
    argv.push(std::ptr::null());

    let mut detach = false;
    let mut deadline = 10i64;
    let mut new_fds = [STDIN_FD, STDOUT_FD, STDERR_FD];
    let mut env_block: Option<Vec<u8>> = None;
    let mut cwd: Option<CString> = None;

    if let Some(Value::Table(options)) = arg_at(&args, 1) {
        detach = truthy(&options.raw_get::<Value>("detach")?);
        deadline = opt_field(&options, "timeout", deadline, 2, "process.start")?;
        new_fds[0] = opt_field(&options, "stdin", STDIN_FD, 2, "process.start")?;
        new_fds[1] = opt_field(&options, "stdout", STDOUT_FD, 2, "process.start")?;
        new_fds[2] = opt_field(&options, "stderr", STDERR_FD, 2, "process.start")?;
        for stream in 0..3 {
            if new_fds[stream] > STDERR_FD || new_fds[stream] < REDIRECT_PARENT {
                return Err(LuaError::runtime(
                    "error: redirect to handles, FILE* and paths are not supported",
                ));
            }
            // shortcut: the C accepts REDIRECT_DEFAULT (-1) and then reads
            // child_pipes[-1] in the child; treat it as "this stream's default".
            if new_fds[stream] == REDIRECT_DEFAULT {
                new_fds[stream] = stream as i64;
            }
        }

        if let Value::Function(env_fn) = options.raw_get::<Value>("env")? {
            let empty = lua.create_table()?;
            let returned: Value = env_fn.call(empty)?;
            let block = lua
                .coerce_string(returned)?
                .ok_or_else(|| LuaError::runtime("error: env must return a string"))?;
            let bytes = block.as_bytes();
            let bytes: &[u8] = &bytes;
            let mut owned = bytes.to_vec();
            owned.push(0); // the child walks it with strlen
            env_block = Some(owned);
        }

        if let Some(path) = lua.coerce_string(options.raw_get::<Value>("cwd")?)? {
            let bytes = path.as_bytes();
            let bytes: &[u8] = &bytes;
            cwd = Some(c_string(bytes)?);
        }
    }

    // Make only the parent's ends non-blocking; the child's must block.
    let mut pipes: [[RawFd; 2]; 3] = [[-1, -1], [-1, -1], [-1, -1]];
    for stream in 0..3usize {
        let mut fds: [RawFd; 2] = [-1, -1];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            let err = errno();
            close_pipes(&mut pipes);
            return Err(LuaError::runtime(error_message("cannot create pipe", err)));
        }
        pipes[stream] = fds;
        let parent_side = if stream == 0 { 1 } else { 0 };
        if unsafe { libc::fcntl(fds[parent_side], libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
            let err = errno();
            close_pipes(&mut pipes);
            return Err(LuaError::runtime(error_message("cannot create pipe", err)));
        }
    }

    // A pipe whose write end the child holds until `exec` reports back whether
    // `execvp` worked.
    let mut control: [RawFd; 2] = [-1, -1];
    if unsafe { libc::pipe(control.as_mut_ptr()) } == -1 {
        let err = errno();
        close_pipes(&mut pipes);
        return Err(LuaError::runtime(format!(
            "Error creating control pipe: {}",
            strerror(err)
        )));
    }
    if unsafe { libc::fcntl(control[1], libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
        let err = errno();
        unsafe {
            libc::close(control[0]);
            libc::close(control[1]);
        }
        close_pipes(&mut pipes);
        return Err(LuaError::runtime(format!(
            "Error setting FD_CLOEXEC: {}",
            strerror(err)
        )));
    }

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        let err = errno();
        unsafe {
            libc::close(control[0]);
            libc::close(control[1]);
        }
        close_pipes(&mut pipes);
        return Err(LuaError::runtime(error_message("cannot create child process", err)));
    }

    if pid == 0 {
        // Child: no allocation, no destructors, only libc calls.
        unsafe {
            child_exec(
                detach,
                new_fds,
                pipes,
                control[1],
                argv.as_ptr(),
                env_block.as_mut().map_or(std::ptr::null_mut(), |b| b.as_mut_ptr()),
                cwd.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            )
        };
    }

    // Parent: drop the child's ends so reads see EOF when the child exits.
    for stream in 0..3usize {
        let child_side = if stream == 0 { 0 } else { 1 };
        unsafe { libc::close(pipes[stream][child_side]) };
        pipes[stream][child_side] = -1;
    }
    unsafe { libc::close(control[1]) };
    control[1] = -1;

    // Either the child execs (CLOEXEC closes the write end, giving EOF) or it
    // writes the errno that stopped it.
    let mut reported = [0u8; 4];
    let read_len = loop {
        let n = unsafe {
            libc::read(
                control[0],
                reported.as_mut_ptr() as *mut libc::c_void,
                reported.len(),
            )
        };
        if n >= 0 {
            break n;
        }
        let err = errno();
        if err == libc::EINTR {
            continue;
        }
        unsafe { libc::close(control[0]) };
        close_pipes(&mut pipes);
        return Err(LuaError::runtime(format!(
            "Error getting child process status: {}",
            strerror(err)
        )));
    };
    unsafe { libc::close(control[0]) };

    if read_len != 0 {
        let mut status: libc::c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        close_pipes(&mut pipes);
        return Err(LuaError::runtime(format!(
            "Error creating child process: {}",
            strerror(i32::from_ne_bytes(reported))
        )));
    }

    let process = Process {
        running: true,
        detached: detach,
        returncode: 0,
        deadline,
        pid: pid as i64,
        stdin_fd: pipes[0][1],
        stdout_fd: pipes[1][0],
        stderr_fd: pipes[2][0],
    };
    Ok(Value::UserData(lua.create_userdata(process)?))
}

/// Between `fork()` and `execvp()`: redirect the standard streams, apply the
/// environment block and the working directory, then exec or report the errno.
unsafe fn child_exec(
    detach: bool,
    new_fds: [i64; 3],
    pipes: [[RawFd; 2]; 3],
    control_write: RawFd,
    argv: *const *const libc::c_char,
    mut env: *mut u8,
    cwd: *const libc::c_char,
) -> ! {
    if !detach {
        libc::setpgid(0, 0);
    }
    for stream in 0..3usize {
        let child_side = if stream == 0 { 0 } else { 1 };
        let parent_side = if stream == 0 { 1 } else { 0 };
        match new_fds[stream] {
            REDIRECT_DISCARD => {
                libc::close(pipes[stream][child_side]);
                libc::close(stream as RawFd);
            }
            REDIRECT_PARENT => {}
            target => {
                let target = target as usize;
                let from = pipes[target][if target == 0 { 0 } else { 1 }];
                libc::dup2(from, stream as RawFd);
            }
        }
        libc::close(pipes[stream][parent_side]);
    }

    if !env.is_null() {
        loop {
            let len = libc::strlen(env as *const libc::c_char);
            if len == 0 {
                break;
            }
            let equals = libc::strchr(env as *const libc::c_char, b'=' as libc::c_int);
            if equals.is_null() {
                // shortcut: the C writes through this NULL pointer.
                break;
            }
            *equals = 0;
            libc::setenv(env as *const libc::c_char, equals.add(1), 1);
            env = env.add(len + 1);
        }
    }

    if (!detach || libc::setsid() != -1) && (cwd.is_null() || libc::chdir(cwd) != -1) {
        libc::execvp(*argv, argv);
    }
    let err = errno();
    libc::write(control_write, &err as *const i32 as *const libc::c_void, 4);
    libc::_exit(-1)
}

fn close_pipes(pipes: &mut [[RawFd; 2]; 3]) {
    for pair in pipes.iter_mut() {
        for fd in pair.iter_mut() {
            close_fd(fd);
        }
    }
}

// -------------------------------------------------------------- streams

/// `g_read`. Reads until `read_size` bytes are collected, the stream ends or
/// an error stops it; an empty string means "nothing available right now".
fn g_read(lua: &Lua, process: &mut Process, stream: i64, read_size: i64) -> LuaResult<MultiValue> {
    if stream != STDOUT_FD && stream != STDERR_FD {
        return Err(LuaError::runtime(
            "error: redirect to handles, FILE* and paths are not supported",
        ));
    }
    let fd = process.stream_fd(stream);
    let mut out: Vec<u8> = Vec::new();
    let mut remaining = read_size;
    let mut buffer = [0u8; LUAL_BUFFERSIZE as usize];

    loop {
        let want = if remaining < LUAL_BUFFERSIZE {
            remaining
        } else {
            LUAL_BUFFERSIZE
        };
        if want < 0 {
            // shortcut: the C passes the negative size to read() as a huge
            // size_t, which fails with EFAULT; do what it looks like it meant.
            process.signal(Signal::Term);
            return Ok(MultiValue::new());
        }

        let n = unsafe {
            libc::read(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                want as usize,
            )
        };
        let mut length = n as i64;
        if length == 0 {
            // EOF only once the child is gone; otherwise "nothing yet".
            if !process.poll(WAIT_NONE) {
                break;
            }
        } else if length < 0 {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                length = 0;
            } else {
                process.signal(Signal::Term);
                return Ok(MultiValue::new());
            }
        }
        if length > 0 {
            out.extend_from_slice(&buffer[..length as usize]);
            remaining -= length;
        }
        if !(remaining > 0 && length > 0) {
            break;
        }
    }

    Ok(MultiValue::from_vec(vec![Value::String(
        lua.create_string(&out)?,
    )]))
}

/// `f_write`.
fn write_stdin(lua: &Lua, process: &mut Process, args: &Variadic<Value>) -> LuaResult<MultiValue> {
    let value = arg_at(args, 0)
        .map(|v| coerce_string(lua, v, 2, "write"))
        .transpose()?
        .ok_or_else(|| argerror("write", 2, "string", "no value"))?;
    let bytes = value.as_bytes();
    let bytes: &[u8] = &bytes;

    let written = unsafe { libc::write(process.stdin_fd, bytes.as_ptr() as *const libc::c_void, bytes.len()) };
    if written < 0 {
        let err = errno();
        if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
            return Ok(MultiValue::from_vec(vec![Value::Integer(0)]));
        }
        let message = error_message("cannot write to child process", err);
        process.signal(Signal::Term);
        return Err(LuaError::runtime(message));
    }
    Ok(MultiValue::from_vec(vec![Value::Integer(written as i64)]))
}

// ------------------------------------------------------------- Lua module

impl UserData for Process {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("pid", |_, this, ()| Ok(this.pid));
        methods.add_meta_method(MetaMethod::Type, |_, _, ()| Ok(API_TYPE_PROCESS));
        methods.add_meta_method(MetaMethod::ToString, |_, _, ()| Ok(API_TYPE_PROCESS));

        methods.add_method_mut("running", |_, this, ()| Ok(this.poll(WAIT_NONE)));

        methods.add_method_mut("returncode", |_, this, ()| {
            if this.poll(WAIT_NONE) {
                Ok(MultiValue::new())
            } else {
                Ok(MultiValue::from_vec(vec![Value::Integer(this.returncode as i64)]))
            }
        });

        methods.add_method_mut("wait", |_, this, args: Variadic<Value>| {
            let timeout = number_value(arg_at(&args, 0).unwrap_or(&Value::Nil), WAIT_NONE, 2, "wait")?;
            if this.poll(timeout) {
                Ok(MultiValue::new())
            } else {
                Ok(MultiValue::from_vec(vec![Value::Integer(this.returncode as i64)]))
            }
        });

        methods.add_method_mut("read", |lua, this, args: Variadic<Value>| {
            let stream = number_value(
                arg_at(&args, 0).ok_or_else(|| argerror("read", 2, "number", "no value"))?,
                0,
                2,
                "read",
            )?;
            let size = number_value(arg_at(&args, 1).unwrap_or(&Value::Nil), READ_BUF_SIZE, 3, "read")?;
            g_read(lua, this, stream, size)
        });

        methods.add_method_mut("read_stdout", |lua, this, args: Variadic<Value>| {
            let size = number_value(
                arg_at(&args, 0).unwrap_or(&Value::Nil),
                READ_BUF_SIZE,
                2,
                "read_stdout",
            )?;
            g_read(lua, this, STDOUT_FD, size)
        });

        methods.add_method_mut("read_stderr", |lua, this, args: Variadic<Value>| {
            let size = number_value(
                arg_at(&args, 0).unwrap_or(&Value::Nil),
                READ_BUF_SIZE,
                2,
                "read_stderr",
            )?;
            g_read(lua, this, STDERR_FD, size)
        });

        methods.add_method_mut("write", |lua, this, args: Variadic<Value>| {
            write_stdin(lua, this, &args)
        });

        methods.add_method_mut("close_stream", |_, this, args: Variadic<Value>| {
            let stream = number_value(
                arg_at(&args, 0).ok_or_else(|| argerror("close_stream", 2, "number", "no value"))?,
                0,
                2,
                "close_stream",
            )?;
            // shortcut: the C indexes child_pipes[stream] unchecked here, so a
            // bad index closes whatever fd the garbage points at; refuse it.
            let fd = match stream {
                STDIN_FD => &mut this.stdin_fd,
                STDOUT_FD => &mut this.stdout_fd,
                STDERR_FD => &mut this.stderr_fd,
                other => {
                    return Err(LuaError::runtime(format!(
                        "error: unknown stream {}",
                        other
                    )))
                }
            };
            close_fd(fd);
            Ok(true)
        });

        methods.add_method_mut("terminate", |_, this, ()| {
            this.signal(Signal::Term);
            Ok(true)
        });
        methods.add_method_mut("kill", |_, this, ()| {
            this.signal(Signal::Kill);
            Ok(true)
        });
        methods.add_method_mut("interrupt", |_, this, ()| {
            this.signal(Signal::Interrupt);
            Ok(true)
        });
    }
}

pub fn luaopen_process(lua: &Lua) -> LuaResult<Table> {
    kill_list(); // `kill_list_init`: the worker exists from module load on

    let metatable = lua.create_table()?;
    metatable.set(
        "__gc",
        lua.create_function(|_, _: Value| {
            kill_list_shutdown();
            Ok(())
        })?,
    )?;

    let module = lua.create_table()?;
    module.set_metatable(Some(metatable))?;
    module.set("start", lua.create_function(process_start)?)?;
    module.set(
        "strerror",
        lua.create_function(|lua, args: Variadic<Value>| {
            let err = number_value(
                arg_at(&args, 0).ok_or_else(|| argerror("strerror", 1, "number", "no value"))?,
                0,
                1,
                "strerror",
            )?;
            Ok(Value::String(lua.create_string(strerror(err as i32))?))
        })?,
    )?;

    // The C pushes these with lua_pushnumber, so they are floats, not integers.
    let number = Value::Number;
    module.set("WAIT_INFINITE", number(WAIT_INFINITE as f64))?;
    module.set("WAIT_DEADLINE", number(WAIT_DEADLINE as f64))?;
    module.set("STREAM_STDIN", number(STDIN_FD as f64))?;
    module.set("STREAM_STDOUT", number(STDOUT_FD as f64))?;
    module.set("STREAM_STDERR", number(STDERR_FD as f64))?;
    module.set("REDIRECT_DEFAULT", number(REDIRECT_DEFAULT as f64))?;
    module.set("REDIRECT_STDOUT", number(STDOUT_FD as f64))?;
    module.set("REDIRECT_STDERR", number(STDERR_FD as f64))?;
    module.set("REDIRECT_PARENT", number(REDIRECT_PARENT as f64))?;
    module.set("REDIRECT_DISCARD", number(REDIRECT_DISCARD as f64))?;

    Ok(module)
}
