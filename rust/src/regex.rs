//! `regex`: PCRE2-backed pattern matching, ported from `src/api/regex.c`.
//!
//! Lua-visible contract (see docs/rust-port/PLAN.md §8 W1): a compiled pattern
//! is a *table* whose metatable is this module's table, so `pattern:gsub(...)`
//! works through the `__index` that `data/core/regex.lua` installs.

use std::cell::RefCell;
use std::os::raw::c_int;
use std::rc::Rc;

use mlua::{
    BorrowedBytes, Error as LuaError, LightUserData, Lua, LuaString, MultiValue, Result as LuaResult,
    Table, Value, Variadic,
};

use crate::pcre;

/// A compiled pattern. `owned` is false when borrowed from a precompiled
/// options table, whose `__gc` frees it.
struct Pattern {
    code: *mut pcre::Pcre2Code,
    owned: bool,
}

impl Pattern {
    fn free_if_owned(&self) {
        if self.owned && !self.code.is_null() {
            unsafe { pcre::pcre2_code_free_8(self.code) };
        }
    }
}

fn free_match_data(md: *mut pcre::Pcre2MatchData) {
    if !md.is_null() {
        unsafe { pcre::pcre2_match_data_free_8(md) };
    }
}

/// `luaL_checklstring`: accepts strings and numbers. The message matches
/// Lua's own `luaL_argerror` text so find/replace shows the same error as the C
/// core (mlua still wraps it, adding "runtime error: " and a traceback).
fn coerce_string(lua: &Lua, v: &Value, arg: usize, fname: &str) -> LuaResult<LuaString> {
    lua.coerce_string(v.clone())?
        .ok_or_else(|| argerror(fname, arg, "string", v.type_name()))
}

fn argerror(fname: &str, arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!(
        "bad argument #{} to '{}' ({} expected, got {})",
        arg, fname, expected, got
    ))
}

fn opt_number(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Integer(i)) => *i as f64,
        Some(Value::Number(n)) => *n,
        _ => 0.0,
    }
}

fn compile_pattern(bytes: &[u8], options: u32) -> Result<*mut pcre::Pcre2Code, (usize, String)> {
    let mut errcode: c_int = 0;
    let mut erroffset: pcre::Size = 0;
    let re = unsafe {
        pcre::pcre2_compile_8(
            bytes.as_ptr(),
            bytes.len(),
            options,
            &mut errcode,
            &mut erroffset,
            std::ptr::null_mut(),
        )
    };
    if re.is_null() {
        return Err((erroffset, pcre::error_message(errcode)));
    }
    unsafe { pcre::pcre2_jit_compile_8(re, pcre::PCRE2_JIT_COMPLETE) };
    Ok(re)
}

/// First argument: either a precompiled table or a pattern string.
fn pattern_arg(lua: &Lua, v: &Value, fname: &str) -> LuaResult<Pattern> {
    if let Value::Table(t) = v {
        return match t.raw_get::<Value>(1)? {
            Value::LightUserData(LightUserData(p)) if !p.is_null() => Ok(Pattern {
                code: p as *mut pcre::Pcre2Code,
                owned: false,
            }),
            _ => Err(LuaError::runtime("regex: invalid compiled pattern")),
        };
    }
    let s = coerce_string(lua, v, 1, fname)?;
    match compile_pattern(&s.as_bytes(), pcre::PCRE2_UTF) {
        Ok(code) => Ok(Pattern {
            code,
            owned: true,
        }),
        Err((offset, msg)) => Err(LuaError::runtime(format!(
            "regex pattern error at offset {}: {}",
            offset, msg
        ))),
    }
}

/// `luaL_setmetatable(L, "regex")`: compiled patterns are plain tables.
fn pattern_table(lua: &Lua, module: &Table, code: *mut pcre::Pcre2Code) -> LuaResult<Table> {
    let t = lua.create_table()?;
    t.raw_set(1, LightUserData(code as *mut _))?;
    t.set_metatable(Some(module.clone()))?;
    Ok(t)
}

/// `regex_offset_relative`: Lua-style 1-based offsets, negatives from the end.
fn offset_relative(pos: f64, len: usize) -> usize {
    if pos > 0.0 {
        pos as usize
    } else if pos == 0.0 {
        1
    } else if pos < -(len as f64) {
        1
    } else {
        (len as f64 + pos + 1.0) as usize
    }
}

fn match_data_for(code: *mut pcre::Pcre2Code) -> LuaResult<*mut pcre::Pcre2MatchData> {
    let md = unsafe { pcre::pcre2_match_data_create_from_pattern_8(code, std::ptr::null_mut()) };
    if md.is_null() {
        Err(LuaError::runtime("regex: out of memory"))
    } else {
        Ok(md)
    }
}

fn matching_error(rc: c_int) -> LuaError {
    LuaError::runtime(format!(
        "regex matching error {}: {}",
        rc,
        pcre::error_message(rc)
    ))
}

fn k_assertion_error() -> LuaError {
    LuaError::runtime(
        "regex matching error: \\K was used in an assertion to  set the match start after its end",
    )
}

/// State of a `gmatch` iterator. The C version keeps the subject alive in the
/// closure's upvalues; here `BorrowedBytes` anchors it.
struct Gmatch {
    code: *mut pcre::Pcre2Code,
    owned: bool,
    md: *mut pcre::Pcre2MatchData,
    subject: BorrowedBytes,
    offset: usize,
    found: bool,
}

impl Drop for Gmatch {
    fn drop(&mut self) {
        free_match_data(self.md);
        if self.owned {
            unsafe { pcre::pcre2_code_free_8(self.code) };
        }
    }
}

pub fn luaopen_regex(lua: &Lua) -> LuaResult<Table> {
    let module = lua.create_table()?;

    // compile(pattern, options?) -> pattern | nil, errmsg
    let m = module.clone();
    module.set(
        "compile",
        lua.create_function(move |lua, args: Variadic<Value>| {
            let pattern = args.first().ok_or_else(|| {
                LuaError::runtime("bad argument #1 to 'compile' (string expected, got no value)")
            })?;
            let s = coerce_string(lua, pattern, 1, "regex.compile")?;
            let mut flags = pcre::PCRE2_UTF;
            if let Some(options) = args.get(1) {
                let os = coerce_string(lua, options, 2, "regex.compile")?;
                let bytes = os.as_bytes();
                if bytes.contains(&b'i') {
                    flags |= pcre::PCRE2_CASELESS;
                }
                if bytes.contains(&b'm') {
                    flags |= pcre::PCRE2_MULTILINE;
                }
                if bytes.contains(&b's') {
                    flags |= pcre::PCRE2_DOTALL;
                }
            }
            match compile_pattern(&s.as_bytes(), flags) {
                Ok(code) => Ok(MultiValue::from_vec(vec![Value::Table(pattern_table(
                    lua, &m, code,
                )?)])),
                Err((offset, msg)) => Ok(MultiValue::from_vec(vec![
                    Value::Nil,
                    Value::String(lua.create_string(format!(
                        "regex compilation failed at offset {}: {}",
                        offset, msg
                    ))?),
                ])),
            }
        })?,
    )?;

    // cmatch(pattern, subject, offset?, options?) -> start,end,... (1-based)
    module.set(
        "cmatch",
        lua.create_function(|lua, args: Variadic<Value>| {
            let pattern = args
                .first()
                .ok_or_else(|| LuaError::runtime("bad argument #1 (pattern expected)"))?;
            let subject = args
                .get(1)
                .ok_or_else(|| LuaError::runtime("bad argument #2 (string expected)"))?;
            let p = pattern_arg(lua, pattern, "regex.cmatch")?;
            let subject = coerce_string(lua, subject, 2, "regex.cmatch")?;
            let bytes = subject.as_bytes();
            let subject_len = bytes.len();

            let mut start = match args.get(2) {
                Some(v) => offset_relative(opt_number(Some(v)), subject_len),
                None => 1,
            };
            start -= 1;
            let len = subject_len.saturating_sub(start);
            let options = opt_number(args.get(3)) as u32;

            let md = match match_data_for(p.code) {
                Ok(md) => md,
                Err(e) => {
                    p.free_if_owned();
                    return Err(e);
                }
            };
            let rc = unsafe {
                pcre::pcre2_match_8(
                    p.code,
                    bytes.as_ptr().add(start),
                    len,
                    0,
                    options,
                    md,
                    std::ptr::null_mut(),
                )
            };
            if rc < 0 {
                p.free_if_owned();
                free_match_data(md);
                if rc != pcre::PCRE2_ERROR_NOMATCH {
                    return Err(matching_error(rc));
                }
                return Ok(MultiValue::new());
            }
            let ov = unsafe { pcre::pcre2_get_ovector_pointer_8(md) };
            if unsafe { *ov } > unsafe { *ov.add(1) } {
                p.free_if_owned();
                free_match_data(md);
                return Err(k_assertion_error());
            }
            let mut values = Vec::with_capacity((rc * 2) as usize);
            for i in 0..(rc * 2) as usize {
                let off = unsafe { *ov.add(i) };
                values.push(Value::Integer((off + start + 1) as i64));
            }
            p.free_if_owned();
            free_match_data(md);
            Ok(MultiValue::from_vec(values))
        })?,
    )?;

    // gmatch(pattern, subject, offset?) -> iterator
    module.set(
        "gmatch",
        lua.create_function(|lua, args: Variadic<Value>| {
            let pattern = args
                .first()
                .ok_or_else(|| LuaError::runtime("bad argument #1 (pattern expected)"))?;
            let subject = args
                .get(1)
                .ok_or_else(|| LuaError::runtime("bad argument #2 (string expected)"))?;
            let p = pattern_arg(lua, pattern, "regex.gmatch")?;
            let subject = coerce_string(lua, subject, 2, "regex.gmatch")?;
            let subject = subject.as_bytes();
            let offset = match args.get(2) {
                Some(v) => offset_relative(opt_number(Some(v)), subject.len()),
                None => 1,
            } - 1;
            let md = match match_data_for(p.code) {
                Ok(md) => md,
                Err(e) => {
                    p.free_if_owned();
                    return Err(e);
                }
            };
            let state = Rc::new(RefCell::new(Gmatch {
                code: p.code,
                owned: p.owned,
                md,
                subject,
                offset,
                found: true,
            }));
            lua.create_function(move |lua, ()| {
                let mut st = state.borrow_mut();
                if !st.found {
                    return Ok(MultiValue::new());
                }
                let rc = unsafe {
                    pcre::pcre2_match_8(
                        st.code,
                        st.subject.as_ptr(),
                        st.subject.len(),
                        st.offset,
                        0,
                        st.md,
                        std::ptr::null_mut(),
                    )
                };
                if rc < 0 {
                    if rc != pcre::PCRE2_ERROR_NOMATCH {
                        return Err(matching_error(rc));
                    }
                    st.found = false;
                    return Ok(MultiValue::new());
                }
                let ovector_count = unsafe { pcre::pcre2_get_ovector_count_8(st.md) };
                if ovector_count == 0 {
                    st.found = false;
                    return Ok(MultiValue::new());
                }
                let ov = unsafe { pcre::pcre2_get_ovector_pointer_8(st.md) };
                let (o0, o1) = unsafe { (*ov, *ov.add(1)) };
                if o0 > o1 {
                    return Err(k_assertion_error());
                }
                // With captures, skip the whole match and yield groups only.
                let index = if ovector_count > 1 { 2 } else { 0 };
                let total_results = ovector_count as usize * 2;
                let mut last_offset = 0usize;
                let mut values = Vec::new();
                for i in (index..total_results).step_by(2) {
                    let (a, b) = unsafe { (*ov.add(i), *ov.add(i + 1)) };
                    if a == b {
                        values.push(Value::Integer((a + 1) as i64));
                    } else {
                        values.push(Value::String(lua.create_string(&st.subject[a..b])?));
                    }
                    last_offset = b;
                }
                if last_offset.wrapping_sub(1) < st.subject.len() {
                    st.offset = last_offset;
                } else {
                    st.found = false;
                }
                Ok(MultiValue::from_vec(values))
            })
        })?,
    )?;

    // gsub(pattern, subject, replacement, limit?) -> string, count
    module.set(
        "gsub",
        lua.create_function(|lua, args: Variadic<Value>| {
            let pattern = args
                .first()
                .ok_or_else(|| LuaError::runtime("bad argument #1 (pattern expected)"))?;
            let subject = args
                .get(1)
                .ok_or_else(|| LuaError::runtime("bad argument #2 (string expected)"))?;
            let replacement = args
                .get(2)
                .ok_or_else(|| LuaError::runtime("bad argument #3 (string expected)"))?;
            let p = pattern_arg(lua, pattern, "regex.gsub")?;
            let subject = coerce_string(lua, subject, 2, "regex.gsub")?;
            let replacement = coerce_string(lua, replacement, 3, "regex.gsub")?;
            let repl = replacement.as_bytes();
            let limit = (opt_number(args.get(3)) as i64).max(0) as usize;

            let md = match match_data_for(p.code) {
                Ok(md) => md,
                Err(e) => {
                    p.free_if_owned();
                    return Err(e);
                }
            };

            // The C code hands PCRE2 a raw buffer and juggles ownership by
            // hand. Owning the subject up front keeps the same control flow
            // without the manual frees.
            let mut subject: Vec<u8> = subject.as_bytes().to_vec();
            let mut buffer_size: usize = 1024;
            let mut output: Vec<u8> = vec![0; buffer_size];
            let mut options =
                pcre::PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | pcre::PCRE2_SUBSTITUTE_EXTENDED;
            if limit == 0 {
                options |= pcre::PCRE2_SUBSTITUTE_GLOBAL;
            }

            let mut results_count: i64;
            let final_len: usize;
            let mut offset: usize = 0;
            let mut limit_count: i64 = 0;
            let mut done = false;

            loop {
                let mut outlen: usize = buffer_size;
                let rc = unsafe {
                    pcre::pcre2_substitute_8(
                        p.code,
                        subject.as_ptr(),
                        subject.len(),
                        offset,
                        options,
                        md,
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        output.as_mut_ptr(),
                        &mut outlen,
                    )
                };
                results_count = rc as i64;

                if rc != pcre::PCRE2_ERROR_NOMEMORY || buffer_size >= outlen {
                    if limit == 0 || rc < 0 {
                        // Global path, or a real error reported below.
                        done = true;
                    } else {
                        let ovector_count = unsafe { pcre::pcre2_get_ovector_count_8(md) };
                        if rc > 0 && ovector_count > 0 {
                            limit_count += 1;
                            let ov = unsafe { pcre::pcre2_get_ovector_pointer_8(md) };
                            let end = unsafe { *ov.add(1) };
                            offset = if outlen > subject.len() {
                                end.wrapping_add(outlen - subject.len())
                            } else {
                                end.wrapping_sub(subject.len() - outlen)
                            };
                            if limit_count as usize == limit || offset.wrapping_sub(1) == outlen {
                                done = true;
                                results_count = limit_count;
                            } else {
                                // The C code keeps the substituted subject and
                                // its length in separate variables; truncating
                                // to the written length is the same thing.
                                subject = std::mem::take(&mut output);
                                subject.truncate(outlen);
                                output = vec![0; buffer_size];
                            }
                        } else {
                            done = true;
                            results_count = limit_count;
                        }
                    }
                } else {
                    buffer_size = outlen;
                    output.resize(buffer_size, 0);
                }
                if done {
                    final_len = outlen;
                    break;
                }
            }

            let result = if results_count > 0 {
                (
                    Value::String(lua.create_string(&output[..final_len])?),
                    Value::Integer(results_count),
                )
            } else if results_count == 0 {
                (
                    Value::String(lua.create_string(&subject[..])?),
                    Value::Integer(0),
                )
            } else {
                free_match_data(md);
                p.free_if_owned();
                return Err(LuaError::runtime(format!(
                    "regex substitute error: {}",
                    pcre::error_message(results_count as c_int)
                )));
            };

            free_match_data(md);
            p.free_if_owned();
            Ok(MultiValue::from_vec(vec![result.0, result.1]))
        })?,
    )?;

    // Frees the compiled pattern held in field 1 of a precompiled table.
    module.set(
        "__gc",
        lua.create_function(|_, t: Table| {
            if let Value::LightUserData(LightUserData(p)) = t.raw_get::<Value>(1)? {
                if !p.is_null() {
                    unsafe { pcre::pcre2_code_free_8(p as *mut pcre::Pcre2Code) };
                }
            }
            Ok(())
        })?,
    )?;

    // PCRE2_ANCHORED is intentionally used for ENDANCHORED too: the C core has
    // this long-standing quirk and Lua code may rely on the value.
    // shortcut: ENDANCHORED aliases ANCHORED, fix only if PCRE2_ENDANCHORED is wanted.
    for (name, value) in [
        ("ANCHORED", pcre::PCRE2_ANCHORED),
        ("ENDANCHORED", pcre::PCRE2_ANCHORED),
        ("NOTBOL", pcre::PCRE2_NOTBOL),
        ("NOTEOL", pcre::PCRE2_NOTEOL),
        ("NOTEMPTY", pcre::PCRE2_NOTEMPTY),
        ("NOTEMPTY_ATSTART", pcre::PCRE2_NOTEMPTY_ATSTART),
    ] {
        module.set(name, value as i64)?;
    }

    module.set("__name", "regex")?;
    Ok(module)
}
