//! `regex`: pattern matching backed by the `fancy-regex` crate.
//!
//! The Lua-visible contract matches the previous PCRE2-backed module: `compile`
//! returns a pattern *table* whose metatable is this module's table (so
//! `pattern:gsub(...)` works through `data/core/regex.lua`), `cmatch`/`gmatch`/
//! `gsub` accept either that table or a pattern string, and only the `ANCHORED`
//! match option is available.
//!
//! The replacement dialect is fancy-regex' native `$name`/`${name}` expansion,
//! not PCRE2's extended one. Unknown or malformed references expand to the
//! empty string or their literal text (follow `Expander`'s runtime behavior;
//! there is no separate template validation).
//!
//! Backtracking is limited per match attempt and substitution bytes are capped
//! through `set_limits`; neither setting is a deadline or total-operation budget.

use std::borrow::Cow;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use fancy_regex::{
    Captures, CompileError, Error as RegexError, Expander, Regex, RegexBuilder, RegexInput,
    RuntimeError,
};
use mlua::{
    Error as LuaError, Lua, LuaString, MultiValue, Result as LuaResult, Table, UserData, Value,
    Variadic,
};

/// `PCRE2_ANCHORED`'s value, the only accepted match option.
const ANCHORED: u32 = 0x8000_0000;

/// Approximate program/cache size settings per delegated expression, not a process cap.
const DELEGATE_SIZE_LIMIT: usize = 2 * 1024 * 1024;

/// A compiled pattern plus its two compiled forms: the plain form, and a
/// `find_not_empty` form used to retry at the same position after an empty
/// match. Both are rebuilt if the backtrack limit changes.
struct Pattern {
    source: String,
    flags: String,
    pair: RefCell<Pair>,
}

struct Pair {
    backtrack: usize,
    re: Regex,
    nonempty: Option<Option<Regex>>,
}

/// Private userdata owning the compiled pattern; the Lua GC is the only way to
/// free it, so no `__gc` method is needed.
struct Handle(Rc<Pattern>);

impl UserData for Handle {}

/// Immutable per-call resource limits. Iterators snapshot these on creation.
#[derive(Clone, Copy)]
struct Limits {
    backtrack: usize,
    output: usize,
}

/// `luaL_checklstring`-style coercion: strings and numbers, never `nil`.
fn lua_string(lua: &Lua, v: Option<&Value>) -> LuaResult<LuaString> {
    lua.coerce_string(v.cloned().unwrap_or(Value::Nil))?
        .ok_or_else(|| LuaError::runtime("string expected"))
}

/// Non-lossy view of a subject/pattern/replacement; invalid UTF-8 is a safe
/// error rather than a partial or byte-wise match.
fn utf8(s: &LuaString) -> LuaResult<mlua::BorrowedStr> {
    s.to_str()
        .map_err(|_| LuaError::runtime("REGEX_UTF8_ERROR: invalid UTF-8"))
}

fn opt_number(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Integer(i)) => *i as f64,
        Some(Value::Number(n)) => *n,
        _ => 0.0,
    }
}

/// `regex_offset_relative`: 1-based Lua offsets to a 0-based byte position.
/// Zero and overly negative offsets map to 1; other negatives count back from
/// the byte length. The result must be a real UTF-8 boundary.
fn offset_relative(v: Option<&Value>, text: &str) -> LuaResult<usize> {
    let n = opt_number(v);
    if !n.is_finite() {
        return Err(LuaError::runtime("REGEX_OFFSET_ERROR: non-finite offset"));
    }
    let one = if n > 0.0 {
        n as usize
    } else if n == 0.0 || n < -(text.len() as f64) {
        1
    } else {
        (text.len() as f64 + n + 1.0) as usize
    };
    let pos = one.saturating_sub(1);
    if pos > text.len() || !text.is_char_boundary(pos) {
        return Err(LuaError::runtime(
            "REGEX_OFFSET_ERROR: offset beyond end or inside UTF-8",
        ));
    }
    Ok(pos)
}

/// Only `ANCHORED` (and its `ENDANCHORED` alias) is supported. Any other bit is
/// an explicit error instead of silently changing assertion behavior.
fn matching_anchored(v: Option<&Value>) -> LuaResult<bool> {
    let options = opt_number(v) as u32;
    if options & !ANCHORED != 0 {
        return Err(LuaError::runtime(
            "REGEX_OPTIONS_ERROR: unsupported match options (only ANCHORED/ENDANCHORED)",
        ));
    }
    Ok(options & ANCHORED != 0)
}

/// Assertion overrides stay disabled, so `\A` and `\z` remain hard anchors.
fn build(source: &str, flags: &str, backtrack: usize, nonempty: bool) -> Result<Regex, RegexError> {
    RegexBuilder::new(source)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .backtrack_limit(backtrack)
        .delegate_size_limit(DELEGATE_SIZE_LIMIT)
        .delegate_dfa_size_limit(DELEGATE_SIZE_LIMIT)
        .find_not_empty(nonempty)
        .build()
}

fn compile(source: &str, flags: &str, backtrack: usize) -> Result<Rc<Pattern>, RegexError> {
    let re = build(source, flags, backtrack, false)?;
    Ok(Rc::new(Pattern {
        source: source.to_owned(),
        flags: flags.to_owned(),
        pair: RefCell::new(Pair {
            backtrack,
            re,
            nonempty: None,
        }),
    }))
}

fn engine_error(e: RegexError) -> LuaError {
    match e {
        RegexError::RuntimeError(RuntimeError::BacktrackLimitExceeded) => {
            LuaError::runtime("REGEX_WORK_LIMIT: backtrack limit exceeded")
        }
        _ => LuaError::runtime(format!("REGEX_ENGINE_ERROR: {e}")),
    }
}

/// Find captures and return them with only a borrow of `text`. The returned
/// `Captures` owns its spans and named-group table, borrows neither the regex
/// nor the `RefCell`, so an `Expander` can consume it after the borrow drops.
fn find_captures<'t>(
    p: &Pattern,
    text: &'t str,
    pos: usize,
    anchored: bool,
    nonempty: bool,
    backtrack: usize,
) -> LuaResult<Option<Captures<'t, str>>> {
    let mut pair = p.pair.borrow_mut();
    if pair.backtrack != backtrack {
        pair.re = build(&p.source, &p.flags, backtrack, false).map_err(engine_error)?;
        pair.backtrack = backtrack;
        pair.nonempty = None;
    }
    if nonempty && pair.nonempty.is_none() {
        pair.nonempty = Some(match build(&p.source, &p.flags, backtrack, true) {
            Ok(re) => Some(re),
            Err(RegexError::CompileError(e))
                if matches!(e.as_ref(), CompileError::PatternCanNeverMatch) =>
            {
                None
            }
            Err(e) => return Err(engine_error(e)),
        });
    }
    let re = if nonempty {
        match pair.nonempty.as_ref().expect("nonempty form built above") {
            Some(re) => re,
            None => return Ok(None),
        }
    } else {
        &pair.re
    };
    let input = RegexInput::new(text).from_pos(pos).anchored(anchored);
    re.captures_input(input).map_err(engine_error)
}

/// Advance one UTF-8 character; `None` means the end was reached.
fn advance(text: &str, pos: usize) -> Option<usize> {
    text.get(pos..)?.chars().next().map(|c| pos + c.len_utf8())
}

/// gmatch/gsub step that makes empty matches progress: after an empty match at
/// `pos`, retry there with a non-empty form, otherwise skip one character.
fn next_match<'t>(
    p: &Pattern,
    text: &'t str,
    pos: &mut usize,
    retry: &mut bool,
    backtrack: usize,
) -> LuaResult<Option<Captures<'t, str>>> {
    if *retry {
        if let Some(c) = find_captures(p, text, *pos, true, true, backtrack)? {
            *retry = false;
            return Ok(Some(c));
        }
        match advance(text, *pos) {
            Some(next) => *pos = next,
            None => return Ok(None),
        }
        *retry = false;
    }
    find_captures(p, text, *pos, false, false, backtrack)
}

/// gmatch yields groups only when the pattern has captures; a hole before a
/// later group becomes `nil` and trailing unmatched groups are omitted.
fn capture_strings(lua: &Lua, c: &Captures<str>) -> LuaResult<MultiValue> {
    let len = c.len();
    let end = (0..len)
        .rposition(|i| c.get(i).is_some())
        .map_or(0, |n| n + 1);
    let start = if len > 1 { 1 } else { 0 };
    let mut out = Vec::with_capacity(end.saturating_sub(start));
    for i in start..end {
        match c.get(i) {
            None => out.push(Value::Nil),
            Some(m) if m.start() == m.end() => out.push(Value::Integer((m.start() + 1) as i64)),
            Some(m) => out.push(Value::String(lua.create_string(m.as_str())?)),
        }
    }
    Ok(MultiValue::from_vec(out))
}

fn output_limit() -> LuaError {
    LuaError::runtime("REGEX_OUTPUT_LIMIT: output byte quota exhausted")
}

/// Byte-capped `io::Write` sink. Substitution writes straight through it, so
/// no unbounded intermediate expansion is ever materialized; exceeding the cap
/// raises `REGEX_OUTPUT_LIMIT` with nothing partially returned.
struct CappedSink {
    buf: Vec<u8>,
    max: usize,
}

impl CappedSink {
    fn new(max: usize) -> Self {
        Self {
            buf: Vec::new(),
            max,
        }
    }

    fn write_str(&mut self, s: &str) -> LuaResult<()> {
        self.write_all(s.as_bytes()).map_err(|_| output_limit())
    }

    fn len(&self) -> usize {
        self.buf.len()
    }

    fn finish(self) -> Vec<u8> {
        self.buf
    }
}

impl Write for CappedSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if data.len() > self.max.saturating_sub(self.buf.len()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "output limit",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn expand(sink: &mut CappedSink, replacement: &str, captures: &Captures<str>) -> LuaResult<()> {
    Expander::default()
        .write_expansion(&mut *sink, replacement, captures)
        .map_err(|_| output_limit())
}

/// Substitute `replacement` for every match, returning the result bytes and the
/// substitution count. A positive `limit` rebuilds the subject at each step
/// (which reproduces the historical `x*` limit-2 quirk `--abc`); the global
/// path matches the original subject and makes empty matches progress.
fn substitute(
    p: &Pattern,
    subject: &str,
    replacement: &str,
    limit: usize,
    limits: Limits,
) -> LuaResult<(Vec<u8>, usize)> {
    if limit > 0 {
        let mut current = Cow::Borrowed(subject);
        let mut pos = 0;
        let mut count = 0;
        while count < limit && pos <= current.len() {
            let (start, part_len, next) = {
                let Some(c) = find_captures(p, &current, pos, false, false, limits.backtrack)?
                else {
                    break;
                };
                let m = c.get(0).expect("match 0 is always present");
                let mut sink = CappedSink::new(limits.output);
                sink.write_str(&current[..m.start()])?;
                let before = sink.len();
                expand(&mut sink, replacement, &c)?;
                let part_len = sink.len() - before;
                sink.write_str(&current[m.end()..])?;
                (m.start(), part_len, sink.finish())
            };
            pos = start + part_len;
            current = Cow::Owned(String::from_utf8(next).expect("substitution output is UTF-8"));
            count += 1;
        }
        if current.len() > limits.output {
            return Err(output_limit());
        }
        return Ok((current.into_owned().into_bytes(), count));
    }

    let mut sink = CappedSink::new(limits.output);
    let mut pos = 0;
    let mut last = 0;
    let mut retry = false;
    let mut count = 0;
    while let Some(c) = next_match(p, subject, &mut pos, &mut retry, limits.backtrack)? {
        let m = c.get(0).expect("match 0 is always present");
        let (start, end) = (m.start(), m.end());
        sink.write_str(&subject[last..start])?;
        expand(&mut sink, replacement, &c)?;
        count += 1;
        last = end;
        pos = end;
        retry = start == end;
    }
    sink.write_str(&subject[last..])?;
    Ok((sink.finish(), count))
}

/// First argument: either a compiled pattern table or a pattern string.
fn pattern_arg(lua: &Lua, v: Option<&Value>, backtrack: usize) -> LuaResult<Rc<Pattern>> {
    if let Some(Value::Table(t)) = v {
        return match t.raw_get::<Value>(1)? {
            Value::UserData(ud) => ud
                .borrow::<Handle>()
                .map(|h| h.0.clone())
                .map_err(|_| LuaError::runtime("regex: invalid compiled pattern")),
            _ => Err(LuaError::runtime("regex: invalid compiled pattern")),
        };
    }
    let raw = lua_string(lua, v)?;
    let source = utf8(&raw)?;
    compile(&source, "", backtrack)
        .map_err(|e| LuaError::runtime(format!("regex pattern error: {e}")))
}

pub fn luaopen_regex(lua: &Lua) -> LuaResult<Table> {
    let module = lua.create_table()?;

    // shortcut: accounting is per match attempt (backtrack limit) and per
    // substitution result (output cap); there is no aggregate call counter or
    // wall-clock deadline. Add one if a caller needs a total-work budget.
    let limits = Rc::new(RefCell::new(Limits {
        backtrack: 1_000_000,
        output: 8 * 1024 * 1024,
    }));

    let l = limits.clone();
    module.set(
        "set_limits",
        lua.create_function(move |_, (backtrack, output): (usize, usize)| {
            if backtrack == 0 || output == 0 {
                return Err(LuaError::runtime(
                    "REGEX_LIMIT_ERROR: positive limits required",
                ));
            }
            *l.borrow_mut() = Limits { backtrack, output };
            Ok(())
        })?,
    )?;

    // compile(pattern, flags?) -> pattern | nil, errmsg
    let m = module.clone();
    let l = limits.clone();
    module.set(
        "compile",
        lua.create_function(move |lua, args: Variadic<Value>| {
            let raw = lua_string(lua, args.first())?;
            let source = match utf8(&raw) {
                Ok(s) => s,
                Err(_) => {
                    return Ok(MultiValue::from_vec(vec![
                        Value::Nil,
                        Value::String(
                            lua.create_string("regex compilation failed: invalid UTF-8")?,
                        ),
                    ]))
                }
            };
            let flags = match args.get(1) {
                Some(v) => {
                    let f = lua_string(lua, Some(v))?;
                    ['i', 'm', 's']
                        .into_iter()
                        .filter(|c| f.as_bytes().contains(&(*c as u8)))
                        .collect()
                }
                None => String::new(),
            };
            match compile(&source, &flags, l.borrow().backtrack) {
                Ok(p) => {
                    let t = lua.create_table()?;
                    t.raw_set(1, lua.create_userdata(Handle(p))?)?;
                    t.set_metatable(Some(m.clone()))?;
                    Ok(MultiValue::from_vec(vec![Value::Table(t)]))
                }
                Err(e) => Ok(MultiValue::from_vec(vec![
                    Value::Nil,
                    Value::String(lua.create_string(format!("regex compilation failed: {e}"))?),
                ])),
            }
        })?,
    )?;

    // cmatch(pattern, subject, offset?, options?) -> start,end,... (1-based)
    let l = limits.clone();
    module.set(
        "cmatch",
        lua.create_function(move |lua, args: Variadic<Value>| {
            let limits = *l.borrow();
            let p = pattern_arg(lua, args.first(), limits.backtrack)?;
            let raw = lua_string(lua, args.get(1))?;
            let subject = utf8(&raw)?;
            let subject: &str = &subject;
            let start = offset_relative(args.get(2), subject)?;
            let anchored = matching_anchored(args.get(3))?;
            // cmatch is suffix-relative: the offset becomes the new subject
            // start, so anchors and lookbehind cannot see the prefix.
            let Some(c) =
                find_captures(&p, &subject[start..], 0, anchored, false, limits.backtrack)?
            else {
                return Ok(MultiValue::new());
            };
            let len = c.len();
            let end = (0..len)
                .rposition(|i| c.get(i).is_some())
                .map_or(0, |n| n + 1);
            let mut values = Vec::with_capacity(end * 2);
            for i in 0..end {
                match c.get(i) {
                    Some(m) => {
                        values.push(Value::Integer((start + m.start() + 1) as i64));
                        values.push(Value::Integer((start + m.end() + 1) as i64));
                    }
                    None => {
                        values.push(Value::Nil);
                        values.push(Value::Nil);
                    }
                }
            }
            Ok(MultiValue::from_vec(values))
        })?,
    )?;

    // gmatch(pattern, subject, offset?) -> iterator
    let l = limits.clone();
    module.set(
        "gmatch",
        lua.create_function(move |lua, args: Variadic<Value>| {
            let limits = *l.borrow();
            let p = pattern_arg(lua, args.first(), limits.backtrack)?;
            let raw = lua_string(lua, args.get(1))?;
            let subject = utf8(&raw)?.to_string();
            let start = offset_relative(args.get(2), &subject)?;
            // gmatch searches the whole subject from `start` (unlike cmatch),
            // so a leading anchor can still see the prefix.
            let state = Rc::new(RefCell::new((start, false, false)));
            lua.create_function(move |lua, ()| {
                let mut st = state.borrow_mut();
                if st.2 {
                    return Ok(MultiValue::new());
                }
                let (pos, retry, done) = &mut *st;
                let Some(c) = next_match(&p, &subject, pos, retry, limits.backtrack)? else {
                    *done = true;
                    return Ok(MultiValue::new());
                };
                let m = c.get(0).expect("match 0 is always present");
                (*pos, *retry) = (m.end(), m.start() == m.end());
                capture_strings(lua, &c)
            })
        })?,
    )?;

    // gsub(pattern, subject, replacement, limit?) -> string, count
    let l = limits.clone();
    module.set(
        "gsub",
        lua.create_function(move |lua, args: Variadic<Value>| {
            let limits = *l.borrow();
            let p = pattern_arg(lua, args.first(), limits.backtrack)?;
            let raw = lua_string(lua, args.get(1))?;
            let subject = utf8(&raw)?;
            let raw_repl = lua_string(lua, args.get(2))?;
            let replacement = utf8(&raw_repl)?;
            let limit = (opt_number(args.get(3)) as i64).max(0) as usize;
            let (out, count) = substitute(&p, &subject, &replacement, limit, limits)?;
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(out)?),
                Value::Integer(count as i64),
            ]))
        })?,
    )?;

    // ENDANCHORED remains an alias of ANCHORED, a long-standing quirk bundled
    // callers may depend on.
    // shortcut: ENDANCHORED aliases ANCHORED; split them only if end anchoring is wanted.
    for (name, value) in [
        ("ANCHORED", ANCHORED as i64),
        ("ENDANCHORED", ANCHORED as i64),
    ] {
        module.set(name, value)?;
    }

    module.set("__name", "regex")?;
    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::FromLuaMulti;

    fn lua() -> Lua {
        let lua = Lua::new();
        let module = luaopen_regex(&lua).unwrap();
        lua.globals().set("regex", module).unwrap();
        lua
    }

    fn eval<T: FromLuaMulti>(lua: &Lua, src: &str) -> T {
        lua.load(src).eval().unwrap()
    }

    #[test]
    fn native_dollar_expansion() {
        let lua = lua();
        let (out, n): (String, i64) =
            eval(&lua, r#"return regex.gsub("a(b)c", "=abc=", "+$1$0$1+")"#);
        assert_eq!((out.as_str(), n), ("=+babcb+=", 1));
        let (out, n): (String, i64) =
            eval(&lua, r#"return regex.gsub("(?<word>a)", "a", "<${word}>")"#);
        assert_eq!((out.as_str(), n), ("<a>", 1));
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("(a)", "a", "$$$1")"#);
        assert_eq!((out.as_str(), n), ("$a", 1));
    }

    #[test]
    fn unknown_refs_follow_native_behavior() {
        let lua = lua();
        // Unknown numbered group -> empty.
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("(a)", "a", "$9")"#);
        assert_eq!((out.as_str(), n), ("", 1));
        // Trailing `$` with nothing to reference -> literal.
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("a", "a", "x$")"#);
        assert_eq!((out.as_str(), n), ("x$", 1));
    }

    #[test]
    fn no_pcre_backslash_case_conversion() {
        let lua = lua();
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("(a)", "a", [[\U$1\E]])"#);
        assert_eq!((out.as_str(), n), (r"\Ua\E", 1));
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("(a)", "a", [[\n]])"#);
        assert_eq!((out.as_str(), n), (r"\n", 1));
    }

    #[test]
    fn output_limit_is_explicit_and_atomic() {
        let lua = lua();
        eval::<()>(&lua, r#"regex.set_limits(1000000, 4)"#);
        let (ok, msg): (bool, String) = eval(
            &lua,
            r#"local ok, err = pcall(regex.gsub, "a", "aaaa", "bbbb"); return ok, tostring(err)"#,
        );
        assert!(!ok);
        assert!(msg.contains("REGEX_OUTPUT_LIMIT"), "{msg}");
        // The limited no-match path must enforce the same result-byte cap.
        let ok: bool = eval(&lua, r#"return (pcall(regex.gsub, "z", "aaaaa", "", 1))"#);
        assert!(!ok);
        eval::<()>(&lua, r#"regex.set_limits(1000000, 8388608)"#);
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("a", "aaaa", "bbbb")"#);
        assert_eq!((out.as_str(), n), ("bbbbbbbbbbbbbbbb", 4));
    }

    #[test]
    fn compiled_pattern_survives_gc() {
        let lua = lua();
        let (out, n): (String, i64) = eval(
            &lua,
            r#"
            local p = regex.compile("(a)")
            collectgarbage("collect")
            return regex.gsub(p, "banana", "<$1>")
            "#,
        );
        assert_eq!((out.as_str(), n), ("b<a>n<a>n<a>", 3));
        let kind: String = eval(&lua, r#"return type(regex.compile("a")[1])"#);
        assert_eq!(kind, "userdata");
        let (first, second, remaining): (String, String, i64) = eval(
            &lua,
            r##"
            local iter
            do local p = regex.compile("a"); iter = regex.gmatch(p, "aa") end
            collectgarbage("collect"); collectgarbage("collect")
            return iter(), iter(), select("#", iter())
            "##,
        );
        assert_eq!((first.as_str(), second.as_str(), remaining), ("a", "a", 0));
    }

    #[test]
    fn empty_match_progress_advances_utf8() {
        let lua = lua();
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("x*", "é", "-")"#);
        assert_eq!((out.as_str(), n), ("-é-", 2));
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("x*", "aé", "-")"#);
        assert_eq!((out.as_str(), n), ("-a-é-", 3));
        let count: i64 = eval(
            &lua,
            r#"local n = 0; for _ in regex.gmatch("x*", "aé") do n = n + 1 end; return n"#,
        );
        assert_eq!(count, 3);
    }

    #[test]
    fn positive_limit_substitution_quirk() {
        let lua = lua();
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("x*", "abc", "-", 2)"#);
        assert_eq!((out.as_str(), n), ("--abc", 2));
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("x*", "abc", "-")"#);
        assert_eq!((out.as_str(), n), ("-a-b-c-", 4));
    }

    #[test]
    fn removed_match_options_are_rejected() {
        let lua = lua();
        let missing: i64 = eval(
            &lua,
            r#"
            local n = 0
            n = n + (regex.NOTBOL == nil and 1 or 0)
            n = n + (regex.NOTEOL == nil and 1 or 0)
            n = n + (regex.NOTEMPTY == nil and 1 or 0)
            n = n + (regex.NOTEMPTY_ATSTART == nil and 1 or 0)
            return n
            "#,
        );
        assert_eq!(missing, 4);
        assert_eq!(
            eval::<bool>(&lua, r#"return regex.ENDANCHORED == regex.ANCHORED"#),
            true
        );
        let (ok, msg): (bool, String) = eval(
            &lua,
            r#"local ok, err = pcall(regex.cmatch, "a", "ba", 1, 1); return ok, tostring(err)"#,
        );
        assert!(!ok);
        assert!(msg.contains("REGEX_OPTIONS_ERROR"), "{msg}");
        // ANCHORED still constrains the match to the offset.
        let n: i64 = eval(
            &lua,
            r##"return select("#", regex.cmatch("a", "ba", 1, regex.ANCHORED))"##,
        );
        assert_eq!(n, 0);
        let (a, b): (i64, i64) = eval(&lua, r#"return regex.cmatch("a", "ba", 1)"#);
        assert_eq!((a, b), (2, 3));
    }

    #[test]
    fn set_limits_then_reuse_compiled_handle() {
        let lua = lua();
        let out: String = eval(
            &lua,
            r#"
            local p = regex.compile("a")
            regex.set_limits(500000, 1024)
            return regex.gsub(p, "aa", "x")
            "#,
        );
        assert_eq!(out, "xx");
        let (ok, msg): (bool, String) = eval(
            &lua,
            r#"local ok, err = pcall(regex.set_limits, 0, 1024); return ok, tostring(err)"#,
        );
        assert!(!ok);
        assert!(msg.contains("REGEX_LIMIT_ERROR"), "{msg}");
        let (ok, msg): (bool, String) = eval(
            &lua,
            r#"
            regex.set_limits(1000, 1024)
            local ok, err = pcall(regex.cmatch, [[(a|aa)+\1b]], string.rep("a", 1000))
            return ok, tostring(err)
            "#,
        );
        assert!(!ok);
        assert!(msg.contains("REGEX_WORK_LIMIT"), "{msg}");
    }

    #[test]
    fn input_assertions_are_hard() {
        let lua = lua();
        // Whole-subject gmatch: \A only matches the real start, not the offset.
        let count: i64 = eval(
            &lua,
            r#"local n = 0; for _ in regex.gmatch("\\Aa", "ba", 2) do n = n + 1 end; return n"#,
        );
        assert_eq!(count, 0);
        let (out, n): (String, i64) = eval(&lua, r#"return regex.gsub("\\Aa", "a", "z")"#);
        assert_eq!((out.as_str(), n), ("z", 1));
        let (a, b): (i64, i64) = eval(&lua, r#"return regex.cmatch("a\\z", "a")"#);
        assert_eq!((a, b), (1, 2));
    }

    #[test]
    fn cmatch_offsets_holes_and_trailing_omission() {
        let lua = lua();
        let (a, b): (i64, i64) = eval(&lua, r#"return regex.cmatch("a", "banana")"#);
        assert_eq!((a, b), (2, 3));
        // Trailing unmatched group is omitted.
        let n: i64 = eval(
            &lua,
            r##"return select("#", regex.cmatch("(a)(b)?", "a"))"##,
        );
        assert_eq!(n, 4);
        // A hole before a later group is nil, nil.
        let (n, hole): (i64, bool) = eval(
            &lua,
            r##"
            local _, _, _, _, e, f = regex.cmatch("(a)(b)?(c)", "ac")
            return select("#", regex.cmatch("(a)(b)?(c)", "ac")), (e == nil and f == nil)
            "##,
        );
        assert_eq!((n, hole), (8, true));
    }

    #[test]
    fn native_unicode_classes() {
        let lua = lua();
        // fancy-regex is Unicode-aware by default, unlike the old PCRE2 build.
        let (a, b): (i64, i64) = eval(&lua, r#"return regex.cmatch("\\w+", "é")"#);
        assert_eq!((a, b), (1, 3));
    }

    #[test]
    fn utf8_failures_are_safe() {
        let lua = lua();
        let (is_nil, msg_len): (bool, i64) = eval(
            &lua,
            r#"
            local p, msg = regex.compile("\255")
            return p == nil, #msg
            "#,
        );
        assert!(is_nil);
        assert!(msg_len > 0);
        let ok: bool = eval(&lua, r#"return (pcall(regex.cmatch, "a", "\255"))"#);
        assert!(!ok);
        let safe: bool = eval(
            &lua,
            r#"
            for _, args in ipairs({ { {}, "a" }, { {io.stdout}, "a" },
                                    { ".", "é", 2 }, { ".", "a", 3 } }) do
                if pcall(regex.cmatch, table.unpack(args)) then return false end
            end
            return true
        "#,
        );
        assert!(safe);
    }
}
