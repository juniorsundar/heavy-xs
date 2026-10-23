//! `src/rencache.c`: a cache over the software renderer.
//!
//! All drawing operations of a frame are stored as commands in the window's command list.
//! At the end of the frame the commands are hashed into a grid of 96x96 cells, the cells that
//! differ from the previous frame are merged into dirty rectangles, and only those regions are
//! redrawn (each dirty rect replays the whole command list under that rect's clip).
//!
//! The grid is per-thread (Lua and native events run on the main thread) and assumes a single
//! window, as in C: one set of cell buffers is shared by every `RenWindow`.
//! shortcut: one window only (as C), upgrade if multi-window ever becomes real.

use std::cell::RefCell;
use std::ffi::c_int;
use std::ops::Range;

use crate::renderer::{
    ren_draw_rect, ren_draw_text, ren_font_group_get_height, ren_font_group_get_tab_size,
    ren_font_group_get_width, ren_font_group_set_tab_size, FontGroup,
};
use crate::renwindow::{RenRect, RenWindow};

// ------------------------------------------------------------- the C types

/// `RenColor` from `src/renderer.h`: note the byte order, b first.
#[derive(Clone, Copy, Default)]
pub struct RenColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

/// `RenTab` from `src/renderer.h`.
#[derive(Clone, Copy, Default)]
pub struct RenTab {
    pub offset: f64,
}

pub const FONT_FALLBACK_MAX: usize = 10;

/// One recorded drawing operation (the C's `Command`, which packed these into a byte buffer).
pub(crate) enum Command {
    SetClip(RenRect),
    DrawRect {
        rect: RenRect,
        color: RenColor,
    },
    DrawText {
        rect: RenRect,
        color: RenColor,
        fonts: FontGroup,
        text_x: f32,
        /// The text, in the window's text arena.
        text: Range<usize>,
        tab_size: i8,
        tab: RenTab,
    },
}

impl Command {
    /// The rect every command starts with (the C's `cmd->command[0]`).
    fn rect(&self) -> RenRect {
        match self {
            Command::SetClip(rect) | Command::DrawRect { rect, .. } | Command::DrawText { rect, .. } => *rect,
        }
    }

    /// The command's hash. The C hashed the command's bytes; this hashes the same values,
    /// fonts by identity, so a frame changes cells exactly where its commands change.
    fn hash(&self, text: &[u8]) -> u32 {
        let mut h = HASH_INITIAL;
        let rect = |h: &mut u32, r: RenRect| {
            for v in [r.x, r.y, r.width, r.height] {
                hash(h, &v.to_ne_bytes());
            }
        };
        let color = |h: &mut u32, c: RenColor| hash(h, &[c.b, c.g, c.r, c.a]);
        match self {
            Command::SetClip(r) => {
                hash(&mut h, &[0]);
                rect(&mut h, *r);
            }
            Command::DrawRect { rect: r, color: c } => {
                hash(&mut h, &[1]);
                rect(&mut h, *r);
                color(&mut h, *c);
            }
            Command::DrawText { rect: r, color: c, fonts, text_x, text: range, tab_size, tab } => {
                hash(&mut h, &[2]);
                rect(&mut h, *r);
                color(&mut h, *c);
                for font in fonts.identities() {
                    hash(&mut h, &font.to_ne_bytes());
                }
                hash(&mut h, &text_x.to_ne_bytes());
                hash(&mut h, &(range.len() as u64).to_ne_bytes());
                hash(&mut h, &[*tab_size as u8]);
                hash(&mut h, &tab.offset.to_ne_bytes());
                hash(&mut h, &text[range.clone()]);
            }
        }
        h
    }
}

const CELLS_X: usize = 80;
const CELLS_Y: usize = 50;
const CELLS: usize = CELLS_X * CELLS_Y;
const CELL_SIZE: c_int = 96;
const HASH_INITIAL: u32 = 2166136261;

// ------------------------------------------------------------------- state

struct Cache {
    /// The two grids, swapped each frame like the C's `cells`/`cells_prev` pointers.
    cells: Box<[u32; CELLS]>,
    cells_prev: Box<[u32; CELLS]>,
    rect_buf: Vec<RenRect>,
    resize_issue: bool,
    screen_rect: RenRect,
    last_clip_rect: RenRect,
    show_debug: bool,
    /// The debug overlay's colour source (the C used `rand()`).
    debug_seed: u32,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache {
        cells_prev: Box::new([0u32; CELLS]),
        cells: Box::new([0u32; CELLS]),
        rect_buf: Vec::with_capacity(CELLS / 2),
        resize_issue: false,
        screen_rect: RenRect::default(),
        last_clip_rect: RenRect::default(),
        show_debug: false,
        debug_seed: 1,
    });
}

/// The one cache, borrowed for the duration of one operation. Nothing called while it is
/// borrowed reaches back into it.
fn with_cache<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    CACHE.with(|cache| f(&mut cache.borrow_mut()))
}

// ------------------------------------------------------------------- helpers

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
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);
    RenRect { x: x1, y: y1, width: (x2 - x1).max(0), height: (y2 - y1).max(0) }
}

fn merge_rects(a: RenRect, b: RenRect) -> RenRect {
    let x1 = a.x.min(b.x);
    let y1 = a.y.min(b.y);
    let x2 = (a.x + a.width).max(b.x + b.width);
    let y2 = (a.y + a.height).max(b.y + b.height);
    RenRect { x: x1, y: y1, width: x2 - x1, height: y2 - y1 }
}

/// `update_overlapping_cells`: mix `hash_value` into every cell the rect touches.
fn update_overlapping_cells(cells: &mut [u32; CELLS], r: RenRect, hash_value: u32) {
    let (x1, y1) = (r.x / CELL_SIZE, r.y / CELL_SIZE);
    let (x2, y2) = ((r.x + r.width) / CELL_SIZE, (r.y + r.height) / CELL_SIZE);
    for y in y1..=y2 {
        for x in x1..=x2 {
            hash(&mut cells[cell_idx(x, y)], &hash_value.to_ne_bytes());
        }
    }
}

/// `push_rect`: merge into any overlapping rect already collected, else append.
fn push_rect(rect_buf: &mut Vec<RenRect>, r: RenRect) {
    if let Some(existing) = rect_buf.iter_mut().rev().find(|existing| rects_overlap(**existing, r)) {
        *existing = merge_rects(*existing, r);
    } else {
        rect_buf.push(r);
    }
}

// ------------------------------------------------------------ command list

/// `push_command`: record `command`, unless there is no window or the list could not grow this
/// frame, which then drops its commands. Returns whether it was recorded.
fn push_command(cache: &mut Cache, window: Option<&mut RenWindow>, command: Command) -> bool {
    let Some(window) = window else { return false };
    if cache.resize_issue {
        // No commands while the list is unusable: wait for the next frame.
        return false;
    }
    if window.commands.try_reserve(1).is_err() {
        eprintln!("Warning: (rencache): unable to resize command buffer ({})", window.commands.len() + 1);
        cache.resize_issue = true;
        return false;
    }
    window.commands.push(command);
    true
}

// ------------------------------------------------------------------- the API

pub fn rencache_show_debug(enable: bool) {
    with_cache(|cache| cache.show_debug = enable);
}

pub fn rencache_set_clip_rect(window: Option<&mut RenWindow>, rect: RenRect) {
    with_cache(|cache| {
        let rect = intersect_rects(rect, cache.screen_rect);
        if push_command(cache, window, Command::SetClip(rect)) {
            cache.last_clip_rect = rect;
        }
    });
}

pub fn rencache_draw_rect(window: Option<&mut RenWindow>, rect: RenRect, color: RenColor) {
    with_cache(|cache| {
        if rect.width == 0 || rect.height == 0 || !rects_overlap(cache.last_clip_rect, rect) {
            return;
        }
        push_command(cache, window, Command::DrawRect { rect, color });
    });
}

/// `rencache_draw_text`: records the text and returns the x after it, with or without a window.
pub fn rencache_draw_text(
    window: Option<&mut RenWindow>,
    fonts: &FontGroup,
    text: &[u8],
    x: f64,
    y: c_int,
    color: RenColor,
    tab: RenTab,
) -> f64 {
    let mut x_offset: c_int = 0;
    let width = ren_font_group_get_width(fonts, text, tab, Some(&mut x_offset));
    // The C truncates the sum and the width here, where api/renderer.c's rect_to_grid rounds
    // instead; the asymmetry is deliberate (a text run must cover its glyphs exactly).
    let rect = RenRect {
        x: (x + x_offset as f64) as c_int,
        y,
        width: (width - x_offset as f64) as c_int,
        height: ren_font_group_get_height(fonts),
    };
    with_cache(|cache| {
        if !rects_overlap(cache.last_clip_rect, rect) {
            return;
        }
        let Some(window) = window else { return };
        if cache.resize_issue {
            return;
        }
        if window.text.try_reserve(text.len()).is_err() {
            eprintln!("Warning: (rencache): unable to resize command buffer ({})", window.text.len() + text.len());
            cache.resize_issue = true;
            return;
        }
        let start = window.text.len();
        window.text.extend_from_slice(text);
        let command = Command::DrawText {
            rect,
            color,
            fonts: fonts.clone(),
            text_x: x as f32,
            text: start..window.text.len(),
            tab_size: ren_font_group_get_tab_size(fonts) as i8,
            tab,
        };
        if !push_command(cache, Some(window), command) {
            window.text.truncate(start);
        }
    });
    x + width
}

/// `rencache_invalidate`: make every cell count as changed, so the next frame repaints all.
pub fn rencache_invalidate() {
    with_cache(|cache| cache.cells_prev.fill(0xffffffff));
}

pub fn rencache_begin_frame(window: &mut RenWindow) {
    let (w, h) = window.size();
    with_cache(|cache| {
        cache.resize_issue = false;
        if cache.screen_rect.width != w || cache.screen_rect.height != h {
            cache.screen_rect.width = w;
            cache.screen_rect.height = h;
            cache.cells_prev.fill(0xffffffff);
        }
        cache.last_clip_rect = cache.screen_rect;
    });
}

/// `rencache_end_frame`: repaint what changed from `window`'s commands, present it, and start
/// the next frame with an empty command list.
pub fn rencache_end_frame(window: &mut RenWindow) {
    let commands = std::mem::take(&mut window.commands);
    let text = std::mem::take(&mut window.text);

    with_cache(|cache| {
        // Update the cells from the commands.
        let mut cr = cache.screen_rect;
        for cmd in &commands {
            if let Command::SetClip(rect) = cmd {
                cr = *rect;
            }
            let r = intersect_rects(cmd.rect(), cr);
            if r.width == 0 || r.height == 0 {
                continue;
            }
            update_overlapping_cells(&mut cache.cells, r, cmd.hash(&text));
        }

        // Collect the cells changed since the last frame, and reset them.
        cache.rect_buf.clear();
        let max_x = cache.screen_rect.width / CELL_SIZE + 1;
        let max_y = cache.screen_rect.height / CELL_SIZE + 1;
        for y in 0..max_y {
            for x in 0..max_x {
                let idx = cell_idx(x, y);
                if cache.cells[idx] != cache.cells_prev[idx] {
                    push_rect(&mut cache.rect_buf, RenRect { x, y, width: 1, height: 1 });
                }
                cache.cells_prev[idx] = HASH_INITIAL;
            }
        }

        // Expand the rects from cells to pixels, clamped to the screen.
        let screen = cache.screen_rect;
        for r in cache.rect_buf.iter_mut() {
            r.x *= CELL_SIZE;
            r.y *= CELL_SIZE;
            r.width *= CELL_SIZE;
            r.height *= CELL_SIZE;
            *r = intersect_rects(*r, screen);
        }

        let mut rs = window.surface();

        // Redraw the updated regions.
        for &r in &cache.rect_buf {
            rs.set_clip(r);

            for cmd in &commands {
                match cmd {
                    Command::SetClip(rect) => rs.set_clip(intersect_rects(*rect, r)),
                    Command::DrawRect { rect, color } => ren_draw_rect(&mut rs, *rect, *color),
                    Command::DrawText { rect, color, fonts, text_x, text: range, tab_size, tab } => {
                        ren_font_group_set_tab_size(fonts, *tab_size as c_int);
                        ren_draw_text(&mut rs, fonts, &text[range.clone()], *text_x, rect.y, *color, *tab);
                    }
                }
            }

            if cache.show_debug {
                let mut next = || {
                    cache.debug_seed = cache.debug_seed.wrapping_mul(1103515245).wrapping_add(12345);
                    (cache.debug_seed >> 16) as u8
                };
                let color = RenColor { b: next(), g: next(), r: next(), a: 50 };
                ren_draw_rect(&mut rs, r, color);
            }
        }

        if !cache.rect_buf.is_empty() {
            window.update_rects();
        }

        // Swap the cell buffers; the frame's commands go with `commands`.
        std::mem::swap(&mut cache.cells, &mut cache.cells_prev);
    });

    // Keep the allocations for the next frame; the fonts the commands held are released here.
    let (mut commands, mut text) = (commands, text);
    commands.clear();
    text.clear();
    window.commands = commands;
    window.text = text;
}

/// The dirty rects of the frame just ended: test-only (see `test_hooks.rs`). The C version that
/// recorded the W6 baseline lived in `src/rencache.c`, which this file replaces.
#[cfg(feature = "test_hooks")]
pub fn rencache_debug_rects() -> Vec<RenRect> {
    with_cache(|cache| cache.rect_buf.clone())
}
