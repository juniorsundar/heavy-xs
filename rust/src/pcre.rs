//! Raw PCRE2 (8-bit code units) declarations.
//!
//! Hand-written rather than using the `pcre2` crate: `pcre2-sys` builds
//! `libpcre2-8`, and the C API is only a dozen
//! functions. `pcre2.h` renames everything with a `_8` suffix via
//! `PCRE2_SUFFIX` because `PCRE2_CODE_UNIT_WIDTH == 8`.

use std::os::raw::{c_int, c_void};

pub enum Pcre2Code {}
pub enum Pcre2MatchData {}

/// `PCRE2_SPTR` / `PCRE2_UCHAR`: the 8-bit code unit.
pub type CUchar = u8;
/// `PCRE2_SIZE`.
pub type Size = usize;

pub type CompileContext = c_void;
pub type MatchContext = c_void;

// Compile options (pcre2.h.generic).
pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_UTF: u32 = 0x0008_0000;

// Match options.
pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;

// Substitute options.
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;

pub const PCRE2_JIT_COMPLETE: u32 = 0x0000_0001;

pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_NOMEMORY: c_int = -48;

extern "C" {
    pub fn pcre2_compile_8(
        pattern: *const CUchar,
        length: Size,
        options: u32,
        errorcode: *mut c_int,
        erroroffset: *mut Size,
        ccontext: *mut CompileContext,
    ) -> *mut Pcre2Code;

    pub fn pcre2_code_free_8(code: *mut Pcre2Code);
    pub fn pcre2_jit_compile_8(code: *mut Pcre2Code, options: u32) -> c_int;

    pub fn pcre2_match_data_create_from_pattern_8(
        code: *const Pcre2Code,
        gcontext: *mut c_void,
    ) -> *mut Pcre2MatchData;

    pub fn pcre2_match_data_free_8(match_data: *mut Pcre2MatchData);

    pub fn pcre2_match_8(
        code: *const Pcre2Code,
        subject: *const CUchar,
        length: Size,
        startoffset: Size,
        options: u32,
        match_data: *mut Pcre2MatchData,
        mcontext: *mut MatchContext,
    ) -> c_int;

    pub fn pcre2_get_ovector_count_8(match_data: *mut Pcre2MatchData) -> u32;
    pub fn pcre2_get_ovector_pointer_8(match_data: *mut Pcre2MatchData) -> *mut Size;

    pub fn pcre2_substitute_8(
        code: *const Pcre2Code,
        subject: *const CUchar,
        length: Size,
        startoffset: Size,
        options: u32,
        match_data: *mut Pcre2MatchData,
        mcontext: *mut MatchContext,
        replacement: *const CUchar,
        rlength: Size,
        outputbuffer: *mut CUchar,
        outlengthptr: *mut Size,
    ) -> c_int;

    pub fn pcre2_get_error_message_8(
        errorcode: c_int,
        buffer: *mut CUchar,
        bufflen: Size,
    ) -> c_int;
}

/// Same buffer size the C implementation uses when formatting error messages.
pub fn error_message(code: c_int) -> String {
    let mut buf = [0u8; 256];
    let n = unsafe { pcre2_get_error_message_8(code, buf.as_mut_ptr(), buf.len()) };
    if n < 0 {
        return String::new();
    }
    String::from_utf8_lossy(&buf[..n as usize]).into_owned()
}
