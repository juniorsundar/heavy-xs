//! `src/renderer.c`: the glyph rasteriser and the drawing primitives.
//!
//! Fonts (`RenFont`) are opaque to C: `renderer.h` declares `typedef struct RenFont RenFont`,
//! so the only things that cross the boundary are `RenFont *` values and the `ren_font_*` /
//! `ren_draw_*` functions below. The struct is owned here and its layout is free to differ from
//! the C (there is none to match any more).
//!
//! Glyphs are rasterised by the Rust-native backend in [`crate::font`]; this module keeps the
//! atlas bump-allocator, glyph cache, fallback and blend maths ported line by line from
//! `src/renderer.c`. Where the C relied on unsigned wrap-around or `char`/`float` truncation,
//! the Rust mirrors it with explicit casts and `wrapping_*` so debug builds do not panic and
//! release output matches.

use core::ffi::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;

use crate::font::Font;
use crate::rencache::{RenColor, RenTab, FONT_FALLBACK_MAX};
use crate::renwindow::{rect_empty, rect_intersection, RenRect, RenSurface};

// --------------------------------------------------------------- C constants

const GLYPHS_PER_ATLAS: c_int = 96;
const FONT_HEIGHT_OVERFLOW_PX: c_int = 0;
const FONT_WIDTH_OVERFLOW_PX: u32 = 9;

const MAX_UNICODE: u32 = 0x10FFFF;
const CHARMAP_ROW: usize = 128;
const CHARMAP_COL: usize = 8704; // ceil(0x10FFFF / 128)
const GLYPHMAP_ROW: usize = 128;
const GLYPHMAP_COL: usize = 512; // ceil(65535 / 128)
const SUBPIXEL_BITMAPS_CACHED: usize = 3;

const FONT_ANTIALIASING_NONE: c_int = 0;
const FONT_ANTIALIASING_SUBPIXEL: c_int = 2;

const FONT_STYLE_UNDERLINE: u8 = 4;
const FONT_STYLE_STRIKETHROUGH: u8 = 16;

const EGLYPH_FORMAT_GRAYSCALE: usize = 0;
const EGLYPH_FORMAT_SUBPIXEL: usize = 1;
const EGLYPH_FORMAT_SIZE: usize = 2;

const EGLYPH_XADVANCE: u16 = 1;
const EGLYPH_BITMAP: u16 = 2;

// --------------------------------------------------------------- C structures

/// `GlyphMetric`. The layout is internal (never hashed); only the values matter.
#[derive(Clone, Copy, Default)]
struct GlyphMetric {
    xadvance: f32,
    atlas_idx: u16,
    surface_idx: u16,
    bitmap_left: i32,
    bitmap_top: i32,
    x1: u32,
    y0: u32,
    y1: u32,
    flags: u16,
    format: u8,
}

/// `CharMap`: codepoint -> glyph ID, lazily allocated rows, sentinel `u32::MAX`.
struct CharMap {
    rows: [Option<Box<[u32; CHARMAP_COL]>>; CHARMAP_ROW],
}

impl CharMap {
    fn new() -> Self {
        Self {
            rows: std::array::from_fn(|_| None),
        }
    }
}

/// Coverage bytes of one atlas surface: 3 per pixel (subpixel RGB) or 1 (grayscale), rows
/// `pitch` bytes apart, zero-filled.
struct Coverage {
    pixels: Vec<u8>,
    pitch: c_int,
    h: c_int,
}

/// One atlas surface: its coverage plus its current `"metric"` property (the C stores the
/// same pointer in an SDL property; a Rust `Option` keeps the semantics without the FFI).
/// Boxed so pointers handed out stay valid while the atlas grows.
struct AtlasSurface {
    surface: Box<Coverage>,
    metric: Option<*mut GlyphMetric>,
}

/// `GlyphAtlas`: a fixed-width bump allocator over `surfaces`.
struct GlyphAtlas {
    surfaces: Vec<AtlasSurface>,
    width: u32,
}

/// `GlyphMap`.
struct GlyphMap {
    metrics: [[Option<Box<[GlyphMetric; GLYPHMAP_COL]>>; GLYPHMAP_ROW]; SUBPIXEL_BITMAPS_CACHED],
    atlas: [Vec<GlyphAtlas>; EGLYPH_FORMAT_SIZE],
    bytesize: usize,
}

impl GlyphMap {
    fn new() -> Self {
        Self {
            metrics: std::array::from_fn(|_| std::array::from_fn(|_| None)),
            atlas: [Vec::new(), Vec::new()],
            bytesize: 0,
        }
    }
}

/// `struct RenFont` (opaque to C).
pub struct RenFont {
    face: Font,
    /// Raster line height in pixels (`FaceMetrics::line_px`), used to size new atlas surfaces.
    line_px: c_int,
    charmap: CharMap,
    glyphs: GlyphMap,
    size: f32,
    space_advance: f32,
    baseline: u16,
    height: u16,
    tab_size: u16,
    underline_thickness: u16,
    antialiasing: c_int,
    hinting: c_int,
    style: u8,
    path: CString,
}

// --------------------------------------------------------------- globals

// --------------------------------------------------------------- font helpers

fn font_is_subpixel(font: &RenFont) -> bool {
    font.antialiasing == FONT_ANTIALIASING_SUBPIXEL
}

fn font_bitmap_count(font: &RenFont) -> usize {
    if font.antialiasing == FONT_ANTIALIASING_SUBPIXEL {
        SUBPIXEL_BITMAPS_CACHED
    } else {
        1
    }
}

fn glyphformat_bytes_per_pixel(format: usize) -> c_int {
    if format == EGLYPH_FORMAT_SUBPIXEL {
        3
    } else {
        1
    }
}

// --------------------------------------------------------------- UTF-8 / whitespace

/// `utf8_to_codepoint`: consume one (possibly malformed) sequence, return the next pointer.
///
/// # Safety
/// `p` and `endp` point into the same buffer, and `p <= endp`.
unsafe fn utf8_to_codepoint(p: *const u8, endp: *const u8, dst: &mut u32) -> *const u8 {
    let byte = *p;
    let mut res: u32;
    let mut n: u32;
    match byte & 0xf0 {
        0xf0 => {
            res = (byte & 0x07) as u32;
            n = 3;
        }
        0xe0 => {
            res = (byte & 0x0f) as u32;
            n = 2;
        }
        0xd0 | 0xc0 => {
            res = (byte & 0x1f) as u32;
            n = 1;
        }
        _ => {
            res = byte as u32;
            n = 0;
        }
    }
    let mut up = p;
    while up < endp && n > 0 {
        n -= 1;
        up = up.add(1);
        res = (res << 6) | (*up & 0x3f) as u32;
    }
    *dst = res;
    up.add(1)
}

fn is_whitespace(codepoint: u32) -> bool {
    match codepoint {
        0x20 | 0x85 | 0xA0 | 0x1680 | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 => true,
        _ => (0x9..=0xD).contains(&codepoint) || (0x2000..=0x200A).contains(&codepoint),
    }
}

// --------------------------------------------------------------- glyph cache

unsafe fn font_get_glyph_id(font: &mut RenFont, codepoint: u32) -> u32 {
    if codepoint > MAX_UNICODE {
        return 0;
    }
    let row = codepoint as usize / CHARMAP_COL;
    let col = codepoint as usize - row * CHARMAP_COL;
    if font.charmap.rows[row].is_none() {
        font.charmap.rows[row] = Some(Box::new([0u32; CHARMAP_COL]));
    }
    let cells = font.charmap.rows[row].as_mut().unwrap();
    if cells[col] == 0 {
        let glyph_id = font.face.glyph_id(codepoint);
        // 0 marks "not available"; store the sentinel so the row is only probed once.
        cells[col] = if glyph_id == 0 { u32::MAX } else { glyph_id };
    }
    if cells[col] == u32::MAX {
        0
    } else {
        cells[col]
    }
}

/// `font_allocate_glyph_surface`: pick or create the atlas surface with the least waste.
unsafe fn font_allocate_glyph_surface(
    font: &mut RenFont,
    glyph_format: usize,
    bitmap_rows: c_int,
    metric: *mut GlyphMetric,
) -> *mut Coverage {
    let mut atlas_idx: c_int = -1;
    for (i, atlas) in font.glyphs.atlas[glyph_format].iter().enumerate() {
        if atlas.width >= (*metric).x1 {
            atlas_idx = i as c_int;
            break;
        }
    }
    if atlas_idx < 0 {
        font.glyphs.atlas[glyph_format].push(GlyphAtlas {
            width: (*metric).x1 + FONT_WIDTH_OVERFLOW_PX,
            surfaces: Vec::new(),
        });
        font.glyphs.bytesize += std::mem::size_of::<GlyphAtlas>();
        atlas_idx = font.glyphs.atlas[glyph_format].len() as c_int - 1;
    }
    (*metric).atlas_idx = atlas_idx as u16;

    {
        let atlas = &mut font.glyphs.atlas[glyph_format][atlas_idx as usize];

        // Find the surface with the minimum height that can fit the glyph (limited to the last 100).
        let mut surface_idx: c_int = -1;
        let max_surface_idx = atlas.surfaces.len() as c_int - 100;
        let mut min_waste = c_int::MAX;
        let mut i = atlas.surfaces.len() as c_int - 1;
        while i >= 0 && i > max_surface_idx {
            let last = atlas.surfaces[i as usize]
                .metric
                .expect("atlas surface metric");
            let new_min_waste = atlas.surfaces[i as usize].surface.h - (*last).y1 as c_int;
            if new_min_waste >= (*metric).y1 as c_int && new_min_waste < min_waste {
                surface_idx = i;
                min_waste = new_min_waste;
            }
            i -= 1;
        }

        if surface_idx < 0 {
            let mut h = FONT_HEIGHT_OVERFLOW_PX + font.line_px;
            if h <= FONT_HEIGHT_OVERFLOW_PX {
                h += bitmap_rows;
            }
            if h <= FONT_HEIGHT_OVERFLOW_PX {
                h += font.size as c_int;
            }
            let pitch = atlas.width as c_int * glyphformat_bytes_per_pixel(glyph_format);
            let h = GLYPHS_PER_ATLAS * h;
            atlas.surfaces.push(AtlasSurface {
                surface: Box::new(Coverage {
                    pixels: vec![0; pitch as usize * h as usize],
                    pitch,
                    h,
                }),
                metric: None,
            });
            font.glyphs.bytesize += std::mem::size_of::<AtlasSurface>()
                + std::mem::size_of::<Coverage>()
                + atlas.width as usize * GLYPHS_PER_ATLAS as usize * h as usize * glyph_format;
            surface_idx = atlas.surfaces.len() as c_int - 1;
        }

        (*metric).surface_idx = surface_idx as u16;
        let entry = &mut atlas.surfaces[surface_idx as usize];
        if let Some(last) = entry.metric {
            (*metric).y0 = (*last).y1;
            (*metric).y1 = (*metric).y1.wrapping_add((*last).y1);
        }
        entry.metric = Some(metric);
        &mut *entry.surface
    }
}

/// `font_load_glyph_metric`: ensure all subpixel metrics (xadvance) for a glyph are cached.
///
/// # Safety
/// `font` is live; the returned pointer is into `font.glyphs`.
unsafe fn font_load_glyph_metric(
    font: &mut RenFont,
    glyph_id: u32,
    bitmap_idx: usize,
) -> *mut GlyphMetric {
    let row = glyph_id as usize / GLYPHMAP_COL;
    let col = glyph_id as usize - row * GLYPHMAP_COL;
    let bitmaps = font_bitmap_count(font);

    let needs_load = font.glyphs.metrics[0][row].is_none()
        || (font.glyphs.metrics[0][row].as_ref().unwrap()[col].flags & EGLYPH_XADVANCE) == 0;
    if needs_load {
        let Some(advance) = font.face.advance(glyph_id) else {
            return ptr::null_mut();
        };
        for i in 0..bitmaps {
            if font.glyphs.metrics[i][row].is_none() {
                font.glyphs.metrics[i][row] =
                    Some(Box::new([GlyphMetric::default(); GLYPHMAP_COL]));
                font.glyphs.bytesize += std::mem::size_of::<GlyphMetric>() * GLYPHMAP_COL;
            }
            let metric = &mut font.glyphs.metrics[i][row].as_mut().unwrap()[col];
            metric.flags |= EGLYPH_XADVANCE;
            metric.xadvance = advance;
        }
    }

    &mut font.glyphs.metrics[bitmap_idx][row].as_mut().unwrap()[col] as *mut GlyphMetric
}

/// `font_load_glyph_bitmap`: render one subpixel index and place it in an atlas surface.
///
/// # Safety
/// `font` is live and `glyph_id` is a valid glyph for it.
unsafe fn font_load_glyph_bitmap(
    font: &mut RenFont,
    glyph_id: u32,
    bitmap_idx: usize,
) -> *mut Coverage {
    let metric = font_load_glyph_metric(font, glyph_id, bitmap_idx);
    if metric.is_null() {
        return ptr::null_mut();
    }
    if (*metric).flags & EGLYPH_BITMAP != 0 {
        return &mut *font.glyphs.atlas[(*metric).format as usize][(*metric).atlas_idx as usize]
            .surfaces[(*metric).surface_idx as usize]
            .surface;
    }

    let x_shift = bitmap_idx as f32 / SUBPIXEL_BITMAPS_CACHED as f32;
    let Some(image) = font
        .face
        .render(glyph_id, x_shift, font.antialiasing, font.style)
    else {
        return ptr::null_mut();
    };
    let format = if image.subpixel {
        EGLYPH_FORMAT_SUBPIXEL
    } else {
        EGLYPH_FORMAT_GRAYSCALE
    };
    (*metric).x1 = image.width;
    (*metric).y1 = image.rows;
    // The FreeType mono copy writes column c at atlas x = c + 1; keep mono placement identical.
    (*metric).bitmap_left = image.left + (font.antialiasing == FONT_ANTIALIASING_NONE) as i32;
    (*metric).bitmap_top = image.top;
    (*metric).flags |= EGLYPH_BITMAP;
    (*metric).format = format as u8;
    let surface = font_allocate_glyph_surface(font, format, image.rows as c_int, metric);
    let coverage = &mut *surface;
    let row_bytes = image.width as usize * glyphformat_bytes_per_pixel(format) as usize;
    for line in 0..image.rows as usize {
        let target = coverage.pitch as usize * (line + (*metric).y0 as usize);
        coverage.pixels[target..target + row_bytes]
            .copy_from_slice(&image.data[line * row_bytes..(line + 1) * row_bytes]);
    }
    surface
}

/// `font_group_get_glyph`: pick the first font in the group that has the codepoint.
///
/// # Safety
/// `fonts` is a NULL-terminated (at `FONT_FALLBACK_MAX`) array of live fonts.
unsafe fn font_group_get_glyph(
    fonts: *mut *mut RenFont,
    codepoint: u32,
    mut subpixel_idx: c_int,
    surface: *mut *mut Coverage,
    metric: *mut *mut GlyphMetric,
) -> *mut RenFont {
    if subpixel_idx < 0 {
        subpixel_idx += SUBPIXEL_BITMAPS_CACHED as c_int;
    }

    let mut font: *mut RenFont = ptr::null_mut();
    let mut glyph_id: u32 = 0;
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !(*fonts.offset(i as isize)).is_null() {
        let candidate = *fonts.offset(i as isize);
        font = candidate;
        glyph_id = font_get_glyph_id(&mut *candidate, codepoint);
        // Use the first font that has a glyph, but whitespace always uses the first font.
        if glyph_id != 0 || is_whitespace(codepoint) {
            break;
        }
        i += 1;
    }

    let subpixel_idx = if font_is_subpixel(&*font) {
        subpixel_idx
    } else {
        0
    };
    let m = font_load_glyph_metric(&mut *font, glyph_id, subpixel_idx as usize);
    // Fall back to the box-drawing character, unless this is the box or whitespace.
    if (m.is_null() || (*m).flags == 0) && codepoint != 0x25A1 && !is_whitespace(codepoint) {
        return font_group_get_glyph(fonts, 0x25A1, subpixel_idx, surface, metric);
    }
    if !metric.is_null() && !m.is_null() {
        *metric = m;
    }
    if !surface.is_null() && !m.is_null() {
        *surface = font_load_glyph_bitmap(&mut *font, glyph_id, subpixel_idx as usize);
    }
    font
}

unsafe fn font_clear_glyph_cache(font: &mut RenFont) {
    for glyph_format_idx in 0..EGLYPH_FORMAT_SIZE {
        font.glyphs.atlas[glyph_format_idx].clear();
    }
    for subpixel_idx in 0..font_bitmap_count(font) {
        for glyphmap_row in 0..GLYPHMAP_ROW {
            font.glyphs.metrics[subpixel_idx][glyphmap_row] = None;
        }
    }
    font.glyphs.bytesize = 0;
}

// --------------------------------------------------------------- face setup

/// `font_set_face_metrics`.
///
/// # Safety
/// `font.size` and the font backend are set.
unsafe fn font_set_face_metrics(font: &mut RenFont) -> c_int {
    let pixel_size = font.size;
    // Same truncation the caller applied to the logical size; heights stay in logical units.
    // `as i32 as u32` wraps negative sizes before the backend clamps them.
    let px = pixel_size as i32 as u32 as f32;
    match font
        .face
        .configure(font.size, px, font.antialiasing, font.hinting)
    {
        Ok(m) => {
            font.height = m.height as i16 as u16;
            font.baseline = m.ascent as i16 as u16;
            font.underline_thickness = m.underline_thickness as u16;
            if font.underline_thickness == 0 {
                font.underline_thickness = (font.height as f64 / 14.0).ceil() as u16;
            }
            font.space_advance = m.space_advance;
            font.line_px = m.line_px;
            0
        }
        Err(_) => 0x17, // FreeType's "invalid pixel size"
    }
}

// --------------------------------------------------------------- font lifecycle

/// `ren_font_load`. The error is owned by the caller; it used to travel via `SDL_SetError`.
///
/// # Safety
/// `path` is a NUL-terminated font path (not NULL).
pub unsafe fn ren_font_load(
    path: *const c_char,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: u8,
) -> Result<*mut RenFont, String> {
    let face = Font::load(CStr::from_ptr(path))?;

    let mut font = Box::new(RenFont {
        face,
        line_px: 0,
        charmap: CharMap::new(),
        glyphs: GlyphMap::new(),
        size,
        space_advance: 0.0,
        baseline: 0,
        height: 0,
        tab_size: 2,
        underline_thickness: 0,
        antialiasing,
        hinting,
        style,
        path: CStr::from_ptr(path).to_owned(),
    });

    if font_set_face_metrics(&mut font) != 0 {
        // The backend hides the hinting-failure detail behind FreeType's pixel-size error.
        return Err("invalid pixel size".to_string());
    }
    Ok(Box::into_raw(font))
}

/// `ren_font_copy`; `-1` means "keep the original".
///
/// # Safety
/// `font` is a live font from [`ren_font_load`].
pub unsafe fn ren_font_copy(
    font: *mut RenFont,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: c_int,
) -> Result<*mut RenFont, String> {
    let font = &*font;
    let antialiasing = if antialiasing == -1 {
        font.antialiasing
    } else {
        antialiasing
    };
    let hinting = if hinting == -1 { font.hinting } else { hinting };
    let style = if style == -1 {
        font.style as c_int
    } else {
        style
    };
    ren_font_load(font.path.as_ptr(), size, antialiasing, hinting, style as u8)
}

/// # Safety
/// `font` is a live font from [`ren_font_load`].
#[no_mangle]
pub unsafe extern "C" fn ren_font_get_path(font: *mut RenFont) -> *const c_char {
    (*font).path.as_ptr()
}

/// # Safety
/// `font` came from [`ren_font_load`] and is not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ren_font_free(font: *mut RenFont) {
    let mut font = Box::from_raw(font);
    font_clear_glyph_cache(&mut font);
    // `font` drops here, unmapping the font file and releasing the backend.
    drop(font);
}

/// # Safety
/// `fonts` is a NULL-terminated array of live fonts.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_set_tab_size(fonts: *mut *mut RenFont, n: c_int) {
    let mut j = 0;
    while j < FONT_FALLBACK_MAX && !(*fonts.offset(j as isize)).is_null() {
        (*(*fonts.offset(j as isize))).tab_size = n as u16;
        j += 1;
    }
}

/// # Safety
/// `fonts[0]` is a live font.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_get_tab_size(fonts: *mut *mut RenFont) -> c_int {
    (**fonts).tab_size as c_int
}

/// # Safety
/// `fonts[0]` is a live font.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_get_size(fonts: *mut *mut RenFont) -> f32 {
    (**fonts).size
}

/// `ren_font_group_set_size`.
///
/// # Safety
/// `fonts` is a NULL-terminated array of live fonts.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_set_size(fonts: *mut *mut RenFont, size: f32) {
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !(*fonts.offset(i as isize)).is_null() {
        let font = &mut *(*fonts.offset(i as isize));
        font_clear_glyph_cache(font);
        font.size = size;
        font.tab_size = 2;
        font_set_face_metrics(font);
        i += 1;
    }
}

/// # Safety
/// `fonts[0]` is a live font.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_get_height(fonts: *mut *mut RenFont) -> c_int {
    (**fonts).height as c_int
}

/// `font_get_xadvance`.
///
/// # Safety
/// `metric` is NULL or points at a live glyph metric.
unsafe fn font_get_xadvance(
    font: &RenFont,
    codepoint: u32,
    metric: *const GlyphMetric,
    curr_x: f64,
    tab: RenTab,
) -> f32 {
    // Some fonts (e.g. Unifont) provide xadvance for whitespace; ignore it.
    if !is_whitespace(codepoint) && !metric.is_null() && (*metric).xadvance != 0.0 {
        return (*metric).xadvance;
    }
    if codepoint != b'\t' as u32 {
        return font.space_advance;
    }
    let tab_size = font.space_advance * font.tab_size as f32;
    if tab.offset.is_nan() {
        return tab_size;
    }
    let offset = (curr_x + tab.offset) % (tab_size as f64);
    let mut adv = (tab_size as f64 - offset) as f32;
    // If there is not enough space until the next stop, skip it.
    if adv < font.space_advance {
        adv += tab_size;
    }
    adv
}

/// `ren_font_group_get_width`.
///
/// # Safety
/// `fonts` is a NULL-terminated array of live fonts; `text` has `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn ren_font_group_get_width(
    fonts: *mut *mut RenFont,
    text: *const c_char,
    len: usize,
    tab: RenTab,
    x_offset: *mut c_int,
) -> f64 {
    let mut width = 0.0f64;
    let mut set_x_offset = x_offset.is_null();
    let end = (text as *const u8).add(len);
    let mut p = text as *const u8;

    while p < end {
        let mut codepoint: u32 = 0;
        p = utf8_to_codepoint(p, end, &mut codepoint);
        let mut metric: *mut GlyphMetric = ptr::null_mut();
        font_group_get_glyph(fonts, codepoint, 0, ptr::null_mut(), &mut metric);
        width += font_get_xadvance(&**fonts, codepoint, metric, width, tab) as f64;
        if !set_x_offset && !metric.is_null() {
            set_x_offset = true;
            *x_offset = (*metric).bitmap_left;
        }
    }
    if !set_x_offset {
        *x_offset = 0;
    }
    width
}

// --------------------------------------------------------------- text drawing

/// `ren_draw_text`.
///
/// # Safety
/// `rs` is a live surface, `fonts` a NULL-terminated array, `text` has `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn ren_draw_text(
    rs: *mut RenSurface,
    fonts: *mut *mut RenFont,
    text: *const c_char,
    len: usize,
    x: f32,
    y: c_int,
    color: RenColor,
    tab: RenTab,
) -> f64 {
    let surface = &*rs;
    let clip = surface.clip;

    let surface_scale = surface.scale;
    let mut pen_x = (x * surface_scale as f32) as f64;
    let original_pen_x = pen_x;
    let y = y * surface_scale; // scaled below; `y / surface_scale` recovers the point value
    let end = (text as *const u8).add(len);
    let mut p = text as *const u8;
    let destination_pixels = surface.pixels;
    let clip_end_x = clip.x + clip.width;
    let clip_end_y = clip.y + clip.height;

    // Glyph loading mutably borrows these fonts; do not retain a shared reference across it.
    let first = *fonts;
    let mut last: *mut RenFont = ptr::null_mut();
    let mut last_pen_x = x as f64;
    let underline = (*first).style & FONT_STYLE_UNDERLINE != 0;
    let strikethrough = (*first).style & FONT_STYLE_STRIKETHROUGH != 0;

    while p < end {
        let mut codepoint: u32 = 0;
        p = utf8_to_codepoint(p, end, &mut codepoint);
        let mut font_surface: *mut Coverage = ptr::null_mut();
        let mut metric: *mut GlyphMetric = ptr::null_mut();
        let subpixel_idx = (pen_x % 1.0 * SUBPIXEL_BITMAPS_CACHED as f64) as c_int;
        let font = font_group_get_glyph(
            fonts,
            codepoint,
            subpixel_idx,
            &mut font_surface,
            &mut metric,
        );
        if metric.is_null() {
            break;
        }

        let mut start_x = (pen_x.floor() as c_int).wrapping_add((*metric).bitmap_left);
        let end_x = ((*metric).x1 as c_int).wrapping_add(start_x);
        let mut glyph_end = (*metric).x1 as c_int;
        let mut glyph_start: c_int = 0;

        if font_surface.is_null() && !is_whitespace(codepoint) {
            ren_draw_rect(
                rs,
                RenRect {
                    x: start_x.wrapping_add(1),
                    y,
                    width: ((*font).space_advance - 1.0) as c_int,
                    height: ren_font_group_get_height(fonts),
                },
                color,
            );
        }

        if !is_whitespace(codepoint)
            && !font_surface.is_null()
            && color.a > 0
            && end_x >= clip.x
            && start_x < clip_end_x
        {
            let source_pixels = (*font_surface).pixels.as_ptr();
            let font_bytes_per_pixel = glyphformat_bytes_per_pixel((*metric).format as usize);
            let subpixel = (*metric).format as usize == EGLYPH_FORMAT_SUBPIXEL;

            for line in (*metric).y0..(*metric).y1 {
                let target_y = line
                    .wrapping_sub((*metric).y0)
                    .wrapping_add(y as u32)
                    .wrapping_sub((*metric).bitmap_top as u32)
                    .wrapping_add(((*first).baseline as c_int * surface_scale) as u32)
                    as c_int;
                if target_y < clip.y {
                    continue;
                }
                if target_y >= clip_end_y {
                    break;
                }
                if start_x + (glyph_end - glyph_start) >= clip_end_x {
                    glyph_end = glyph_start + (clip_end_x - start_x);
                }
                if start_x < clip.x {
                    let offset = clip.x - start_x;
                    start_x += offset;
                    glyph_start += offset;
                }

                let mut destination_pixel = destination_pixels.offset(
                    (surface.pitch * target_y + start_x * surface.bytes_per_pixel)
                        as isize,
                ) as *mut u32;
                let mut source_pixel = source_pixels.offset(
                    (line as c_int * (*font_surface).pitch + glyph_start * font_bytes_per_pixel)
                        as isize,
                );

                let mut cursor = glyph_start;
                while cursor < glyph_end {
                    let destination_color = *destination_pixel;
                    // Not SDL_GetRGBA: the C unpacks manually for speed, and the value is
                    // truncated to a byte before blending.
                    let dst_r = (surface.r.get(destination_color) as u8) as u32;
                    let dst_g = (surface.g.get(destination_color) as u8) as u32;
                    let dst_b = (surface.b.get(destination_color) as u8) as u32;
                    let dst_a = (surface.a.get(destination_color) as u8) as u32;

                    let src_r;
                    let src_g;
                    let src_b;
                    if subpixel {
                        src_r = *source_pixel as u32;
                        source_pixel = source_pixel.add(1);
                        src_g = *source_pixel as u32;
                        source_pixel = source_pixel.add(1);
                    } else {
                        src_r = *source_pixel as u32;
                        src_g = *source_pixel as u32;
                    }
                    src_b = *source_pixel as u32;
                    source_pixel = source_pixel.add(1);

                    let r = (color.r as u32 * src_r * color.a as u32
                        + dst_r * (65025 - src_r * color.a as u32)
                        + 32767)
                        / 65025;
                    let g = (color.g as u32 * src_g * color.a as u32
                        + dst_g * (65025 - src_g * color.a as u32)
                        + 32767)
                        / 65025;
                    let b = (color.b as u32 * src_b * color.a as u32
                        + dst_b * (65025 - src_b * color.a as u32)
                        + 32767)
                        / 65025;

                    *destination_pixel = (dst_a << surface.a.shift)
                        | (r << surface.r.shift)
                        | (g << surface.g.shift)
                        | (b << surface.b.shift);
                    destination_pixel = destination_pixel.add(1);
                    cursor += 1;
                }
            }
        }

        let adv = font_get_xadvance(&*first, codepoint, metric, pen_x - original_pen_x, tab);

        if last.is_null() {
            last = font;
        } else if font != last || p == end {
            let local_pen_x = if p == end { pen_x + adv as f64 } else { pen_x };
            if underline {
                ren_draw_rect(
                    rs,
                    RenRect {
                        x: last_pen_x as c_int,
                        y: y / surface_scale + (*last).height as c_int - 1,
                        width: ((local_pen_x - last_pen_x) / surface_scale as f64) as c_int,
                        height: (*last).underline_thickness as c_int * surface_scale,
                    },
                    color,
                );
            }
            if strikethrough {
                ren_draw_rect(
                    rs,
                    RenRect {
                        x: last_pen_x as c_int,
                        y: y / surface_scale + (*last).height as c_int / 2,
                        width: ((local_pen_x - last_pen_x) / surface_scale as f64) as c_int,
                        height: (*last).underline_thickness as c_int * surface_scale,
                    },
                    color,
                );
            }
            last = font;
            last_pen_x = pen_x;
        }

        pen_x += adv as f64;
    }

    pen_x / surface_scale as f64
}

// --------------------------------------------------------------- rectangles

/// `ren_draw_rect`.
///
/// # Safety
/// `rs` is a live surface.
#[no_mangle]
pub unsafe extern "C" fn ren_draw_rect(rs: *mut RenSurface, rect: RenRect, color: RenColor) {
    if color.a == 0 {
        return;
    }

    let surface = &*rs;
    let surface_scale = surface.scale;
    let dest_rect = RenRect {
        x: rect.x * surface_scale,
        y: rect.y * surface_scale,
        width: rect.width * surface_scale,
        height: rect.height * surface_scale,
    };
    // SDL_FillSurfaceRect intersects with the clip itself; the blend path clipped manually
    // because scaled blits clip differently. Either way: SDL's rect intersection.
    let Some(clipped) = rect_intersection(dest_rect, surface.clip).filter(|r| !rect_empty(*r)) else {
        return;
    };
    if surface.w == 0 || surface.h == 0 || surface.pixels.is_null() {
        return;
    }
    let row = |y: c_int| surface.pixels.offset((surface.pitch * y) as isize) as *mut u32;

    if color.a == 0xff {
        // `SDL_MapSurfaceRGB` for an 8-bit-per-channel format: alpha opaque under its mask.
        let pixel = ((color.r as u32) << surface.r.shift)
            | ((color.g as u32) << surface.g.shift)
            | ((color.b as u32) << surface.b.shift)
            | ((0xff << surface.a.shift) & surface.a.mask);
        for y in clipped.y..clipped.y + clipped.height {
            std::slice::from_raw_parts_mut(row(y).offset(clipped.x as isize), clipped.width as usize)
                .fill(pixel);
        }
    } else {
        // SDL stretched a 1x1 blended pixel here; stretching a single colour is that colour,
        // so only its per-channel blend (`ALPHA_BLEND_CHANNEL`, SDL_blit.h) remains.
        let a = color.a as u32;
        let blend = |s: u32, d: u32| {
            let mut x = (s.wrapping_sub(d).wrapping_mul(a).wrapping_add((d << 8) - d)) & 0xffff;
            x += 1;
            x += x >> 8;
            (x >> 8) & 0xff
        };
        for y in clipped.y..clipped.y + clipped.height {
            for pixel in std::slice::from_raw_parts_mut(
                row(y).offset(clipped.x as isize),
                clipped.width as usize,
            ) {
                let d = *pixel;
                let (r, g, b, al) = (surface.r, surface.g, surface.b, surface.a);
                let mut out = (blend(color.r as u32, r.get(d)) << r.shift)
                    | (blend(color.g as u32, g.get(d)) << g.shift)
                    | (blend(color.b as u32, b.get(d)) << b.shift);
                if al.mask != 0 {
                    out |= blend(255, al.get(d)) << al.shift;
                }
                *pixel = out;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_load_error_is_an_owned_string() {
        let path = c"/nonexistent/font.ttf";
        let err = unsafe { ren_font_load(path.as_ptr(), 16.0, FONT_ANTIALIASING_SUBPIXEL, 1, 0) }
            .expect_err("a missing font must fail");
        assert!(err.contains("/nonexistent/font.ttf"), "{err}");
    }

    #[test]
    fn font_copy_forwards_the_load_error() {
        let tmp = std::env::temp_dir().join(format!("lite-xl-copy-test-{}.ttf", std::process::id()));
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/fonts/JetBrainsMono-Regular.ttf"),
            &tmp,
        ).expect("stage the fixture");
        let path = CString::new(tmp.to_str().unwrap()).unwrap();
        let font = unsafe { ren_font_load(path.as_ptr(), 16.0, FONT_ANTIALIASING_SUBPIXEL, 1, 0) }
            .expect("the staged fixture must load");
        std::fs::remove_file(&tmp).expect("drop the staged fixture");
        // The copy re-loads by path, so the vanished file surfaces through `ren_font_copy`.
        let err = unsafe { ren_font_copy(font, 16.0, -1, -1, -1) }.expect_err("a vanished font must fail");
        assert!(err.contains(tmp.to_str().unwrap()), "{err}");
        unsafe { ren_font_free(font) };
    }
}
