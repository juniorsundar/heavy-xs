//! `system`: ported piecemeal from `src/api/system.c`.
//!
//! `luaopen_system` builds the module table, creates the `NativePlugin` metatable and registers
//! the `dialogfinished` custom event. `app::api_load_libs` registers it like any other library
//! (`luaL_requiref(..., global = 1)`), so `system` is both a global and in `package.loaded`.
//!
//! The port landed as an overlay of Rust functions on the C table, one group at a time, so the
//! editor stayed runnable and the contract suite comparable after every step; this file keeps
//! that shape (`overlay`) because it is also the list of what the module exports.
//!
//! Deviations from the C are noted next to the code and listed in docs/rust-port/PLAN.md.

use std::ffi::{c_void, CString};
use std::os::unix::ffi::OsStrExt;

use mlua::{Error as LuaError, Lua, LuaString, MultiValue, Result as LuaResult, Table, Value, Variadic};
use crate::clipboard::{self, Kind as ClipboardKind};

// ------------------------------------------------------------- argument glue

fn argerror(fname: &str, arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!(
        "bad argument #{} to '{}' ({} expected, got {})",
        arg, fname, expected, got
    ))
}

fn no_value() -> &'static str {
    "no value"
}

/// `luaL_checkstring`: strings and numbers (coerced) only. Returns the bytes.
fn check_string(lua: &Lua, args: &[Value], i: usize, fname: &str) -> LuaResult<LuaString> {
    match args.get(i) {
        Some(v) => lua
            .coerce_string(v.clone())?
            .ok_or_else(|| argerror(fname, i + 1, "string", v.type_name())),
        None => Err(argerror(fname, i + 1, "string", no_value())),
    }
}

/// A NUL-terminated copy for libc and SDL, cut at an embedded NUL as `const char *` would be.
fn cstr(bytes: &[u8]) -> CString {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    CString::new(&bytes[..end]).unwrap()
}

/// The native path a `const char *` would name: the bytes up to an embedded NUL, unconverted.
fn native_path(bytes: &[u8]) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::ffi::OsStr::from_bytes(&bytes[..end]).into()
}

/// `strerror` text for an OS error, which is what SDL put in its messages. std displays it as
/// "<text> (os error N)".
pub(crate) fn os_error_text(err: &std::io::Error) -> String {
    let text = err.to_string();
    match err.raw_os_error() {
        Some(code) => text.strip_suffix(&format!(" (os error {code})")).map_or(text.clone(), str::to_owned),
        None => text,
    }
}

fn bytes_of(s: &LuaString) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// `luaL_checknumber`.
fn check_number(args: &[Value], i: usize, fname: &str) -> LuaResult<f64> {
    match args.get(i) {
        Some(Value::Integer(n)) => Ok(*n as f64),
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::String(s)) => s
            .to_str()
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| argerror(fname, i + 1, "number", "string")),
        Some(v) => Err(argerror(fname, i + 1, "number", v.type_name())),
        None => Err(argerror(fname, i + 1, "number", no_value())),
    }
}

fn str_value(lua: &Lua, bytes: &[u8]) -> LuaResult<Value> {
    Ok(Value::String(lua.create_string(bytes)?))
}

fn multi(values: Vec<Value>) -> MultiValue {
    MultiValue::from_vec(values)
}

/// `strerror` text for an errno value, through std (which calls `strerror_r`).
pub(crate) fn strerror(err: i32) -> String {
    os_error_text(&std::io::Error::from_raw_os_error(err))
}

// --------------------------------------------------------- filesystem: info

/// `f_get_file_info`: `{size, type?, modified, symlink?}`, or `nil, message`.
fn get_file_info(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    use std::os::unix::fs::MetadataExt;
    let path = check_string(lua, &args, 0, "get_file_info")?;
    let path = native_path(&bytes_of(&path));

    let st = match std::fs::metadata(&path) {
        Ok(st) => st,
        Err(err) => return Ok(multi(vec![Value::Nil, str_value(lua, os_error_text(&err).as_bytes())?])),
    };

    let info = lua.create_table()?;
    info.raw_set("size", st.size() as i64)?;
    // A type that is neither a regular file nor a directory is nil, which Lua
    // stores as an absent key (the C does `lua_pushnil` + `lua_setfield`).
    if st.is_file() {
        info.raw_set("type", "file")?;
    } else if st.is_dir() {
        info.raw_set("type", "dir")?;
    }
    info.raw_set("modified", st.mtime() as f64 + st.mtime_nsec() as f64 / 1_000_000_000.0)?;

    if st.is_dir() {
        if let Ok(lst) = std::fs::symlink_metadata(&path) {
            info.raw_set("symlink", lst.file_type().is_symlink())?;
        }
    }
    Ok(multi(vec![Value::Table(info)]))
}

/// `f_absolute_path`: zero values (not nil) on failure.
fn absolute_path(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let path = check_string(lua, &args, 0, "absolute_path")?;
    // std's canonicalize is `realpath`.
    match std::fs::canonicalize(native_path(&bytes_of(&path))) {
        Ok(resolved) => Ok(multi(vec![str_value(lua, resolved.as_os_str().as_bytes())?])),
        Err(_) => Ok(MultiValue::new()),
    }
}

/// `f_get_fs_type`: Linux magic numbers; unknown filesystems give "unknown".
fn get_fs_type(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let path = check_string(lua, &args, 0, "get_fs_type")?;
    let bytes = bytes_of(&path);
    let c_path = cstr(&bytes);
    let Ok(buf) = rustix::fs::statfs(c_path.as_c_str()) else {
        return Err(LuaError::runtime(format!(
            "error calling statfs on {}",
            String::from_utf8_lossy(c_path.to_bytes())
        )));
    };
    const NAMES: [(u32, &str); 9] = [
        (0xef53, "ext2/ext3"),
        (0x6969, "nfs"),
        (0x65735546, "fuse"),
        (0x517b, "smb"),
        (0xfe534d42, "smb2"),
        (0x52654973, "reiserfs"),
        (0x01021994, "tmpfs"),
        (0x858458f6, "ramfs"),
        (0x5346544e, "ntfs"),
    ];
    let name = NAMES
        .iter()
        .find(|(magic, _)| *magic == buf.f_type as u32)
        .map_or("unknown", |(_, name)| name);
    Ok(multi(vec![str_value(lua, name.as_bytes())?]))
}

// --------------------------------------------------- filesystem: directories

/// `f_list_dir`: names in readdir order without `.` and `..`, or `nil, message` in
/// `SDL_EnumerateDirectory`'s wording. A read error part way through ends the list quietly, as
/// SDL's readdir loop did.
fn list_dir(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    use std::os::unix::ffi::OsStrExt;
    let path = check_string(lua, &args, 0, "list_dir")?;
    let entries = match std::fs::read_dir(native_path(&bytes_of(&path))) {
        Ok(entries) => entries,
        Err(err) => {
            let msg = format!("Can't open directory: {}", os_error_text(&err));
            return Ok(multi(vec![Value::Nil, str_value(lua, msg.as_bytes())?]));
        }
    };
    let list = lua.create_table()?;
    for (i, entry) in entries.map_while(Result::ok).enumerate() {
        list.raw_set(i + 1, lua.create_string(entry.file_name().as_bytes())?)?;
    }
    Ok(multi(vec![Value::Table(list)]))
}

/// `f_mkdir`: `true`, or `false, message`.
fn mkdir(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    use std::os::unix::fs::DirBuilderExt;
    let path = check_string(lua, &args, 0, "mkdir")?;
    // A single mkdir(2) with mode 0755, before the umask.
    if let Err(err) = std::fs::DirBuilder::new().mode(0o755).create(native_path(&bytes_of(&path))) {
        return Ok(multi(vec![Value::Boolean(false), str_value(lua, os_error_text(&err).as_bytes())?]));
    }
    Ok(multi(vec![Value::Boolean(true)]))
}

/// `f_rmdir`: what `SDL_RemovePath` did through C `remove`: unlink, and rmdir only if that says
/// it is a directory. So it also removes files and symlinks (never their targets), and a path that
/// is already gone counts as removed. `true`, or `false, message` in SDL's wording.
fn rmdir(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    use std::io::ErrorKind;
    let path = check_string(lua, &args, 0, "rmdir")?;
    let path = native_path(&bytes_of(&path));
    let removed = std::fs::remove_file(&path).or_else(|err| match err.kind() {
        ErrorKind::IsADirectory => std::fs::remove_dir(&path),
        _ => Err(err),
    });
    match removed {
        Ok(()) => Ok(multi(vec![Value::Boolean(true)])),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(multi(vec![Value::Boolean(true)])),
        Err(err) => {
            let msg = format!("Can't remove path: {}", os_error_text(&err));
            Ok(multi(vec![Value::Boolean(false), str_value(lua, msg.as_bytes())?]))
        }
    }
}

/// `f_chdir`: raises on failure.
fn chdir(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let path = check_string(lua, &args, 0, "chdir")?;
    if let Err(err) = std::env::set_current_dir(native_path(&bytes_of(&path))) {
        return Err(LuaError::runtime(format!("chdir() failed: {}", os_error_text(&err))));
    }
    Ok(MultiValue::new())
}

/// `f_ftruncate`: `luaL_checkudata(L, 1, LUA_FILEHANDLE)`, a `luaL_Stream`.
fn ftruncate(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    #[repr(C)]
    struct LuaStream {
        f: *mut libc::FILE,
        closef: *const c_void,
    }

    let handle = match args.first() {
        Some(Value::UserData(ud)) => ud,
        Some(v) => return Err(argerror("ftruncate", 1, "FILE*", v.type_name())),
        None => return Err(argerror("ftruncate", 1, "FILE*", no_value())),
    };
    // `luaL_checkudata(L, 1, LUA_FILEHANDLE)`: the stream, only if the userdata
    // carries Lua's "FILE*" metatable (mlua userdata do not).
    let stream: usize = unsafe {
        lua.exec_raw(handle.clone(), |state| {
            let found = mlua::ffi::luaL_testudata(state, -1, c"FILE*".as_ptr());
            mlua::ffi::lua_pop(state, 1); // exec_raw returns the whole stack above its start
            mlua::ffi::lua_pushinteger(state, found as usize as mlua::ffi::lua_Integer);
        })?
    };
    if stream == 0 {
        return Err(argerror("ftruncate", 1, "FILE*", "userdata"));
    }
    let length = match args.get(1) {
        None | Some(Value::Nil) => 0,
        Some(Value::Integer(i)) => *i,
        Some(Value::Number(n)) => *n as i64,
        Some(v) => match lua.coerce_integer(v.clone())? {
            Some(i) => i,
            None => return Err(argerror("ftruncate", 2, "number", v.type_name())),
        },
    };

    let file = unsafe { (*(stream as *const LuaStream)).f };
    if file.is_null() {
        // shortcut: the C passes a closed handle's NULL FILE* to fileno() and crashes.
        return Err(LuaError::runtime("attempt to use a closed file"));
    }
    if unsafe { libc::ftruncate(libc::fileno(file), length as libc::off_t) } != 0 {
        let err = std::io::Error::last_os_error();
        return Ok(multi(vec![
            Value::Boolean(false),
            str_value(lua, format!("ftruncate(): {}", os_error_text(&err)).as_bytes())?,
        ]));
    }
    Ok(multi(vec![Value::Boolean(true)]))
}

// ------------------------------------------------- process, time, environment

fn get_process_id(_: &Lua, _: Variadic<Value>) -> LuaResult<i64> {
    Ok(std::process::id() as i64)
}

/// Seconds on CLOCK_MONOTONIC_RAW, the clock and nanosecond counter SDL_GetPerformanceCounter
/// used, so values keep their boot-relative origin: core's temp-file prefix is built from it and
/// must differ between editors started at different times.
fn get_time(_: &Lua, _: Variadic<Value>) -> LuaResult<f64> {
    use rustix::time::{clock_gettime, ClockId};
    let now = clock_gettime(ClockId::MonotonicRaw);
    let ticks = now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64;
    Ok(ticks as f64 / 1_000_000_000.0)
}

/// `f_sleep`: `SDL_Delay(n * 1000)`, truncated to whole milliseconds as the C does. SDL_Delay(0)
/// still made a zero-length nanosleep (a timer-slack pause), which `thread::sleep` skips.
fn sleep(_: &Lua, args: Variadic<Value>) -> LuaResult<()> {
    let n = check_number(&args, 0, "sleep")?;
    let n = if n < 0.0 { 0.0 } else { n };
    let ms = (n * 1000.0) as u32;
    if ms == 0 {
        let _ = rustix::thread::nanosleep(&rustix::time::Timespec { tv_sec: 0, tv_nsec: 0 });
    } else {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
    Ok(())
}

/// `f_exec`: runs `"<cmd> &"` the way `system()` does, through `/bin/sh -c`, waiting only for
/// the shell, which backgrounds the command and returns at once.
fn exec(lua: &Lua, args: Variadic<Value>) -> LuaResult<()> {
    let cmd = check_string(lua, &args, 0, "exec")?;
    let mut line = bytes_of(&cmd);
    line.extend_from_slice(b" &");
    let _ = std::process::Command::new("/bin/sh").arg("-c").arg(native_path(&line)).status();
    Ok(())
}

// SDL kept its own snapshot of the environment (built when SDL initialised, before Lua) and
// `get_sandbox` read the SNAP variables from it, not from the live environment; `setenv` updated
// both. The three presence flags are all the classification needs.
const SNAP_VARS: [&[u8]; 3] = [b"SNAP", b"SNAP_NAME", b"SNAP_REVISION"];
static SNAP_SNAPSHOT: std::sync::Mutex<[bool; 3]> = std::sync::Mutex::new([false; 3]);

/// Captures the snapshot at the point SDL used to: process start, before any Lua.
pub fn snapshot_environment() {
    let mut snapshot = SNAP_SNAPSHOT.lock().unwrap();
    for (present, name) in snapshot.iter_mut().zip(SNAP_VARS) {
        // SDL_getenv saw a variable whatever its value, empty included.
        *present = std::env::var_os(std::ffi::OsStr::from_bytes(name)).is_some();
    }
}

/// `f_setenv`: always overwrites. Validation and NUL truncation are libc's and SDL's: an empty
/// name or a name containing `=` is `false`.
///
/// Goes through `std::env::set_var`, not libc `setenv`, because the editor has other threads
/// (tokio workers, the clipboard): std serialises its own environment reads and writes behind one
/// lock, so Rust code on those threads cannot read a half-updated environment. A C library that
/// calls `getenv` on another thread is still outside that lock.
fn setenv(lua: &Lua, args: Variadic<Value>) -> LuaResult<bool> {
    let key = cstr(&bytes_of(&check_string(lua, &args, 0, "setenv")?));
    let value = cstr(&bytes_of(&check_string(lua, &args, 1, "setenv")?));
    let name = key.as_bytes();
    if name.is_empty() || name.contains(&b'=') {
        return Ok(false);
    }
    // SDL updated its snapshot before the OS, so a refused libc call still showed here.
    if let Some(i) = SNAP_VARS.iter().position(|v| *v == name) {
        SNAP_SNAPSHOT.lock().unwrap()[i] = true;
    }
    use std::ffi::OsStr;
    std::env::set_var(OsStr::from_bytes(name), OsStr::from_bytes(value.as_bytes()));
    Ok(true)
}

/// SDL 3.4's Linux sandbox priority, over already-gathered facts.
fn classify_sandbox(flatpak_info: bool, snap: [bool; 3], container_manager: bool) -> &'static str {
    if flatpak_info {
        "flatpak"
    } else if snap.iter().all(|present| *present) {
        "snap"
    } else if container_manager {
        "unknown"
    } else {
        "none"
    }
}

/// `access(path, F_OK)`: the path resolves, following symlinks.
fn accessible(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// `get_sandbox`: classified at the first call, then cached for the life of the process (so it
/// survives a Lua restart), independent of any window or portal. Needs `snapshot_environment`
/// to have run at startup (`app::run` does), or the SNAP variables read as absent.
fn get_sandbox(_: &Lua, _: Variadic<Value>) -> LuaResult<&'static str> {
    static SANDBOX: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    Ok(SANDBOX.get_or_init(|| {
        let snap = *SNAP_SNAPSHOT.lock().unwrap();
        classify_sandbox(accessible("/.flatpak-info"), snap, accessible("/run/host/container-manager"))
    }))
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;

    #[test]
    fn sandbox_priority_matches_sdl() {
        let none = [false; 3];
        let all = [true; 3];
        assert_eq!(classify_sandbox(true, all, true), "flatpak");
        assert_eq!(classify_sandbox(false, all, true), "snap");
        // All three variables are needed: they are set for unrelated reasons too.
        assert_eq!(classify_sandbox(false, [true, true, false], false), "none");
        assert_eq!(classify_sandbox(false, [true, true, false], true), "unknown");
        assert_eq!(classify_sandbox(false, none, true), "unknown");
        assert_eq!(classify_sandbox(false, none, false), "none");
    }

    #[test]
    fn snapshot_is_taken_once_and_setenv_updates_it() {
        // Process-global state: this is the only test that touches the SNAP variables.
        for name in SNAP_VARS {
            std::env::remove_var(std::ffi::OsStr::from_bytes(name));
        }
        std::env::set_var("SNAP", ""); // present but empty counts, as SDL_getenv saw it
        std::env::set_var("SNAP_NAME", "x");
        snapshot_environment();
        assert_eq!(*SNAP_SNAPSHOT.lock().unwrap(), [true, true, false]);
        // A change made outside the public setter is invisible to the snapshot.
        std::env::set_var("SNAP_REVISION", "1");
        assert_eq!(*SNAP_SNAPSHOT.lock().unwrap(), [true, true, false]);
        // The public setter updates both, and refuses what SDL refused.
        let lua = Lua::new();
        let call = |k: &str, v: &str| setenv(&lua, Variadic::from_iter([
            Value::String(lua.create_string(k).unwrap()),
            Value::String(lua.create_string(v).unwrap()),
        ])).unwrap();
        assert!(call("SNAP_REVISION", "2"));
        assert_eq!(*SNAP_SNAPSHOT.lock().unwrap(), [true, true, true]);
        assert!(!call("", "v"));
        assert!(!call("A=B", "v"));
        for name in SNAP_VARS {
            std::env::remove_var(std::ffi::OsStr::from_bytes(name));
        }
    }
}

// ----------------------------------------------------------------- clipboard

/// Always one string; ordinary failures and unavailable/non-text selections are empty.
fn get_clipboard(lua: &Lua, _: Variadic<Value>) -> LuaResult<LuaString> {
    lua.create_string(clipboard::get(ClipboardKind::Clipboard))
}

fn set_clipboard(lua: &Lua, args: Variadic<Value>) -> LuaResult<()> {
    let text = check_string(lua, &args, 0, "set_clipboard")?;
    clipboard::set(ClipboardKind::Clipboard, &text.as_bytes());
    Ok(())
}

fn get_primary_selection(lua: &Lua, _: Variadic<Value>) -> LuaResult<LuaString> {
    lua.create_string(clipboard::get(ClipboardKind::Primary))
}

fn set_primary_selection(lua: &Lua, args: Variadic<Value>) -> LuaResult<()> {
    let text = check_string(lua, &args, 0, "set_primary_selection")?;
    clipboard::set(ClipboardKind::Primary, &text.as_bytes());
    Ok(())
}

// ------------------------------------------------------- fuzzy and path sorting

/// `f_fuzzy_match`: case-insensitive subsequence score; 0 values for no match.
/// The C walks NUL-terminated `const char *`, so an embedded NUL ends a string.
fn fuzzy_match(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = check_string(lua, &args, 0, "fuzzy_match")?;
    let p = check_string(lua, &args, 1, "fuzzy_match")?;
    let (s, p) = (bytes_of(&s), bytes_of(&p));
    let (str_len, ptn_len) = (s.len() as i64, p.len() as i64);
    let files = matches!(args.get(2), Some(Value::Boolean(true)));

    // Indexing with `at`, which reads 0 outside the string like the C's terminator.
    let at = |buf: &[u8], i: i64| -> u8 { if i >= 0 && (i as usize) < buf.len() { buf[i as usize] } else { 0 } };
    let lower = |c: u8| c.to_ascii_lowercase();

    let (mut score, mut run) = (0i32, 0i32);
    let increment: i64 = if files { -1 } else { 1 };
    let mut st = if files { str_len - 1 } else { 0 };
    let mut pt = if files { ptn_len - 1 } else { 0 };

    while st >= 0 && pt >= 0 && at(&s, st) != 0 && at(&p, pt) != 0 {
        while st >= 0 && at(&s, st) == b' ' {
            st += increment;
        }
        while pt >= 0 && at(&p, pt) == b' ' {
            pt += increment;
        }
        if lower(at(&s, st)) == lower(at(&p, pt)) {
            score += run * 10 - (at(&s, st) != at(&p, pt)) as i32;
            run += 1;
            pt += increment;
        } else {
            score -= 10;
            run = 0;
        }
        st += increment;
    }
    if pt >= 0 && at(&p, pt) != 0 {
        return Ok(MultiValue::new());
    }
    Ok(multi(vec![Value::Integer((score - (str_len as i32) * 10) as i64)]))
}

/// `f_path_compare`: directories first, numeric runs as integers, `/` as a terminator.
fn path_compare(lua: &Lua, args: Variadic<Value>) -> LuaResult<bool> {
    let p1 = bytes_of(&check_string(lua, &args, 0, "path_compare")?);
    let t1 = bytes_of(&check_string(lua, &args, 1, "path_compare")?);
    let p2 = bytes_of(&check_string(lua, &args, 2, "path_compare")?);
    let t2 = bytes_of(&check_string(lua, &args, 3, "path_compare")?);
    let (len1, len2) = (p1.len(), p2.len());

    // Reads 0 past the end, like the C's terminator.
    let at = |b: &[u8], i: usize| -> u8 { b.get(i).copied().unwrap_or(0) };
    // `strcmp(type, "dir")` sees the type up to its first NUL.
    let nul_cut = |b: &[u8]| -> Vec<u8> { b.iter().copied().take_while(|&c| c != 0).collect() };
    let mut type1 = (nul_cut(&t1) != b"dir") as i32;
    let mut type2 = (nul_cut(&t2) != b"dir") as i32;

    let mut offset = 0usize;
    let mut i = 0;
    while i < len1 && i < len2 {
        if p1[i] != p2[i] {
            break;
        }
        if p1[i] == b'/' {
            offset = i + 1;
        }
        i += 1;
    }
    // `strchr(path + offset, PATHSEP)` stops at the first NUL.
    let has_sep = |b: &[u8]| b.iter().skip(offset).take_while(|&&c| c != 0).any(|&c| c == b'/');
    if has_sep(&p1) {
        type1 = 0;
    }
    if has_sep(&p2) {
        type2 = 0;
    }
    if type1 != type2 {
        return Ok(type1 < type2);
    }

    let mut cfr: i32 = -1;
    let same_len = len1 == len2;
    let (mut i, mut j) = (offset, offset);
    while i <= len1 && j <= len2 {
        let (a, b) = (at(&p1, i), at(&p2, j));
        if a == 0 || b == 0 {
            if cfr < 0 {
                cfr = 0;
            }
            if !same_len {
                cfr = (a == 0) as i32;
            }
        } else if a.is_ascii_digit() && b.is_ascii_digit() {
            let (mut ii, mut ij) = (0usize, 0usize);
            while at(&p1, i + ii).is_ascii_digit() {
                ii += 1;
            }
            while at(&p2, j + ij).is_ascii_digit() {
                ij += 1;
            }
            // size_t arithmetic in the C: wraps on overflow.
            let (mut di, mut dj) = (0usize, 0usize);
            for k in 0..ii {
                di = di.wrapping_mul(10).wrapping_add((at(&p1, i + k) - b'0') as usize);
            }
            for k in 0..ij {
                dj = dj.wrapping_mul(10).wrapping_add((at(&p2, j + k) - b'0') as usize);
            }
            if di == dj {
                i += 1;
                j += 1;
                continue;
            }
            cfr = (di < dj) as i32;
        } else if a == b {
            i += 1;
            j += 1;
            continue;
        } else if a == b'/' || b == b'/' {
            cfr = (a == b'/') as i32;
        } else {
            let (la, lb) = (a.to_ascii_lowercase_c(), b.to_ascii_lowercase_c());
            if la == lb {
                if same_len && cfr < 0 {
                    cfr = (a > b) as i32;
                }
                i += 1;
                j += 1;
                continue;
            }
            cfr = (la < lb) as i32;
        }
        break;
    }
    Ok(cfr != 0)
}

/// The C folds `'A'..='Z'` by adding 32 on a `char`; identical to this for ASCII.
trait AsciiLowerC {
    fn to_ascii_lowercase_c(self) -> u8;
}
impl AsciiLowerC for u8 {
    fn to_ascii_lowercase_c(self) -> u8 {
        if self.is_ascii_uppercase() { self + 32 } else { self }
    }
}

// -------------------------------------------------------------- the overlay

/// Replaces functions on `system` with their Rust versions.
fn overlay(lua: &Lua, system: &Table) -> LuaResult<()> {
    macro_rules! set {
        ($name:literal, $f:expr) => {
            system.raw_set($name, lua.create_function($f)?)?
        };
    }
    set!("get_file_info", get_file_info);
    set!("absolute_path", absolute_path);
    set!("get_fs_type", get_fs_type);
    set!("list_dir", list_dir);
    set!("mkdir", mkdir);
    set!("rmdir", rmdir);
    set!("chdir", chdir);
    set!("ftruncate", ftruncate);
    set!("get_process_id", get_process_id);
    set!("get_time", get_time);
    set!("sleep", sleep);
    set!("exec", exec);
    set!("setenv", setenv);
    set!("get_sandbox", get_sandbox);
    set!("get_clipboard", get_clipboard);
    set!("set_clipboard", set_clipboard);
    set!("get_primary_selection", get_primary_selection);
    set!("set_primary_selection", set_primary_selection);
    set!("fuzzy_match", fuzzy_match);
    set!("path_compare", path_compare);

    // Raw lua_CFunctions: poll_event returns 0-6 values and drives C callbacks that push directly.
    unsafe {
        use crate::sysplugin as p;
        for (name, f) in [
            ("load_native_plugin", p::load_native_plugin as mlua::lua_CFunction),
            ("open_file_dialog", p::open_file_dialog),
            ("save_file_dialog", p::save_file_dialog),
            ("open_directory_dialog", p::open_directory_dialog),
        ] {
            system.raw_set(name, lua.create_c_function(f)?)?;
        }
        use crate::syswindow as w;
        for (name, f) in [
            ("set_cursor", w::set_cursor as mlua::lua_CFunction),
            ("set_window_title", w::set_window_title),
            ("set_window_mode", w::set_window_mode),
            ("get_window_mode", w::get_window_mode),
            ("set_window_bordered", w::set_window_bordered),
            ("set_window_hit_test", w::set_window_hit_test),
            ("get_window_size", w::get_window_size),
            ("set_window_size", w::set_window_size),
            ("set_text_input_rect", w::set_text_input_rect),
            ("clear_ime", w::clear_ime),
            ("window_has_focus", w::window_has_focus),
            ("raise_window", w::raise_window),
            ("show_fatal_error", w::show_fatal_error),
            ("set_window_opacity", w::set_window_opacity),
            ("text_input", w::text_input),
        ] {
            system.raw_set(name, lua.create_c_function(f)?)?;
        }
        system.raw_set("poll_event", lua.create_c_function(crate::sysevents::poll_event)?)?;
        system.raw_set("wait_event", lua.create_c_function(crate::sysevents::wait_event)?)?;
    }
    Ok(())
}

/// `luaopen_system`: returns the module table. Fails (like the C's `luaL_error`) if the
/// `dialogfinished` event cannot be registered.
pub fn luaopen_system(lua: &Lua) -> LuaResult<Table> {
    unsafe {
        if let Err(reason) = crate::custom_events::register_custom_event(
            b"dialogfinished",
            Some(crate::sysplugin::dialogfinished_callback),
        ) {
            return Err(LuaError::runtime(format!(
                "Unable to register custom dialogfinished event: {reason}"
            )));
        }
        // The NativePlugin metatable is looked up by name (luaL_setmetatable) when a plugin loads.
        mlua::ffi::luaL_newmetatable(lua.state(), c"NativePlugin".as_ptr());
        mlua::ffi::lua_pushcclosure(lua.state(), crate::sysplugin::library_gc, 0);
        mlua::ffi::lua_setfield(lua.state(), -2, c"__gc".as_ptr());
        mlua::ffi::lua_settop(lua.state(), mlua::ffi::lua_gettop(lua.state()) - 1);
    }
    #[cfg(feature = "test_hooks")]
    crate::test_hooks::install(lua)?;
    let system = lua.create_table()?;
    overlay(lua, &system)?;
    Ok(system)
}

