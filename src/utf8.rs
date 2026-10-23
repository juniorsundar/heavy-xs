//! `utf8extra`: Lua's string library reimplemented over Unicode codepoints,
//! ported from `src/api/utf8.c` (vendored starwing/luautf8).
//!
//! Two pieces: the `string`-compatible API (`len`, `sub`, `find`, `gsub`, ...)
//! and a pattern engine that works on codepoints instead of bytes. The Unicode
//! tables live in `unidata.rs`.

use std::cell::Cell;

use mlua::{
    Error as LuaError, Lua, MultiValue, Result as LuaResult, Table, Value, Variadic,
};

use crate::luaargs::{argerror, coerce_string};
use crate::unidata::*;

const UTF8_MAX: u32 = 0x7FFF_FFFF;
const UTF8_MAXCP: u32 = 0x10_FFFF;

const MAXCCALLS: i32 = 200;
const LUA_MAXCAPTURES: usize = 32;
const CAP_UNFINISHED: isize = -1;
const CAP_POSITION: isize = -2;
const L_ESC: u32 = b'%' as u32;
const SPECIALS: &[u8] = b"^$*+?.([%-";

//================================================================================
// UTF-8 codec (the C works on bytes with explicit length; positions are byte
// offsets throughout, and "character" means a codepoint whose byte length is
// derived from the lead byte, exactly like the C's utf8_next/utf8_prev).
//================================================================================

fn invalid_utf8() -> LuaError {
    LuaError::runtime("invalid UTF-8 code")
}

fn is_cont(b: &[u8], i: usize) -> bool {
    b.get(i).is_some_and(|c| c & 0xC0 == 0x80)
}

/// Port of `utf8_decode`: returns the codepoint and the index after it.
/// `strict` rejects surrogates and out-of-range codepoints.
fn decode(b: &[u8], start: usize, strict: bool) -> Option<(u32, usize)> {
    const LIMITS: [u32; 6] = [!0u32, 0x80, 0x800, 0x10000, 0x200000, 0x4000000];
    let mut c = *b.get(start)? as u32;
    let mut i = start;
    let mut res: u32 = 0;
    if c < 0x80 {
        res = c;
    } else {
        let mut count = 0usize;
        while c & 0x40 != 0 {
            count += 1;
            c <<= 1;
            let cc = *b.get(start + count)? as u32;
            if cc & 0xC0 != 0x80 {
                return None;
            }
            res = (res << 6) | (cc & 0x3F);
        }
        res |= (c & 0x7F) << (count * 5);
        if count > 5 || res > UTF8_MAX || res < LIMITS[count] {
            return None;
        }
        i = start + count;
    }
    if strict && (res > UTF8_MAXCP || (0xD800..=0xDFFF).contains(&res)) {
        return None;
    }
    Some((res, i + 1))
}

/// `utf8_safe_decode`: the C only ever uses this with `strict = 0`, so malformed
/// sequences raise, but surrogates do not.
fn safe_decode(b: &[u8], i: usize) -> LuaResult<(u32, usize)> {
    decode(b, i, false).ok_or_else(invalid_utf8)
}

/// Reads the byte at `i`, treating one past the end as the C's NUL terminator.
fn decode_at_end(b: &[u8], i: usize) -> LuaResult<(u32, usize)> {
    if i >= b.len() {
        return Ok((0, i + 1));
    }
    safe_decode(b, i)
}

/// `utf8_encode`: emits the C's variable-length (up to 6 byte) form, which is
/// what the editor uses for values above the valid range too.
fn encode(x: u32, out: &mut Vec<u8>) {
    let mut buff = [0u8; 9]; // UTF8_BUFFSZ = 8, plus one slack byte
    let mut n = 1usize;
    let mut x = x;
    let width = 8usize;
    if x < 0x80 {
        buff[width - 1] = (x & 0x7F) as u8;
    } else {
        let mut mfb: u32 = 0x3F;
        loop {
            buff[width - n] = (0x80 | (x & 0x3F)) as u8;
            n += 1;
            x >>= 6;
            mfb >>= 1;
            if x <= mfb {
                break;
            }
        }
        buff[width - n] = (((!mfb << 1) | x) & 0xFF) as u8;
    }
    out.extend_from_slice(&buff[width - n..width]);
}

/// `utf8_next`: index of the next character, or `end` if already there.
fn next_i(b: &[u8], mut i: usize, end: usize) -> usize {
    while i < end && is_cont(b, i + 1) {
        i += 1;
    }
    if i < end {
        i + 1
    } else {
        end
    }
}

/// `utf8_prev`: index of the character starting before `hi`.
fn prev_i(b: &[u8], lo: usize, mut hi: usize) -> usize {
    while lo < hi && is_cont(b, hi - 1) {
        hi -= 1;
    }
    if lo < hi {
        hi - 1
    } else {
        lo
    }
}

fn utf8_length(b: &[u8], mut i: usize, end: usize) -> i64 {
    let mut n = 0;
    while i < end {
        i = next_i(b, i, end);
        n += 1;
    }
    n
}

/// `utf8_offset`: `None` where the C returns NULL. `offset` is a byte offset,
/// and 0 means "one byte before the string" because the C computes
/// `s + offset - 1`, so a signed index is used internally.
fn utf8_offset(b: &[u8], offset: usize, idx: i64) -> Option<usize> {
    let end = b.len() as isize;
    let mut p = offset as isize - 1;
    if idx >= 0 {
        let mut idx = idx;
        while p < end && idx > 0 {
            p = next_signed(b, p, end);
            idx -= 1;
        }
        if idx == 0 {
            Some(p.max(0) as usize)
        } else {
            None
        }
    } else {
        let mut idx = idx;
        while p > 0 && idx < 0 {
            p = prev_signed(b, p);
            idx += 1;
        }
        if idx == 0 {
            Some(p.max(0) as usize)
        } else {
            None
        }
    }
}

fn is_cont_signed(b: &[u8], i: isize) -> bool {
    i >= 0 && is_cont(b, i as usize)
}

fn next_signed(b: &[u8], mut p: isize, end: isize) -> isize {
    while p < end && is_cont_signed(b, p + 1) {
        p += 1;
    }
    if p < end {
        p + 1
    } else {
        end
    }
}

fn prev_signed(b: &[u8], mut hi: isize) -> isize {
    while hi > 0 && is_cont_signed(b, hi - 1) {
        hi -= 1;
    }
    if hi > 0 {
        hi - 1
    } else {
        0
    }
}

/// `utf8_relat`. The C signature takes `int`, so callers' 64-bit Lua integers are
/// truncated to 32 bits here (`math.maxinteger` therefore acts as -1).
fn relat(b: &[u8], idx: i64) -> Option<usize> {
    let idx = idx as i32 as i64;
    let len = b.len();
    if idx >= 0 {
        utf8_offset(b, 1, idx - 1)
    } else {
        utf8_offset(b, len + 1, idx)
    }
}

/// `utf8_range`: returns the byte offsets of the first character to keep and the
/// end of the last one, plus whether the range is non-empty.
fn utf8_range(b: &[u8], i: i64, j: i64) -> (i64, i64, bool) {
    let len = b.len();
    let ps = relat(b, i);
    let pe = relat(b, j);
    let ni = match ps {
        Some(p) => p,
        None => {
            if i > 0 {
                len
            } else {
                0
            }
        }
    } as i64;
    let nj = match pe {
        Some(p) => next_i(b, p, len),
        None => {
            if j > 0 {
                len
            } else {
                0
            }
        }
    } as i64;
    (ni, nj, ni < nj)
}

/// `byte_relat`.
fn byte_relat(pos: i64, len: usize) -> i64 {
    if pos >= 0 {
        pos
    } else if (pos.unsigned_abs() as u64) > len as u64 {
        0
    } else {
        len as i64 + pos + 1
    }
}

//================================================================================
// Helpers
//================================================================================

fn typeerror(_fname: &str, _arg: usize, expected: &str, got: &str) -> LuaError {
    LuaError::runtime(format!("{} expected, got {}", expected, got))
}

//================================================================================
// Unicode character categories (src/unidata.h tables)
//================================================================================

fn find_in_range(t: RangeTable, ch: u32) -> bool {
    let (mut begin, mut end) = (0usize, t.len());
    while begin < end {
        let mid = (begin + end) / 2;
        let (first, last, step) = t[mid];
        if last < ch {
            begin = mid + 1;
        } else if first > ch {
            end = mid;
        } else {
            return (ch - first) % step as u32 == 0;
        }
    }
    false
}

fn convert_char(t: ConvTable, ch: u32) -> u32 {
    let (mut begin, mut end) = (0usize, t.len());
    while begin < end {
        let mid = (begin + end) / 2;
        let (first, last, step, offset) = t[mid];
        if last < ch {
            begin = mid + 1;
        } else if first > ch {
            end = mid;
        } else if (ch - first) % step as u32 == 0 {
            return (ch as i64 + offset as i64) as u32;
        } else {
            return ch;
        }
    }
    ch
}

macro_rules! is_category {
    ($name:ident, $table:ident) => {
        fn $name(ch: u32) -> bool {
            find_in_range($table, ch)
        }
    };
}
is_category!(is_alpha, ALPHA_TABLE);
is_category!(is_cntrl, CNTRL_TABLE);
is_category!(is_digit, DIGIT_TABLE);
is_category!(is_lower, LOWER_TABLE);
is_category!(is_punct, PUNCT_TABLE);
is_category!(is_space, SPACE_TABLE);
is_category!(is_compose, COMPOSE_TABLE);
is_category!(is_upper, UPPER_TABLE);
is_category!(is_xdigit, XDIGIT_TABLE);

fn to_lower(ch: u32) -> u32 {
    convert_char(TOLOWER_TABLE, ch)
}
fn to_fold(ch: u32) -> u32 {
    convert_char(TOFOLD_TABLE, ch)
}

/// `utf8_isgraph`.
fn is_graph(ch: u32) -> bool {
    if is_space(ch) {
        return false;
    }
    is_graph_table(ch) || is_compose(ch)
}
fn is_graph_table(ch: u32) -> bool {
    find_in_range(GRAPH_TABLE, ch)
}

/// `utf8_isalnum`.
fn is_alnum(ch: u32) -> bool {
    is_alpha(ch) || find_in_range(ALNUM_EXTEND_TABLE, ch)
}

/// `utf8_width`.
fn char_width(ch: u32, ambi_is_single: bool) -> i64 {
    if find_in_range(DOUBLEWIDTH_TABLE, ch) {
        2
    } else if find_in_range(AMBIWIDTH_TABLE, ch) {
        if ambi_is_single {
            1
        } else {
            2
        }
    } else if is_compose(ch) || find_in_range(UNPRINTABLE_TABLE, ch) {
        0
    } else {
        1
    }
}

//================================================================================
// Pattern matching (port of the C's MatchState/match and friends)
//================================================================================

struct MatchState<'a> {
    src: &'a [u8],
    src_init: usize,
    src_end: usize,
    pat: &'a [u8],
    p_end: usize,
    depth: i32,
    level: i32,
    capture: [(usize, isize); LUA_MAXCAPTURES],
}

impl<'a> MatchState<'a> {
    fn new(src: &'a [u8], pat: &'a [u8]) -> Self {
        MatchState {
            src,
            src_init: 0,
            src_end: src.len(),
            pat,
            p_end: pat.len(),
            depth: MAXCCALLS,
            level: 0,
            capture: [(0, 0); LUA_MAXCAPTURES],
        }
    }

    fn check_capture(&self, l: i64) -> LuaResult<usize> {
        // The C subtracts the *character* '1', not the number 1.
        let l = l - b'1' as i64;
        if l < 0 || l >= self.level as i64 || self.capture[l as usize].1 == CAP_UNFINISHED {
            return Err(LuaError::runtime(format!(
                "invalid capture index %{}",
                l + 1
            )));
        }
        Ok(l as usize)
    }

    fn capture_to_close(&self) -> LuaResult<usize> {
        let mut level = self.level as i64 - 1;
        while level >= 0 {
            if self.capture[level as usize].1 == CAP_UNFINISHED {
                return Ok(level as usize);
            }
            level -= 1;
        }
        Err(LuaError::runtime("invalid pattern capture"))
    }

    /// `classend`: end of the class starting at `p`.
    fn classend(&self, p: usize) -> LuaResult<usize> {
        let (ch, mut p) = safe_decode(self.pat, p)?;
        match ch {
            L_ESC => {
                if p == self.p_end {
                    return Err(LuaError::runtime("malformed pattern (ends with '%')"));
                }
                Ok(next_i(self.pat, p, self.p_end))
            }
            0x5B => {
                if self.pat.get(p) == Some(&b'^') {
                    p += 1;
                }
                loop {
                    if p == self.p_end {
                        return Err(LuaError::runtime("malformed pattern (missing ']')"));
                    }
                    let c = self.pat[p];
                    p += 1;
                    if c == b'%' && p < self.p_end {
                        p += 1; // skip escapes like `%]'
                    }
                    if self.pat.get(p) == Some(&b']') {
                        break;
                    }
                }
                Ok(p + 1)
            }
            _ => Ok(p),
        }
    }

    fn match_bracket_class(&self, c: u32, p: usize, ec: usize) -> LuaResult<bool> {
        let mut sig = true;
        let mut p = p + 1;
        if self.pat.get(p) == Some(&b'^') {
            sig = false;
            p += 1;
        }
        while p < ec {
            let (ch, np) = safe_decode(self.pat, p)?;
            p = np;
            if ch == L_ESC {
                let (ch, np) = safe_decode(self.pat, p)?;
                p = np;
                if match_class(c, ch) {
                    return Ok(sig);
                }
            } else {
                let (next, np) = safe_decode(self.pat, p)?;
                if next == b'-' as u32 && np < ec {
                    let (next, np2) = safe_decode(self.pat, np)?;
                    p = np2;
                    if ch <= c && c <= next {
                        return Ok(sig);
                    }
                } else if ch == c {
                    return Ok(sig);
                }
            }
        }
        Ok(!sig)
    }

    fn single_match(&self, s: usize, p: usize, ep: usize) -> LuaResult<bool> {
        if s >= self.src_end {
            return Ok(false);
        }
        let (ch, _) = safe_decode(self.src, s)?;
        let (pch, p) = safe_decode(self.pat, p)?;
        match pch {
            x if x == b'.' as u32 => Ok(true),
            L_ESC => {
                let (pch, _) = safe_decode(self.pat, p)?;
                Ok(match_class(ch, pch))
            }
            x if x == b'[' as u32 => self.match_bracket_class(ch, p - 1, ep - 1),
            _ => Ok(pch == ch),
        }
    }

    fn match_balance(&self, s: usize, p: usize) -> LuaResult<Option<(usize, usize)>> {
        let (begin, p) = safe_decode(self.pat, p)?;
        if p >= self.p_end {
            return Err(LuaError::runtime(
                "malformed pattern (missing arguments to '%b')",
            ));
        }
        let (end, p) = safe_decode(self.pat, p)?;
        let (ch, mut s) = decode_at_end(self.src, s)?;
        if ch != begin {
            return Ok(None);
        }
        let mut cont = 1;
        while s < self.src_end {
            let (ch, ns) = safe_decode(self.src, s)?;
            s = ns;
            if ch == end {
                cont -= 1;
                if cont == 0 {
                    return Ok(Some((s, p)));
                }
            } else if ch == begin {
                cont += 1;
            }
        }
        Ok(None) // string ends out of balance
    }

    fn max_expand(&mut self, s: usize, p: usize, ep: usize) -> LuaResult<Option<usize>> {
        let mut m = s;
        while self.single_match(m, p, ep)? {
            m = next_i(self.src, m, self.src_end);
        }
        let s = s;
        while s <= m {
            if let Some(res) = self.match_at(m, ep + 1)? {
                return Ok(Some(res));
            }
            if s == m {
                break;
            }
            m = prev_i(self.src, s, m);
        }
        Ok(None)
    }

    fn min_expand(&mut self, mut s: usize, p: usize, ep: usize) -> LuaResult<Option<usize>> {
        loop {
            if let Some(res) = self.match_at(s, ep + 1)? {
                return Ok(Some(res));
            } else if self.single_match(s, p, ep)? {
                s = next_i(self.src, s, self.src_end);
            } else {
                return Ok(None);
            }
        }
    }

    fn start_capture(&mut self, s: usize, p: usize, what: isize) -> LuaResult<Option<usize>> {
        let level = self.level;
        if level as usize >= LUA_MAXCAPTURES {
            return Err(LuaError::runtime("too many captures"));
        }
        self.capture[level as usize] = (s, what);
        self.level = level + 1;
        let res = self.match_at(s, p)?;
        if res.is_none() {
            self.level -= 1;
        }
        Ok(res)
    }

    fn end_capture(&mut self, s: usize, p: usize) -> LuaResult<Option<usize>> {
        let l = self.capture_to_close()?;
        let init = self.capture[l].0;
        self.capture[l].1 = (s - init) as isize;
        let res = self.match_at(s, p)?;
        if res.is_none() {
            self.capture[l].1 = CAP_UNFINISHED;
        }
        Ok(res)
    }

    fn match_capture(&self, s: usize, l: i64) -> LuaResult<Option<usize>> {
        let l = self.check_capture(l)?;
        let len = self.capture[l].1;
        if (self.src_end - s) as isize >= len
            && self.src[self.capture[l].0..self.capture[l].0 + len as usize]
                == self.src[s..s + len as usize]
        {
            Ok(Some(s + len as usize))
        } else {
            Ok(None)
        }
    }

    /// `match` with the C's recursion guard. The C has one exit that always does
    /// `matchdepth++`; the inner function here returns from many places, so the
    /// bookkeeping lives in this wrapper instead.
    fn match_at(&mut self, s: usize, p: usize) -> LuaResult<Option<usize>> {
        // `if (ms->matchdepth-- == 0)`: tests the old value, then decrements.
        let exhausted = self.depth == 0;
        self.depth -= 1;
        if exhausted {
            return Err(LuaError::runtime("pattern too complex"));
        }
        let res = self.match_inner(s, p);
        self.depth += 1;
        res
    }

    /// The body of `match`; the C uses `goto init` for tail calls, this is a loop.
    fn match_inner(&mut self, s: usize, p: usize) -> LuaResult<Option<usize>> {
        let mut s = s;
        let mut p = p;
        'init: loop {
            if p != self.p_end {
                let (ch, np) = safe_decode(self.pat, p)?;
                match ch {
                    x if x == b'(' as u32 => {
                        // start capture
                        if self.pat.get(np) == Some(&b')') {
                            s = match self.start_capture(s, np + 1, CAP_POSITION)? {
                                Some(v) => v,
                                None => return Ok(None),
                            };
                        } else {
                            s = match self.start_capture(s, np, CAP_UNFINISHED)? {
                                Some(v) => v,
                                None => return Ok(None),
                            };
                        }
                    }
                    x if x == b')' as u32 => {
                        s = match self.end_capture(s, np)? {
                            Some(v) => v,
                            None => return Ok(None),
                        };
                    }
                    x if x == b'$' as u32 => {
                        if np != self.p_end {
                            // not the last char: treat as a literal
                            let r = self.default_case(s, p)?;
                            match r {
                                MatchStep::Continue(ns, np) => {
                                    s = ns;
                                    p = np;
                                    continue 'init;
                                }
                                MatchStep::Done(res) => return Ok(res),
                            }
                        }
                        if s == self.src_end {
                            // stays
                        } else {
                            return Ok(None);
                        }
                    }
                    L_ESC => {
                        let (ch, np) = safe_decode(self.pat, np)?;
                        match ch {
                            x if x == b'b' as u32 => {
                                // balanced string
                                match self.match_balance(s, np)? {
                                    Some((ns, np)) => {
                                        s = ns;
                                        p = np;
                                        continue 'init;
                                    }
                                    None => return Ok(None),
                                }
                            }
                            x if x == b'f' as u32 => {
                                // frontier
                                if self.pat.get(np) != Some(&b'[') {
                                    return Err(LuaError::runtime(
                                        "missing '[' after '%f' in pattern",
                                    ));
                                }
                                let ep = self.classend(np)?;
                                let mut previous = 0;
                                let mut current = 0;
                                if s != self.src_init {
                                    if let Some((c, _)) =
                                        decode(self.src, prev_i(self.src, self.src_init, s), false)
                                    {
                                        previous = c;
                                    }
                                }
                                if s != self.src_end {
                                    if let Some((c, _)) = decode(self.src, s, false) {
                                        current = c;
                                    }
                                }
                                if !self.match_bracket_class(previous, np, ep - 1)?
                                    && self.match_bracket_class(current, np, ep - 1)?
                                {
                                    p = ep;
                                    continue 'init;
                                }
                                return Ok(None);
                            }
                            x if (b'0' as u32..=b'9' as u32).contains(&x) => {
                                match self.match_capture(s, ch as i64)? {
                                    Some(ns) => {
                                        s = ns;
                                        p = np;
                                        continue 'init;
                                    }
                                    None => return Ok(None),
                                }
                            }
                            _ => {
                                let r = self.default_case(s, p)?;
                                match r {
                                    MatchStep::Continue(ns, np) => {
                                        s = ns;
                                        p = np;
                                        continue 'init;
                                    }
                                    MatchStep::Done(res) => return Ok(res),
                                }
                            }
                        }
                    }
                    _ => {
                        let r = self.default_case(s, p)?;
                        match r {
                            MatchStep::Continue(ns, np) => {
                                s = ns;
                                p = np;
                                continue 'init;
                            }
                            MatchStep::Done(res) => return Ok(res),
                        }
                    }
                }
            }
            return Ok(Some(s));
        }
    }

    /// The `default: dflt:` branch of the C's switch.
    fn default_case(&mut self, s: usize, p: usize) -> LuaResult<MatchStep> {
        let ep = self.classend(p)?;
        if !self.single_match(s, p, ep)? {
            if self.pat.get(ep) == Some(&b'*')
                || self.pat.get(ep) == Some(&b'?')
                || self.pat.get(ep) == Some(&b'-')
            {
                return Ok(MatchStep::Continue(s, ep + 1));
            }
            return Ok(MatchStep::Done(None));
        }
        let next_s = next_i(self.src, s, self.src_end);
        match self.pat.get(ep) {
            Some(b'?') => {
                let next_ep = next_i(self.pat, ep, self.p_end);
                match self.match_at(next_s, next_ep)? {
                    Some(res) => Ok(MatchStep::Done(Some(res))),
                    None => Ok(MatchStep::Continue(s, next_ep)),
                }
            }
            Some(b'+') => Ok(MatchStep::Done(self.max_expand(next_s, p, ep)?)),
            Some(b'*') => Ok(MatchStep::Done(self.max_expand(s, p, ep)?)),
            Some(b'-') => Ok(MatchStep::Done(self.min_expand(s, p, ep)?)),
            _ => Ok(MatchStep::Continue(next_s, ep)),
        }
    }

    /// `push_onecapture`: returns the capture as a Lua value.
    fn one_capture(&self, lua: &Lua, i: usize, s: usize, e: usize) -> LuaResult<Value> {
        if i >= self.level as usize {
            if i == 0 {
                return str_value(lua, &self.src[s..e]);
            }
            return Err(LuaError::runtime("invalid capture index"));
        }
        let len = self.capture[i].1;
        if len == CAP_UNFINISHED {
            return Err(LuaError::runtime("unfinished capture"));
        }
        if len == CAP_POSITION {
            let idx = get_index(self.capture[i].0, self.src, self.src_init, self.src_end);
            return Ok(Value::Integer(idx + 1));
        }
        let init = self.capture[i].0;
        str_value(lua, &self.src[init..init + len as usize])
    }

    /// `push_captures`.
    fn captures(&self, lua: &Lua, s: usize, e: usize, with_whole: bool) -> LuaResult<Vec<Value>> {
        let nlevels = if self.level == 0 && with_whole {
            1
        } else {
            self.level as usize
        };
        let mut out = Vec::with_capacity(nlevels);
        for i in 0..nlevels {
            out.push(self.one_capture(lua, i, s, e)?);
        }
        Ok(out)
    }
}

enum MatchStep {
    Continue(usize, usize),
    Done(Option<usize>),
}

/// `match_class`.
fn match_class(c: u32, cl: u32) -> bool {
    let res = match to_lower(cl) {
        x if x == b'a' as u32 => is_alpha(c),
        x if x == b'c' as u32 => is_cntrl(c),
        x if x == b'd' as u32 => is_digit(c),
        x if x == b'l' as u32 => is_lower(c),
        x if x == b'p' as u32 => is_punct(c),
        x if x == b's' as u32 => is_space(c),
        x if x == b't' as u32 => is_compose(c),
        x if x == b'u' as u32 => is_upper(c),
        x if x == b'x' as u32 => is_xdigit(c),
        x if x == b'g' as u32 => is_graph(c),
        x if x == b'w' as u32 => is_alnum(c),
        x if x == b'z' as u32 => c == 0,
        _ => return cl == c,
    };
    if is_lower(cl) {
        res
    } else {
        !res
    }
}

/// `get_index`: character index of `p` within `[s, e)`.
fn get_index(p: usize, b: &[u8], s: usize, e: usize) -> i64 {
    let mut i = s;
    let mut idx = 0i64;
    while i < e && i < p {
        i = next_i(b, i, e);
        idx += 1;
    }
    if i == p {
        idx
    } else {
        idx - 1
    }
}

/// `lmemfind`.
fn lmemfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn nospecials(p: &[u8]) -> bool {
    !p.iter().any(|c| SPECIALS.contains(c))
}

//================================================================================
// string-compatible interface
//================================================================================

/// `luaL_checkinteger`-like: accepts integral floats, rejects the rest.
fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(*i),
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 => Some(*n as i64),
        _ => None,
    }
}

fn to_int(v: Option<&Value>, arg: usize, fname: &str) -> LuaResult<i64> {
    v.and_then(as_int)
        .ok_or_else(|| argerror(fname, arg, "number", v.map_or("no value", |x| x.type_name())))
}

fn opt_int(v: Option<&Value>, default: i64) -> i64 {
    v.and_then(as_int).unwrap_or(default)
}

/// `lua_toboolean` on an optional argument.
fn truthy(v: Option<&Value>) -> bool {
    !matches!(v, None | Some(Value::Nil) | Some(Value::Boolean(false)))
}

fn is_none_or_nil(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Nil))
}

fn str_value(lua: &Lua, bytes: &[u8]) -> LuaResult<Value> {
    Ok(Value::String(lua.create_string(bytes)?))
}

fn multi(values: Vec<Value>) -> MultiValue {
    MultiValue::from_vec(values)
}

/// `push_offset`: the shared body of `charpos` and `next`.
fn push_offset(b: &[u8], offset: usize, idx: i64) -> Option<(i64, u32)> {
    let p = if idx != 0 {
        utf8_offset(b, offset, idx)?
    } else {
        let p = offset - 1;
        if is_cont(b, p) {
            prev_i(b, 0, p)
        } else {
            p
        }
    };
    if p == b.len() {
        return None;
    }
    let ch = decode(b, p, false).map_or(0, |(c, _)| c);
    Some((p as i64 + 1, ch))
}

fn l_len(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "len")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let len = b.len();
    let mut posi = byte_relat(opt_int(args.get(1), 1), len);
    let mut pose = byte_relat(opt_int(args.get(2), -1), len);
    let lax = truthy(args.get(3));
    if posi < 1 {
        return Err(argerror("len", 2, "valid position", "initial position out of string"));
    }
    posi -= 1;
    if posi > len as i64 {
        return Err(argerror("len", 2, "valid position", "initial position out of string"));
    }
    pose -= 1;
    if pose >= len as i64 {
        return Err(argerror("len", 3, "valid position", "final position out of string"));
    }
    let mut p = posi as usize;
    let e = if pose < 0 { 0 } else { (pose + 1) as usize };
    let mut n = 0i64;
    while p < e {
        if lax {
            p = next_i(b, p, e);
        } else {
            match decode(b, p, true) {
                None => {
                    return Ok(multi(vec![Value::Nil, Value::Integer(p as i64 + 1)]));
                }
                Some((ch, np)) => {
                    if ch > UTF8_MAXCP || (0xD800..=0xDFFF).contains(&ch) {
                        return Ok(multi(vec![Value::Nil, Value::Integer(p as i64 + 1)]));
                    }
                    p = np;
                }
            }
        }
        n += 1;
    }
    Ok(multi(vec![Value::Integer(n)]))
}

fn l_sub(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "sub")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let posi = to_int(args.get(1), 2, "sub")?;
    let pose = opt_int(args.get(2), -1);
    let (i, j, ok) = utf8_range(b, posi, pose);
    let out: &[u8] = if ok { &b[i as usize..j as usize] } else { b"" };
    Ok(multi(vec![str_value(lua, out)?]))
}

fn l_reverse(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "reverse")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let lax = truthy(args.get(1));
    let mut out = Vec::with_capacity(b.len());
    if lax {
        let mut e = b.len();
        while e > 0 {
            let prev = prev_i(b, 0, e);
            out.extend_from_slice(&b[prev..e]);
            e = prev;
        }
    } else {
        let mut e = b.len();
        let mut prev = b.len();
        while prev > 0 {
            let pprev = prev_i(b, 0, prev);
            let (code, _) = safe_decode(b, pprev)?;
            if code > UTF8_MAXCP || (0xD800..=0xDFFF).contains(&code) {
                return Err(invalid_utf8());
            }
            if !is_compose(code) {
                out.extend_from_slice(&b[pprev..e]);
                e = pprev;
            }
            prev = pprev;
        }
    }
    Ok(multi(vec![str_value(lua, &out)?]))
}

fn l_byte(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "byte")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let posi = opt_int(args.get(1), 1);
    let pose = opt_int(args.get(2), posi);
    let (i, j, ok) = utf8_range(b, posi, pose);
    let mut out = Vec::new();
    if ok {
        let mut p = i as usize;
        let e = j as usize;
        while p < e {
            let (ch, np) = safe_decode(b, p)?;
            out.push(Value::Integer(ch as i64));
            p = np;
        }
    }
    Ok(multi(out))
}

fn l_codepoint(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "codepoint")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let len = b.len();
    let posi = byte_relat(opt_int(args.get(1), 1), len);
    let pose = byte_relat(opt_int(args.get(2), posi), len);
    let lax = truthy(args.get(3));
    if posi < 1 {
        return Err(argerror("codepoint", 2, "valid position", "out of range"));
    }
    if pose > len as i64 {
        return Err(argerror("codepoint", 3, "valid position", "out of range"));
    }
    if posi > pose {
        return Ok(MultiValue::new());
    }
    let se = pose as usize;
    let mut p = (posi - 1) as usize;
    let mut out = Vec::new();
    while p < se {
        let (code, np) = safe_decode(b, p)?;
        if !lax && (code > UTF8_MAXCP || (0xD800..=0xDFFF).contains(&code)) {
            return Err(invalid_utf8());
        }
        out.push(Value::Integer(code as i64));
        p = np;
    }
    Ok(multi(out))
}

fn l_char(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let mut out = Vec::new();
    for (i, v) in args.iter().enumerate() {
        let code = as_int(v).ok_or_else(|| argerror("char", i + 1, "number", v.type_name()))?;
        if code > UTF8_MAXCP as i64 {
            return Err(argerror("char", i + 1, "valid value", "value out of range"));
        }
        encode(code as u32, &mut out);
    }
    Ok(multi(vec![str_value(lua, &out)?]))
}

/// `bind_converter`: shared body of lower/upper/title/fold.
fn converter(
    lua: &Lua,
    args: Variadic<Value>,
    name: &str,
    table: ConvTable,
) -> LuaResult<MultiValue> {
    match args.first() {
        Some(Value::Integer(i)) => Ok(multi(vec![Value::Integer(
            convert_char(table, *i as u32) as i64,
        )])),
        Some(Value::Number(n)) => Ok(multi(vec![Value::Integer(
            convert_char(table, *n as u32) as i64,
        )])),
        Some(Value::String(_)) => {
            let s = coerce_string(lua, args.first().unwrap(), 1, name)?;
            let b_owned = s.as_bytes();
            let b: &[u8] = &b_owned;
            let mut out = Vec::with_capacity(b.len());
            let mut p = 0;
            while p < b.len() {
                let (ch, np) = safe_decode(b, p)?;
                encode(convert_char(table, ch), &mut out);
                p = np;
            }
            Ok(multi(vec![str_value(lua, &out)?]))
        }
        Some(other) => Err(typeerror(name, 1, "number/string", other.type_name())),
        None => Err(typeerror(name, 1, "number/string", "no value")),
    }
}

/// `parse_escape`: returns the codepoint and the index after the escape.
fn parse_escape(b: &[u8], mut s: usize, e: usize, hex: bool) -> LuaResult<(u32, usize)> {
    let mut code: u32 = 0;
    let mut in_bracket = false;
    if b.get(s) == Some(&b'{') {
        s += 1;
        in_bracket = true;
    }
    while s < e {
        let ch = b[s];
        let digit = if ch.is_ascii_digit() {
            Some((ch - b'0') as u32)
        } else if hex && ch.is_ascii_uppercase() && ch <= b'F' {
            Some(10 + (ch - b'A') as u32)
        } else if hex && ch.is_ascii_lowercase() && ch <= b'f' {
            Some(10 + (ch - b'a') as u32)
        } else if in_bracket && ch == b'}' {
            return Ok((code, s + 1));
        } else if in_bracket {
            return Err(LuaError::runtime(format!(
                "invalid escape '{}'",
                ch as char
            )));
        } else {
            None
        };
        let Some(digit) = digit else { break };
        code = code
            .wrapping_mul(if hex { 16 } else { 10 })
            .wrapping_add(digit);
        s += 1;
    }
    Ok((code, s))
}

fn l_escape(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "escape")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let e = b.len();
    let mut out = Vec::with_capacity(e);
    let mut p = 0;
    while p < e {
        let (mut ch, mut np) = safe_decode(b, p)?;
        if ch == L_ESC {
            // The C reads the byte after '%' even at the end (NUL terminator).
            let peek = *b.get(np).unwrap_or(&0);
            let hex = matches!(peek, b'x' | b'X');
            let u_or_x = matches!(peek, b'x' | b'X' | b'u' | b'U');
            if peek.is_ascii_digit() || peek == b'{' {
                let (c, n) = parse_escape(b, np, e, false)?;
                ch = c;
                np = n;
            } else if u_or_x && np + 1 < e {
                let (c, n) = parse_escape(b, np + 1, e, hex)?;
                ch = c;
                np = n;
            } else {
                let (c, n) = decode_at_end(b, np)?;
                ch = c;
                np = n;
            }
        }
        encode(ch, &mut out);
        p = np;
    }
    Ok(multi(vec![str_value(lua, &out)?]))
}

fn l_insert(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "insert")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let mut first = b.len();
    let mut nargs = 2usize;
    if matches!(args.get(1), Some(Value::Integer(_)) | Some(Value::Number(_))) {
        let idx = opt_int(args.get(1), 0) as i32 as i64;
        if idx != 0 {
            first = relat(b, idx)
                .ok_or_else(|| argerror("insert", 2, "valid index", "invalid index"))?;
        }
        nargs += 1;
    }
    let subs = coerce_string(
        lua,
        args.get(nargs - 1).unwrap_or(&Value::Nil),
        nargs,
        "insert",
    )?;
    let sub = subs.as_bytes();
    let mut out = Vec::with_capacity(b.len() + sub.len());
    out.extend_from_slice(&b[..first]);
    out.extend_from_slice(&sub);
    out.extend_from_slice(&b[first..]);
    Ok(multi(vec![str_value(lua, &out)?]))
}

fn l_remove(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "remove")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let posi = opt_int(args.get(1), -1);
    let pose = opt_int(args.get(2), -1);
    let (i, j, ok) = utf8_range(b, posi, pose);
    if !ok {
        // The C returns argument 1 untouched (`lua_settop(L, 1)`), number or not.
        return Ok(multi(vec![args.first().cloned().unwrap_or(Value::Nil)]));
    }
    let mut out = Vec::with_capacity(b.len());
    out.extend_from_slice(&b[..i as usize]);
    out.extend_from_slice(&b[j as usize..]);
    Ok(multi(vec![str_value(lua, &out)?]))
}

fn l_charpos(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "charpos")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let result = if is_none_or_nil(args.get(2)) {
        let mut idx = opt_int(args.get(1), 0);
        let mut offset = 1i64;
        if idx > 0 {
            idx -= 1;
        } else if idx < 0 {
            offset = b.len() as i64 + 1;
        }
        push_offset(b, offset as usize, idx)
    } else {
        let mut offset = byte_relat(opt_int(args.get(1), 1), b.len());
        if offset < 1 {
            offset = 1;
        }
        push_offset(b, offset as usize, to_int(args.get(2), 3, "charpos")?)
    };
    Ok(match result {
        Some((pos, ch)) => multi(vec![Value::Integer(pos), Value::Integer(ch as i64)]),
        None => MultiValue::new(),
    })
}

fn l_offset(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "offset")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let len = b.len() as i64;
    let mut n = to_int(args.get(1), 2, "offset")?;
    let default_pos = if n >= 0 { 1 } else { len + 1 };
    let mut posi = byte_relat(opt_int(args.get(2), default_pos), b.len());
    if posi < 1 {
        return Err(argerror("offset", 3, "valid position", "position out of range"));
    }
    posi -= 1;
    if posi > len {
        return Err(argerror("offset", 3, "valid position", "position out of range"));
    }
    if n == 0 {
        // find the beginning of the current byte sequence
        while posi > 0 && is_cont(b, posi as usize) {
            posi -= 1;
        }
    } else {
        if is_cont(b, posi as usize) {
            return Err(LuaError::runtime("initial position is a continuation byte"));
        }
        if n < 0 {
            while n < 0 && posi > 0 {
                loop {
                    posi -= 1;
                    if !(posi > 0 && is_cont(b, posi as usize)) {
                        break;
                    }
                }
                n += 1;
            }
        } else {
            n -= 1; // do not move for the first character
            while n > 0 && posi < len {
                loop {
                    posi += 1;
                    if !is_cont(b, posi as usize) {
                        break;
                    }
                }
                n -= 1;
            }
        }
    }
    Ok(multi(vec![if n == 0 {
        Value::Integer(posi + 1)
    } else {
        Value::Nil
    }]))
}

fn l_next(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "next")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    let offset = byte_relat(opt_int(args.get(1), 1), b.len());
    let idx = if is_none_or_nil(args.get(2)) {
        if is_none_or_nil(args.get(1)) {
            0
        } else {
            1
        }
    } else {
        to_int(args.get(2), 3, "next")?
    };
    Ok(match push_offset(b, offset as usize, idx) {
        Some((pos, ch)) => multi(vec![Value::Integer(pos), Value::Integer(ch as i64)]),
        None => MultiValue::new(),
    })
}

fn l_codes(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "codes")?;
    let lax = truthy(args.get(1));
    let iter = lua.create_function(move |lua, args: Variadic<Value>| {
        let state = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "codes")?;
        let b_owned = state.as_bytes();
        let b: &[u8] = &b_owned;
        let n = opt_int(args.get(1), 0);
        let p = if n <= 0 { 0 } else { next_i(b, n as usize - 1, b.len()) };
        if p < b.len() {
            let (code, _) = safe_decode(b, p)?;
            if !lax && (code > UTF8_MAXCP || (0xD800..=0xDFFF).contains(&code)) {
                return Err(invalid_utf8());
            }
            return Ok(multi(vec![
                Value::Integer(p as i64 + 1),
                Value::Integer(code as i64),
            ]));
        }
        Ok(MultiValue::new())
    })?;
    Ok(multi(vec![
        Value::Function(iter),
        Value::String(s),
        Value::Integer(0),
    ]))
}

fn l_width(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let ambi_is_single = !truthy(args.get(1));
    let default_width = opt_int(args.get(2), 0) as i32 as i64;
    match args.first() {
        Some(Value::Integer(i)) => {
            let w = char_width(*i as u32, ambi_is_single);
            Ok(multi(vec![Value::Integer(if w == 0 {
                default_width
            } else {
                w
            })]))
        }
        Some(Value::Number(n)) => {
            let w = char_width(*n as u32, ambi_is_single);
            Ok(multi(vec![Value::Integer(if w == 0 {
                default_width
            } else {
                w
            })]))
        }
        Some(Value::String(_)) => {
            let s = coerce_string(lua, args.first().unwrap(), 1, "width")?;
            let b_owned = s.as_bytes();
            let b: &[u8] = &b_owned;
            let mut width = 0i32;
            let mut p = 0;
            while p < b.len() {
                let (ch, np) = safe_decode(b, p)?;
                let w = char_width(ch, ambi_is_single);
                width = width.wrapping_add(if w == 0 { default_width } else { w } as i32);
                p = np;
            }
            Ok(multi(vec![Value::Integer(width as i64)]))
        }
        Some(other) => Err(typeerror("width", 1, "number/string", other.type_name())),
        None => Err(typeerror("width", 1, "number/string", "no value")),
    }
}

fn l_widthindex(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "widthindex")?;
    let b_owned = s.as_bytes();
    let b: &[u8] = &b_owned;
    // The C casts these to `int`; plain i32 wrapping arithmetic matches it.
    let mut width = to_int(args.get(1), 2, "widthindex")? as i32;
    let ambi_is_single = !truthy(args.get(2));
    let default_width = opt_int(args.get(3), 0) as i32 as i64;
    let mut idx = 1i64;
    let mut p = 0;
    while p < b.len() {
        let (ch, np) = safe_decode(b, p)?;
        let mut w = char_width(ch, ambi_is_single);
        if w == 0 {
            w = default_width;
        }
        width = width.wrapping_sub(w as i32);
        if width <= 0 {
            return Ok(multi(vec![
                Value::Integer(idx),
                Value::Integer(width as i64 + w),
                Value::Integer(w),
            ]));
        }
        idx += 1;
        p = np;
    }
    Ok(multi(vec![Value::Integer(idx)]))
}

fn l_ncasecmp(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s1 = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "ncasecmp")?;
    let s2 = coerce_string(lua, args.get(1).unwrap_or(&Value::Nil), 2, "ncasecmp")?;
    let b1_owned = s1.as_bytes();
    let b1: &[u8] = &b1_owned;
    let b2_owned = s2.as_bytes();
    let b2: &[u8] = &b2_owned;
    let mut i1 = 0;
    let mut i2 = 0;
    while i1 < b1.len() || i2 < b2.len() {
        let (ch1, ch2) = if i1 == b1.len() {
            (0, 1)
        } else if i2 == b2.len() {
            (1, 0)
        } else {
            let (c1, n1) = safe_decode(b1, i1)?;
            let (c2, n2) = safe_decode(b2, i2)?;
            i1 = n1;
            i2 = n2;
            (to_fold(c1), to_fold(c2))
        };
        if ch1 != ch2 {
            return Ok(multi(vec![Value::Integer(if ch1 > ch2 { 1 } else { -1 })]));
        }
    }
    Ok(multi(vec![Value::Integer(0)]))
}

//================================================================================
// Pattern matching interface
//================================================================================

/// `find_aux`: shared body of `find` and `match`.
fn find_aux(lua: &Lua, args: Variadic<Value>, find: bool) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "find")?;
    let pat = coerce_string(lua, args.get(1).unwrap_or(&Value::Nil), 2, "find")?;
    let sb_owned = s.as_bytes();
    let sb: &[u8] = &sb_owned;
    let pb_owned = pat.as_bytes();
    let pb: &[u8] = &pb_owned;
    let mut idx = opt_int(args.get(2), 1);
    if idx == 0 {
        idx = 1;
    }
    let init = match relat(sb, idx) {
        Some(i) => i,
        None => {
            if idx > 0 {
                return Ok(multi(vec![Value::Nil])); // cannot find anything
            }
            0
        }
    };
    let mut init = init;

    // explicit request, or no special characters?
    if find && (truthy(args.get(3)) || nospecials(pb)) {
        if let Some(off) = lmemfind(&sb[init..], pb) {
            let s2 = init + off;
            let mut e2 = s2 + pb.len();
            if is_cont(sb, e2) {
                e2 = next_i(sb, e2, sb.len());
            }
            let start = get_index(s2, sb, 0, sb.len()) + 1;
            let end = start + get_index(e2, sb, s2, sb.len()) - 1;
            return Ok(multi(vec![
                Value::Integer(start),
                Value::Integer(end),
            ]));
        }
    } else {
        let anchor = pb.first() == Some(&b'^');
        let pstart = if anchor { 1 } else { 0 };
        if idx < 0 {
            idx = idx.wrapping_add(utf8_length(sb, 0, sb.len()) + 1);
        }
        let mut ms = MatchState::new(sb, pb);
        loop {
            ms.level = 0;
            if let Some(res) = ms.match_at(init, pstart)? {
                if find {
                    let mut out = vec![
                        Value::Integer(idx),
                        Value::Integer(idx.wrapping_add(utf8_length(sb, init, res)).wrapping_sub(1)),
                    ];
                    out.extend(ms.captures(lua, 0, 0, false)?);
                    return Ok(multi(out));
                }
                return Ok(multi(ms.captures(lua, init, res, true)?));
            }
            if init == sb.len() {
                break;
            }
            idx = idx.wrapping_add(1);
            init = next_i(sb, init, sb.len());
            if !(init <= sb.len() && !anchor) {
                break;
            }
        }
    }
    Ok(multi(vec![Value::Nil]))
}

/// `add_s`: `%n` substitution in a string replacement.
fn add_s(
    lua: &Lua,
    ms: &MatchState,
    out: &mut Vec<u8>,
    s: usize,
    e: usize,
    repl: &[u8],
) -> LuaResult<()> {
    let mut p = 0;
    while p < repl.len() {
        let (ch, np) = safe_decode(repl, p)?;
        p = np;
        if ch != L_ESC {
            encode(ch, out);
            continue;
        }
        let (ch, np) = safe_decode(repl, p)?;
        p = np;
        if !is_digit(ch) {
            if ch != L_ESC {
                return Err(LuaError::runtime(
                    "invalid use of '%' in replacement string",
                ));
            }
            encode(ch, out);
        } else if ch == b'0' as u32 {
            out.extend_from_slice(&ms.src[s..e]);
        } else {
            let v = ms.one_capture(lua, (ch - b'1' as u32) as usize, s, e)?;
            append_value(lua, out, &v)?;
        }
    }
    Ok(())
}

/// Appends a replacement value the way `luaL_addvalue` does (numbers convert).
fn append_value(lua: &Lua, out: &mut Vec<u8>, v: &Value) -> LuaResult<()> {
    let s = lua
        .coerce_string(v.clone())?
        .ok_or_else(|| LuaError::runtime("invalid replacement value (a string expected)"))?;
    out.extend_from_slice(&s.as_bytes());
    Ok(())
}

/// `add_value`: string, function or table replacement.
fn add_value(
    lua: &Lua,
    ms: &mut MatchState,
    out: &mut Vec<u8>,
    s: usize,
    e: usize,
    repl: &Value,
    repl_bytes: Option<&[u8]>,
) -> LuaResult<()> {
    let value = match repl {
        Value::Function(f) => {
            let caps = ms.captures(lua, s, e, true)?;
            let res: Value = f.call(MultiValue::from_vec(caps))?;
            res
        }
        Value::Table(t) => {
            let key = ms.one_capture(lua, 0, s, e)?;
            t.get::<Value>(key)?
        }
        _ => {
            add_s(lua, ms, out, s, e, repl_bytes.unwrap_or(b""))?;
            return Ok(());
        }
    };
    if !truthy(Some(&value)) {
        out.extend_from_slice(&ms.src[s..e]); // keep original text
        return Ok(());
    }
    if !matches!(
        value,
        Value::String(_) | Value::Integer(_) | Value::Number(_)
    ) {
        return Err(LuaError::runtime(format!(
            "invalid replacement value (a {})",
            value.type_name()
        )));
    }
    append_value(lua, out, &value)
}

fn l_gsub(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "gsub")?;
    let pat = coerce_string(lua, args.get(1).unwrap_or(&Value::Nil), 2, "gsub")?;
    let repl = args.get(2).cloned().unwrap_or(Value::Nil);
    if !matches!(
        repl,
        Value::String(_) | Value::Integer(_) | Value::Number(_) | Value::Function(_) | Value::Table(_)
    ) {
        return Err(argerror(
            "gsub",
            3,
            "string/function/table",
            repl.type_name(),
        ));
    }
    let sb_owned = s.as_bytes();
    let sb: &[u8] = &sb_owned;
    let pb_owned = pat.as_bytes();
    let pb: &[u8] = &pb_owned;
    let repl_bytes: Option<Vec<u8>> = match &repl {
        Value::String(_) | Value::Integer(_) | Value::Number(_) => {
            Some(lua.coerce_string(repl.clone())?.unwrap().as_bytes().to_vec())
        }
        _ => None,
    };
    let max_s = opt_int(args.get(3), sb.len() as i64 + 1);
    let anchor = pb.first() == Some(&b'^');
    let pstart = if anchor { 1 } else { 0 };
    let mut out = Vec::with_capacity(sb.len());
    let mut ms = MatchState::new(sb, pb);
    let mut n = 0i64;
    let mut pos = 0usize;
    while n < max_s {
        ms.level = 0;
        let matched = ms.match_at(pos, pstart)?;
        if let Some(e) = matched {
            n += 1;
            add_value(
                lua,
                &mut ms,
                &mut out,
                pos,
                e,
                &repl,
                repl_bytes.as_deref(),
            )?;
        }
        match matched {
            Some(e) if e > pos => pos = e,
            _ if pos < sb.len() => {
                let (ch, np) = safe_decode(sb, pos)?;
                encode(ch, &mut out);
                pos = np;
            }
            _ => break,
        }
        if anchor {
            break;
        }
    }
    out.extend_from_slice(&sb[pos..]);
    Ok(multi(vec![str_value(lua, &out)?, Value::Integer(n)]))
}

fn l_gmatch(lua: &Lua, args: Variadic<Value>) -> LuaResult<MultiValue> {
    let s = coerce_string(lua, args.first().unwrap_or(&Value::Nil), 1, "gmatch")?;
    let pat = coerce_string(lua, args.get(1).unwrap_or(&Value::Nil), 2, "gmatch")?;
    let state = Cell::new(0usize);
    let iter = lua.create_function(move |lua, ()| {
        let sb_owned = s.as_bytes();
        let sb: &[u8] = &sb_owned;
        let pb_owned = pat.as_bytes();
        let pb: &[u8] = &pb_owned;
        let mut ms = MatchState::new(&sb, &pb);
        let mut src = state.get();
        loop {
            if src > sb.len() {
                break;
            }
            ms.level = 0;
            if let Some(e) = ms.match_at(src, 0)? {
                let mut newstart = e;
                if e == src {
                    newstart += 1; // empty match: move at least one position
                }
                state.set(newstart);
                return Ok(multi(ms.captures(lua, src, e, true)?));
            }
            if src == sb.len() {
                break;
            }
            src = next_i(&sb, src, sb.len());
        }
        Ok(MultiValue::new())
    })?;
    Ok(multi(vec![Value::Function(iter)]))
}

//================================================================================
// Module
//================================================================================

pub fn luaopen_utf8extra(lua: &Lua) -> LuaResult<Table> {
    let m = lua.create_table()?;
    m.set("offset", lua.create_function(l_offset)?)?;
    m.set("codes", lua.create_function(l_codes)?)?;
    m.set("codepoint", lua.create_function(l_codepoint)?)?;
    m.set("len", lua.create_function(l_len)?)?;
    m.set("sub", lua.create_function(l_sub)?)?;
    m.set("reverse", lua.create_function(l_reverse)?)?;
    m.set(
        "lower",
        lua.create_function(|lua, args: Variadic<Value>| {
            converter(lua, args, "lower", TOLOWER_TABLE)
        })?,
    )?;
    m.set(
        "upper",
        lua.create_function(|lua, args: Variadic<Value>| {
            converter(lua, args, "upper", TOUPPER_TABLE)
        })?,
    )?;
    m.set(
        "title",
        lua.create_function(|lua, args: Variadic<Value>| {
            converter(lua, args, "title", TOTITLE_TABLE)
        })?,
    )?;
    m.set(
        "fold",
        lua.create_function(|lua, args: Variadic<Value>| {
            converter(lua, args, "fold", TOFOLD_TABLE)
        })?,
    )?;
    m.set("byte", lua.create_function(l_byte)?)?;
    m.set("char", lua.create_function(l_char)?)?;
    m.set("escape", lua.create_function(l_escape)?)?;
    m.set("insert", lua.create_function(l_insert)?)?;
    m.set("remove", lua.create_function(l_remove)?)?;
    m.set("charpos", lua.create_function(l_charpos)?)?;
    m.set("next", lua.create_function(l_next)?)?;
    m.set("width", lua.create_function(l_width)?)?;
    m.set("widthindex", lua.create_function(l_widthindex)?)?;
    m.set("ncasecmp", lua.create_function(l_ncasecmp)?)?;
    m.set("find", lua.create_function(|lua, args: Variadic<Value>| {
        find_aux(lua, args, true)
    })?)?;
    m.set(
        "match",
        lua.create_function(|lua, args: Variadic<Value>| find_aux(lua, args, false))?,
    )?;
    m.set("gmatch", lua.create_function(l_gmatch)?)?;
    m.set("gsub", lua.create_function(l_gsub)?)?;

    m.set("charpattern", lua.create_string(b"[\0-\x7f\xc2-\xf4][\x80-\xbf]*")?)?;
    Ok(m)
}
