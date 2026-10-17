//! `src/rencache.c`: a cache over the software renderer.
//!
//! All drawing operations of a frame are stored as commands in the window's command buffer.
//! At the end of the frame the commands are hashed into a grid of 96x96 cells, the cells that
//! differ from the previous frame are merged into dirty rectangles, and only those regions are
//! redrawn (each dirty rect replays the whole command list under that rect's clip).
//!
//! The grid is process-wide and assumes a single window, as in C: one set of cell buffers is
//! shared by every `RenWindow`.
//! shortcut: one window only (as C), upgrade if multi-window ever becomes real.

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int};
use std::ptr;

use crate::renwindow::{
    expand_command_buffer, ren_get_size, ren_set_clip_rect, ren_update_rects, renwin_get_surface,
    RenRect, RenWindow,
};

// ------------------------------------------------------------- the C types

/// `RenColor` from `src/renderer.h`: note the byte order, b first.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RenColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

/// `RenTab` from `src/renderer.h`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RenTab {
    pub offset: f64,
}

pub use crate::renderer::RenFont;

pub const FONT_FALLBACK_MAX: usize = 10;

/// The command header: an `enum CommandType` (an `int`) followed by the aligned command size.
/// `COMMAND_BARE_SIZE` is the C's `offsetof(Command, command)`.
#[repr(C)]
struct CommandHeader {
    r#type: u32,
    size: u32,
}

const COMMAND_BARE_SIZE: usize = 8;
const SET_CLIP: u32 = 0;
const DRAW_TEXT: u32 = 1;
const DRAW_RECT: u32 = 2;

#[repr(C)]
struct SetClipCommand {
    rect: RenRect,
}

#[repr(C)]
struct DrawRectCommand {
    rect: RenRect,
    color: RenColor,
}

/// `DrawTextCommand`: the fixed part. The text (len + 1 bytes) follows it in the buffer.
#[repr(C)]
struct DrawTextCommand {
    rect: RenRect,
    color: RenColor,
    fonts: [*mut RenFont; FONT_FALLBACK_MAX],
    text_x: f32,
    len: usize,
    tab_size: i8,
    tab: RenTab,
}

const CELLS_X: usize = 80;
const CELLS_Y: usize = 50;
const CELLS: usize = CELLS_X * CELLS_Y;
const CELL_SIZE: c_int = 96;
const CMD_BUF_RESIZE_RATE: f64 = 1.2;
const CMD_BUF_INIT_SIZE: usize = 1024 * 512;
const HASH_INITIAL: u32 = 2166136261;

use crate::renderer::{
    ren_font_group_get_width, ren_font_group_get_height, ren_font_group_get_tab_size,
    ren_font_group_set_tab_size, ren_draw_text, ren_draw_rect,
};

// ------------------------------------------------------------------- state

struct Cache {
    box1: Box<[u32; CELLS]>,
    box2: Box<[u32; CELLS]>,
    /// The two grids, swapped each frame like the C's `cells`/`cells_prev` pointers.
    cells: *mut u32,
    cells_prev: *mut u32,
    rect_buf: [RenRect; CELLS / 2],
    resize_issue: bool,
    screen_rect: RenRect,
    last_clip_rect: RenRect,
    show_debug: bool,
    #[cfg(feature = "test_hooks")]
    debug_rect_count: usize,
}

/// `UnsafeCell` plus `Sync`: Lua and SDL events all run on the main thread, and every access
/// goes through `cache()`, which is the only place that hands out a `&mut Cache`.
struct CacheCell(UnsafeCell<Option<Cache>>);

unsafe impl Sync for CacheCell {}

static CACHE: CacheCell = CacheCell(UnsafeCell::new(None));

/// The one cache. Callers hold `&mut` for the duration of their operation.
fn cache() -> &'static mut Cache {
    unsafe {
        let slot = &mut *CACHE.0.get();
        slot.get_or_insert_with(|| {
            let mut cache = Cache {
                box1: Box::new([0u32; CELLS]),
                box2: Box::new([0u32; CELLS]),
                cells: ptr::null_mut(),
                cells_prev: ptr::null_mut(),
                rect_buf: [RenRect { x: 0, y: 0, width: 0, height: 0 }; CELLS / 2],
                resize_issue: false,
                screen_rect: RenRect { x: 0, y: 0, width: 0, height: 0 },
                last_clip_rect: RenRect { x: 0, y: 0, width: 0, height: 0 },
                show_debug: false,
                #[cfg(feature = "test_hooks")]
                debug_rect_count: 0,
            };
            // The two grids are owned by the struct; the working pointers swap each frame.
            cache.cells_prev = cache.box1.as_mut_ptr();
            cache.cells = cache.box2.as_mut_ptr();
            cache
        })
    }
}

// ------------------------------------------------------------------- helpers

fn min(a: c_int, b: c_int) -> c_int {
    if a < b { a } else { b }
}

fn max(a: c_int, b: c_int) -> c_int {
    if a > b { a } else { b }
}

/// 32-bit FNV-1a, matching the C's `hash`.
fn hash(h: &mut u32, data: &[u8]) {
    for &byte in data {
        *h = (*h ^ byte as u32).wrapping_mul(16777619);
    }
}

fn cell_idx(x: c_int, y: c_int) -> usize {
    (x + y * CELLS_X as c_int) as usize
}

/// Note `>=` / `<=`: rects that merely touch count as overlapping, as in C.
fn rects_overlap(a: RenRect, b: RenRect) -> bool {
    b.x + b.width >= a.x
        && b.x <= a.x + a.width
        && b.y + b.height >= a.y
        && b.y <= a.y + a.height
}

fn intersect_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = max(a.x, b.x);
    let y1 = max(a.y, b.y);
    let x2 = min(a.x + a.width, b.x + b.width);
    let y2 = min(a.y + a.height, b.y + b.height);
    RenRect { x: x1, y: y1, width: max(0, x2 - x1), height: max(0, y2 - y1) }
}

fn merge_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = min(a.x, b.x);
    let y1 = min(a.y, b.y);
    let x2 = max(a.x + a.width, b.x + b.width);
    let y2 = max(a.y + a.height, b.y + b.height);
    RenRect { x: x1, y: y1, width: x2 - x1, height: y2 - y1 }
}

/// `update_overlapping_cells`: mix `hash_value` into every cell the rect touches.
unsafe fn update_overlapping_cells(cache: &mut Cache, r: RenRect, hash_value: u32) {
    let (x1, y1) = (r.x / CELL_SIZE, r.y / CELL_SIZE);
    let (x2, y2) = ((r.x + r.width) / CELL_SIZE, (r.y + r.height) / CELL_SIZE);
    for y in y1..=y2 {
        for x in x1..=x2 {
            let idx = cell_idx(x, y);
            let mut cell = *cache.cells.add(idx);
            hash(&mut cell, &hash_value.to_ne_bytes());
            *cache.cells.add(idx) = cell;
        }
    }
}

/// `push_rect`: merge into any overlapping rect already collected, else append.
fn push_rect(rect_buf: &mut [RenRect], count: &mut usize, r: RenRect) {
    for i in (0..*count).rev() {
        if rects_overlap(rect_buf[i], r) {
            rect_buf[i] = merge_rects(rect_buf[i], r);
            return;
        }
    }
    rect_buf[*count] = r;
    *count += 1;
}

// ------------------------------------------------------------ command buffer

/// `push_command`: reserve `size` payload bytes, zero them, write the header and return the
/// payload pointer. NULL means the buffer could not be grown, so this frame drops commands.
unsafe fn push_command(
    cache: &mut Cache,
    window: *mut RenWindow,
    cmd_type: u32,
    size: usize,
) -> *mut u8 {
    if window.is_null() || cache.resize_issue {
        // No commands while the buffer is unusable: wait for the next frame.
        return ptr::null_mut();
    }
    let alignment = 15usize; // alignof(max_align_t) - 1
    let size = (size + COMMAND_BARE_SIZE + alignment) & !alignment;
    let n = (*window).command_buf_idx + size;
    while n > (*window).command_buf_size {
        let new_size = if (*window).command_buf_size == 0 {
            CMD_BUF_INIT_SIZE
        } else {
            ((*window).command_buf_size as f64 * CMD_BUF_RESIZE_RATE) as usize
        };
        if !expand_command_buffer(window, new_size) {
            eprintln!("Warning: (rencache): unable to resize command buffer ({new_size})");
            cache.resize_issue = true;
            return ptr::null_mut();
        }
    }
    let cmd = (*window).command_buf.add((*window).command_buf_idx);
    (*window).command_buf_idx = n;
    ptr::write_bytes(cmd, 0, size);
    let header = cmd as *mut CommandHeader;
    (*header).r#type = cmd_type;
    (*header).size = size as u32;
    cmd.add(COMMAND_BARE_SIZE)
}

/// The rect every command starts with (the C's `cmd->command[0]`).
unsafe fn command_rect(cmd: *const CommandHeader) -> RenRect {
    ptr::read_unaligned((cmd as *const u8).add(COMMAND_BARE_SIZE) as *const RenRect)
}

/// The text of a `DRAW_TEXT` command, which follows the fixed part. `cmd` is the *command*
/// (the header), not the payload.
unsafe fn command_text(cmd: *const CommandHeader) -> *const c_char {
    (cmd as *const u8).add(COMMAND_BARE_SIZE + std::mem::size_of::<DrawTextCommand>()) as *const c_char
}

/// Walk the frame's commands from the start of the buffer to `command_buf_idx`.
unsafe fn each_command(window: *mut RenWindow, mut visit: impl FnMut(*const CommandHeader)) {
    let base = (*window).command_buf;
    let end = (*window).command_buf_idx;
    let mut offset = 0usize;
    while offset < end {
        let cmd = base.add(offset) as *const CommandHeader;
        visit(cmd);
        offset += (*cmd).size as usize;
    }
}

// ------------------------------------------------------------------- the API

#[no_mangle]
pub extern "C" fn rencache_show_debug(enable: bool) {
    cache().show_debug = enable;
}

/// # Safety
/// `window` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn rencache_set_clip_rect(window: *mut RenWindow, rect: RenRect) {
    let cache = cache();
    let cmd = push_command(cache, window, SET_CLIP, std::mem::size_of::<SetClipCommand>());
    if !cmd.is_null() {
        let rect = intersect_rects(rect, cache.screen_rect);
        (*(cmd as *mut SetClipCommand)).rect = rect;
        cache.last_clip_rect = rect;
    }
}

/// # Safety
/// `window` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn rencache_draw_rect(window: *mut RenWindow, rect: RenRect, color: RenColor) {
    let cache = cache();
    if rect.width == 0 || rect.height == 0 || !rects_overlap(cache.last_clip_rect, rect) {
        return;
    }
    let cmd = push_command(cache, window, DRAW_RECT, std::mem::size_of::<DrawRectCommand>());
    if !cmd.is_null() {
        let cmd = cmd as *mut DrawRectCommand;
        (*cmd).rect = rect;
        (*cmd).color = color;
    }
}

/// # Safety
/// `fonts` points to `FONT_FALLBACK_MAX` font pointers; `text` is `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn rencache_draw_text(
    window: *mut RenWindow,
    fonts: *mut *mut RenFont,
    text: *const c_char,
    len: usize,
    x: f64,
    y: c_int,
    color: RenColor,
    tab: RenTab,
) -> f64 {
    let cache = cache();
    let mut x_offset: c_int = 0;
    let width = ren_font_group_get_width(fonts, text, len, tab, &mut x_offset);
    // The C truncates the sum and the width here, where api/renderer.c's rect_to_grid rounds
    // instead; the asymmetry is deliberate (a text run must cover its glyphs exactly).
    let rect = RenRect {
        x: (x + x_offset as f64) as c_int,
        y,
        width: (width - x_offset as f64) as c_int,
        height: ren_font_group_get_height(fonts),
    };
    if rects_overlap(cache.last_clip_rect, rect) {
        let size = std::mem::size_of::<DrawTextCommand>() + len + 1;
        let payload = push_command(cache, window, DRAW_TEXT, size);
        if !payload.is_null() {
            let cmd = payload as *mut DrawTextCommand;
            let command_start = payload.sub(COMMAND_BARE_SIZE) as *const CommandHeader;
            let text_dest = command_text(command_start) as *mut c_char;
            // Copy the text and its terminator, like the C's memcpy(.., sz).
            ptr::copy_nonoverlapping(text, text_dest, len + 1);
            (*cmd).color = color;
            ptr::copy_nonoverlapping(fonts, (*cmd).fonts.as_mut_ptr(), FONT_FALLBACK_MAX);
            (*cmd).rect = rect;
            (*cmd).text_x = x as f32;
            (*cmd).len = len;
            (*cmd).tab_size = ren_font_group_get_tab_size(fonts) as i8;
            (*cmd).tab = tab;
        }
    }
    x + width
}

/// `rencache_invalidate`: make every cell count as changed, so the next frame repaints all.
#[no_mangle]
pub extern "C" fn rencache_invalidate() {
    let cache = cache();
    unsafe {
        ptr::write_bytes(cache.cells_prev, 0xff, CELLS);
    }
}

/// # Safety
/// `window` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn rencache_begin_frame(window: *mut RenWindow) {
    let cache = cache();
    cache.resize_issue = false;
    let mut w: c_int = 0;
    let mut h: c_int = 0;
    ren_get_size(window, &mut w, &mut h);
    if cache.screen_rect.width != w || cache.screen_rect.height != h {
        cache.screen_rect.width = w;
        cache.screen_rect.height = h;
        rencache_invalidate();
    }
    cache.last_clip_rect = cache.screen_rect;
}

/// # Safety
/// `window` is a live `RenWindow` whose command buffer holds this frame's commands.
#[no_mangle]
pub unsafe extern "C" fn rencache_end_frame(window: *mut RenWindow) {
    let cache = cache();

    // Update the cells from the commands.
    let mut cr = cache.screen_rect;
    each_command(window, |cmd| {
        if (*cmd).r#type == SET_CLIP {
            cr = command_rect(cmd);
        }
        let r = intersect_rects(command_rect(cmd), cr);
        if r.width == 0 || r.height == 0 {
            return;
        }
        let mut h = HASH_INITIAL;
        let bytes = std::slice::from_raw_parts(cmd as *const u8, (*cmd).size as usize);
        hash(&mut h, bytes);
        update_overlapping_cells(cache, r, h);
    });

    // Collect the cells changed since the last frame, and reset them.
    let mut rect_count = 0usize;
    let max_x = cache.screen_rect.width / CELL_SIZE + 1;
    let max_y = cache.screen_rect.height / CELL_SIZE + 1;
    for y in 0..max_y {
        for x in 0..max_x {
            let idx = cell_idx(x, y);
            if *cache.cells.add(idx) != *cache.cells_prev.add(idx) {
                push_rect(&mut cache.rect_buf, &mut rect_count, RenRect { x, y, width: 1, height: 1 });
            }
            *cache.cells_prev.add(idx) = HASH_INITIAL;
        }
    }

    // Expand the rects from cells to pixels, clamped to the screen.
    for i in 0..rect_count {
        let r = &mut cache.rect_buf[i];
        r.x *= CELL_SIZE;
        r.y *= CELL_SIZE;
        r.width *= CELL_SIZE;
        r.height *= CELL_SIZE;
        *r = intersect_rects(*r, cache.screen_rect);
    }

    let mut rs = renwin_get_surface(window);

    // Redraw the updated regions.
    for i in 0..rect_count {
        let r = cache.rect_buf[i];
        ren_set_clip_rect(window, r);

        each_command(window, |cmd| {
            let payload = (cmd as *const u8).add(COMMAND_BARE_SIZE);
            match (*cmd).r#type {
                SET_CLIP => {
                    let ccmd = payload as *const SetClipCommand;
                    ren_set_clip_rect(window, intersect_rects((*ccmd).rect, r));
                }
                DRAW_RECT => {
                    let rcmd = payload as *const DrawRectCommand;
                    ren_draw_rect(&mut rs, (*rcmd).rect, (*rcmd).color);
                }
                DRAW_TEXT => {
                    let tcmd = payload as *const DrawTextCommand;
                    let fonts = (*tcmd).fonts.as_ptr() as *mut *mut RenFont;
                    ren_font_group_set_tab_size(fonts, (*tcmd).tab_size as c_int);
                    ren_draw_text(
                        &mut rs,
                        fonts,
                        command_text(cmd),
                        (*tcmd).len,
                        (*tcmd).text_x,
                        (*tcmd).rect.y,
                        (*tcmd).color,
                        (*tcmd).tab,
                    );
                }
                _ => {}
            }
        });

        if cache.show_debug {
            let color = RenColor { b: libc::rand() as u8, g: libc::rand() as u8, r: libc::rand() as u8, a: 50 };
            ren_draw_rect(&mut rs, r, color);
        }
    }

    if rect_count > 0 {
        ren_update_rects(window, cache.rect_buf.as_mut_ptr(), rect_count as c_int);
    }

    // Swap the cell buffers, reset the command buffer, and remember the frame for the tests.
    std::mem::swap(&mut cache.cells, &mut cache.cells_prev);
    (*window).command_buf_idx = 0;
    #[cfg(feature = "test_hooks")]
    {
        cache.debug_rect_count = rect_count;
    }
}

/// The dirty rects of the frame just ended: test-only (see `test_hooks.rs`). The C version that
/// recorded the W6 baseline lived in `src/rencache.c`, which this file replaces.
#[cfg(feature = "test_hooks")]
#[no_mangle]
pub unsafe extern "C" fn rencache_debug_rects(rects_dest: *mut *mut RenRect) -> usize {
    let cache = cache();
    *rects_dest = cache.rect_buf.as_mut_ptr();
    cache.debug_rect_count
}
