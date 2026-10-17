//! `src/renderer.c`: the FreeType rasteriser and the drawing primitives.
//!
//! Fonts (`RenFont`) are opaque to C: `renderer.h` declares `typedef struct RenFont RenFont`,
//! so the only things that cross the boundary are `RenFont *` values and the `ren_font_*` /
//! `ren_draw_*` functions below. The struct is owned here and its layout is free to differ from
//! the C (there is none to match any more).
//!
//! `src/renderer.c` was ported line by line for pixel parity: the same `FT_Load_Glyph` flags,
//! the same outline style transforms, the same atlas bump-allocator and the same blend maths.
//! Where the C relied on unsigned wrap-around or `char`/`float` truncation, the Rust mirrors it
//! with explicit casts and `wrapping_*` so debug builds do not panic and release output matches.

use core::ffi::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use freetype::face::LoadFlag;
use freetype::ffi as ft;
use freetype::{Face, LcdFilter, Library, RenderMode};
use sdl3_sys::everything as sdl;

use crate::ft::{FT_Library_SetLcdFilterWeights, FT_LOAD_BITMAP_METRICS_ONLY};
use crate::rencache::{RenColor, RenTab, FONT_FALLBACK_MAX};
use crate::renwindow::{RenRect, RenSurface};

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

const FONT_HINTING_NONE: c_int = 0;
const FONT_HINTING_SLIGHT: c_int = 1;

const FONT_ANTIALIASING_NONE: c_int = 0;
const FONT_ANTIALIASING_SUBPIXEL: c_int = 2;

const FONT_STYLE_BOLD: u8 = 1;
const FONT_STYLE_ITALIC: u8 = 2;
const FONT_STYLE_UNDERLINE: u8 = 4;
const FONT_STYLE_SMOOTH: u8 = 8;
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

/// One atlas surface: the SDL surface plus its current `"metric"` property (the C stores the
/// same pointer in an SDL property; a Rust `Option` keeps the semantics without the FFI).
struct AtlasSurface {
    surface: *mut sdl::SDL_Surface,
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
    face: Face,
    charmap: CharMap,
    glyphs: GlyphMap,
    #[cfg(feature = "sdl_renderer")]
    scale: c_int,
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

/// The 1x1 surface `ren_draw_rect` blits for translucent rectangles.
static mut DRAW_RECT_SURFACE: *mut sdl::SDL_Surface = ptr::null_mut();
/// The FreeType library, created by `ren_init`, owned in a box behind a raw pointer so the
/// `static mut` is only ever copied, never borrowed. Freed by `ren_free`.
static mut LIBRARY: *mut Library = ptr::null_mut();

/// The raw library handle the remaining FFI calls (`FT_Open_Face`, LCD filter weights) need.
unsafe fn lib() -> ft::FT_Library {
    if LIBRARY.is_null() {
        ptr::null_mut()
    } else {
        (*LIBRARY).raw()
    }
}

fn check_alloc<T>(ptr: *mut T) -> *mut T {
    if ptr.is_null() {
        eprintln!("renderer: memory allocation failed");
        std::process::exit(1);
    }
    ptr
}

/// `get_ft_error`: the message is the one FreeType's `fterrdef.h` gives for the error base.
fn get_ft_error(err: ft::FT_Error) -> &'static CStr {
    // FreeType 2.14.1 fterrdef.h; the crate's Display changes the C diagnostic strings.
    match err & 0xFF {
        0x00 => c"no error",
        0x01 => c"cannot open resource",
        0x02 => c"unknown file format",
        0x03 => c"broken file",
        0x04 => c"invalid FreeType version",
        0x05 => c"module version is too low",
        0x06 => c"invalid argument",
        0x07 => c"unimplemented feature",
        0x08 => c"broken table",
        0x09 => c"broken offset within table",
        0x0A => c"array allocation size too large",
        0x0B => c"missing module",
        0x0C => c"missing property",
        0x10 => c"invalid glyph index",
        0x11 => c"invalid character code",
        0x12 => c"unsupported glyph image format",
        0x13 => c"cannot render this glyph format",
        0x14 => c"invalid outline",
        0x15 => c"invalid composite glyph",
        0x16 => c"too many hints",
        0x17 => c"invalid pixel size",
        0x18 => c"invalid SVG document",
        0x20 => c"invalid object handle",
        0x21 => c"invalid library handle",
        0x22 => c"invalid module handle",
        0x23 => c"invalid face handle",
        0x24 => c"invalid size handle",
        0x25 => c"invalid glyph slot handle",
        0x26 => c"invalid charmap handle",
        0x27 => c"invalid cache manager handle",
        0x28 => c"invalid stream handle",
        0x30 => c"too many modules",
        0x31 => c"too many extensions",
        0x40 => c"out of memory",
        0x41 => c"unlisted object",
        0x51 => c"cannot open stream",
        0x52 => c"invalid stream seek",
        0x53 => c"invalid stream skip",
        0x54 => c"invalid stream read",
        0x55 => c"invalid stream operation",
        0x56 => c"invalid frame operation",
        0x57 => c"nested frame access",
        0x58 => c"invalid frame read",
        0x60 => c"raster uninitialized",
        0x61 => c"raster corrupted",
        0x62 => c"raster overflow",
        0x63 => c"negative height while rastering",
        0x70 => c"too many registered caches",
        0x80 => c"invalid opcode",
        0x81 => c"too few arguments",
        0x82 => c"stack overflow",
        0x83 => c"code overflow",
        0x84 => c"bad argument",
        0x85 => c"division by zero",
        0x86 => c"invalid reference",
        0x87 => c"found debug opcode",
        0x88 => c"found ENDF opcode in execution stream",
        0x89 => c"nested DEFS",
        0x8A => c"invalid code range",
        0x8B => c"execution context too long",
        0x8C => c"too many function definitions",
        0x8D => c"too many instruction definitions",
        0x8E => c"SFNT font table missing",
        0x8F => c"horizontal header (hhea) table missing",
        0x90 => c"locations (loca) table missing",
        0x91 => c"name table missing",
        0x92 => c"character map (cmap) table missing",
        0x93 => c"horizontal metrics (hmtx) table missing",
        0x94 => c"PostScript (post) table missing",
        0x95 => c"invalid horizontal metrics",
        0x96 => c"invalid character map (cmap) format",
        0x97 => c"invalid ppem value",
        0x98 => c"invalid vertical metrics",
        0x99 => c"could not find context",
        0x9A => c"invalid PostScript (post) table format",
        0x9B => c"invalid PostScript (post) table",
        0x9C => c"found FDEF or IDEF opcode in glyf bytecode",
        0x9D => c"missing bitmap in strike",
        0x9E => c"SVG hooks have not been set",
        0xA0 => c"opcode syntax error",
        0xA1 => c"argument stack underflow",
        0xA2 => c"ignore",
        0xA3 => c"no Unicode glyph name found",
        0xA4 => c"glyph too big for hinting",
        0xB0 => c"`STARTFONT' field missing",
        0xB1 => c"`FONT' field missing",
        0xB2 => c"`SIZE' field missing",
        0xB3 => c"`FONTBOUNDINGBOX' field missing",
        0xB4 => c"`CHARS' field missing",
        0xB5 => c"`STARTCHAR' field missing",
        0xB6 => c"`ENCODING' field missing",
        0xB7 => c"`BBX' field missing",
        0xB8 => c"`BBX' too big",
        0xB9 => c"Font header corrupted or missing fields",
        0xBA => c"Font glyphs corrupted or missing fields",
        _ => c"unknown error",
    }
}

unsafe fn set_error(msg: &'static CStr) {
    sdl::SDL_SetError(c"%s".as_ptr(), msg.as_ptr());
}

// --------------------------------------------------------------- font options

fn font_set_load_options(font: &RenFont) -> LoadFlag {
    let load_target = if font.antialiasing == FONT_ANTIALIASING_NONE {
        LoadFlag::TARGET_MONO
    } else if font.hinting == FONT_HINTING_SLIGHT {
        LoadFlag::TARGET_LIGHT
    } else {
        LoadFlag::TARGET_NORMAL
    };
    let hinting = if font.hinting == FONT_HINTING_NONE {
        LoadFlag::NO_HINTING
    } else {
        LoadFlag::FORCE_AUTOHINT
    };
    load_target | hinting
}

fn font_set_render_options(font: &RenFont) -> RenderMode {
    if font.antialiasing == FONT_ANTIALIASING_NONE {
        return RenderMode::Mono;
    }
    if font.antialiasing == FONT_ANTIALIASING_SUBPIXEL {
        let mut weights: [u8; 5] = [0x10, 0x40, 0x70, 0x40, 0x10];
        match font.hinting {
            FONT_HINTING_NONE => unsafe {
                let _ = (*LIBRARY).set_lcd_filter(LcdFilter::LcdFilterNone);
            },
            _ => unsafe {
                FT_Library_SetLcdFilterWeights(lib(), weights.as_mut_ptr());
            },
        }
        return RenderMode::Lcd;
    }
    match font.hinting {
        FONT_HINTING_NONE => RenderMode::Normal,
        _ => RenderMode::Light,
    }
}

/// `font_set_style`: translate the outline for subpixel positioning, then apply the styles.
unsafe fn font_set_style(slot: ft::FT_GlyphSlot, x_translation: c_int, style: u8) -> c_int {
    let outline: *mut ft::FT_Outline = &mut (*slot).outline;
    ft::FT_Outline_Translate(
        outline as *const ft::FT_Outline,
        x_translation as ft::FT_Pos,
        0,
    );
    if style & FONT_STYLE_SMOOTH != 0 {
        ft::FT_Outline_Embolden(outline, 1 << 5);
    }
    if style & FONT_STYLE_BOLD != 0 {
        ft::FT_Outline_EmboldenXY(outline, 1 << 5, 0);
    }
    if style & FONT_STYLE_ITALIC != 0 {
        let matrix = ft::FT_Matrix {
            xx: 1 << 16,
            xy: 1 << 14,
            yx: 0,
            yy: 1 << 16,
        };
        ft::FT_Outline_Transform(outline as *const ft::FT_Outline, &matrix);
    }
    0
}

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

unsafe fn slot_bitmap_type(slot: ft::FT_GlyphSlot) -> usize {
    if (*slot).bitmap.pixel_mode as u8 as u32 == ft::FT_PIXEL_MODE_LCD {
        EGLYPH_FORMAT_SUBPIXEL
    } else {
        EGLYPH_FORMAT_GRAYSCALE
    }
}

fn glyphformat_to_pixelformat(format: usize) -> sdl::SDL_PixelFormat {
    match format {
        EGLYPH_FORMAT_SUBPIXEL => sdl::SDL_PixelFormat::RGB24,
        EGLYPH_FORMAT_GRAYSCALE => sdl::SDL_PixelFormat::INDEX8,
        _ => sdl::SDL_PixelFormat::UNKNOWN,
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
        let glyph_id = font.face.get_char_index(codepoint as usize);
        // -1 marks "not available"; OpenType glyph IDs are uint16 so this is unreachable in theory.
        cells[col] = glyph_id.unwrap_or(u32::MAX);
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
    slot: ft::FT_GlyphSlot,
    _bitmap_idx: c_int,
    metric: *mut GlyphMetric,
) -> *mut sdl::SDL_Surface {
    let glyph_format = slot_bitmap_type(slot);

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
            let new_min_waste = (*atlas.surfaces[i as usize].surface).h - (*last).y1 as c_int;
            if new_min_waste >= (*metric).y1 as c_int && new_min_waste < min_waste {
                surface_idx = i;
                min_waste = new_min_waste;
            }
            i -= 1;
        }

        if surface_idx < 0 {
            let mut h = FONT_HEIGHT_OVERFLOW_PX
                + font
                    .face
                    .size_metrics()
                    .map_or(0, |m| m.height as f64 as c_int / 64);
            if h <= FONT_HEIGHT_OVERFLOW_PX {
                h += (*slot).bitmap.rows;
            }
            if h <= FONT_HEIGHT_OVERFLOW_PX {
                h += font.size as c_int;
            }
            let format = glyphformat_to_pixelformat(glyph_format);
            let surface = check_alloc(sdl::SDL_CreateSurface(
                atlas.width as c_int,
                GLYPHS_PER_ATLAS * h,
                format,
            ));
            atlas.surfaces.push(AtlasSurface {
                surface,
                metric: None,
            });
            font.glyphs.bytesize += std::mem::size_of::<*mut sdl::SDL_Surface>()
                + std::mem::size_of::<sdl::SDL_Surface>()
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
        entry.surface
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
    let load_option = font_set_load_options(font);
    let row = glyph_id as usize / GLYPHMAP_COL;
    let col = glyph_id as usize - row * GLYPHMAP_COL;
    let bitmaps = font_bitmap_count(font);

    let needs_load = font.glyphs.metrics[0][row].is_none()
        || (font.glyphs.metrics[0][row].as_ref().unwrap()[col].flags & EGLYPH_XADVANCE) == 0;
    if needs_load {
        // Load without hinting to get the correct LSB/RSB delta for monospaced fonts. The
        // transform and subpixel positioning do not affect the xadvance.
        let flags = (load_option
            | LoadFlag::from_bits_retain(FT_LOAD_BITMAP_METRICS_ONLY)
            | LoadFlag::NO_HINTING)
            & !LoadFlag::FORCE_AUTOHINT;
        if font.face.load_glyph(glyph_id, flags).is_err() {
            return ptr::null_mut();
        }
        let advance_x = font.face.glyph().advance().x;
        for i in 0..bitmaps {
            if font.glyphs.metrics[i][row].is_none() {
                font.glyphs.metrics[i][row] =
                    Some(Box::new([GlyphMetric::default(); GLYPHMAP_COL]));
                font.glyphs.bytesize += std::mem::size_of::<GlyphMetric>() * GLYPHMAP_COL;
            }
            let metric = &mut font.glyphs.metrics[i][row].as_mut().unwrap()[col];
            metric.flags |= EGLYPH_XADVANCE;
            metric.xadvance = advance_x as f32 / 64.0;
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
) -> *mut sdl::SDL_Surface {
    let metric = font_load_glyph_metric(font, glyph_id, bitmap_idx);
    if metric.is_null() {
        return ptr::null_mut();
    }
    if (*metric).flags & EGLYPH_BITMAP != 0 {
        return font.glyphs.atlas[(*metric).format as usize][(*metric).atlas_idx as usize].surfaces
            [(*metric).surface_idx as usize]
            .surface;
    }

    let load_option = font_set_load_options(font);
    let render_option = font_set_render_options(font);
    if font
        .face
        .load_glyph(
            glyph_id,
            load_option | LoadFlag::from_bits_retain(FT_LOAD_BITMAP_METRICS_ONLY),
        )
        .is_err()
    {
        return ptr::null_mut();
    }
    let slot = font.face.raw().glyph;
    if font_set_style(
        slot,
        bitmap_idx as c_int * (64 / SUBPIXEL_BITMAPS_CACHED as c_int),
        font.style,
    ) != 0
        || font.face.glyph().render_glyph(render_option).is_err()
    {
        return ptr::null_mut();
    }

    let (bmp_rows, bmp_width, bmp_pitch, bmp_buffer, bmp_pixel_mode) = {
        let bitmap = &(*slot).bitmap;
        (
            bitmap.rows,
            bitmap.width,
            bitmap.pitch,
            bitmap.buffer,
            bitmap.pixel_mode,
        )
    };

    // An empty bitmap, or one in a format we cannot draw, just keeps the xadvance.
    if bmp_width == 0
        || bmp_rows == 0
        || bmp_buffer.is_null()
        || (bmp_pixel_mode as u8 as u32 != ft::FT_PIXEL_MODE_MONO
            && bmp_pixel_mode as u8 as u32 != ft::FT_PIXEL_MODE_GRAY
            && bmp_pixel_mode as u8 as u32 != ft::FT_PIXEL_MODE_LCD)
    {
        return ptr::null_mut();
    }

    let bitmaps = font_bitmap_count(font);
    let mut glyph_width = bmp_width as u32 / bitmaps as u32;
    // FT_PIXEL_MODE_MONO uses a 1-bit-per-pixel packed bitmap.
    if bmp_pixel_mode as u8 as u32 == ft::FT_PIXEL_MODE_MONO {
        glyph_width *= 8;
    }

    (*metric).x1 = glyph_width;
    (*metric).y1 = bmp_rows as u32;
    (*metric).bitmap_left = font.face.glyph().bitmap_left();
    (*metric).bitmap_top = font.face.glyph().bitmap_top();
    (*metric).flags |= EGLYPH_BITMAP;
    (*metric).format = slot_bitmap_type(slot) as u8;

    let surface = font_allocate_glyph_surface(font, slot, bitmap_idx as c_int, metric);
    let pixels = (*surface).pixels as *mut u8;
    for line in 0..bmp_rows {
        let target_offset = (*surface).pitch * (line + (*metric).y0 as c_int);
        let source_offset = line * bmp_pitch;
        if font.antialiasing == FONT_ANTIALIASING_NONE {
            for column in 0..bmp_width {
                let current_source_offset = source_offset + (column / 8);
                let source_pixel = *bmp_buffer.offset(current_source_offset as isize);
                let value = (((source_pixel as u32) >> (7 - (column % 8) as u32)) & 1) * 0xFF;
                *pixels.offset((target_offset + 1 + column) as isize) = value as u8;
            }
        } else {
            ptr::copy_nonoverlapping(
                bmp_buffer.offset(source_offset as isize),
                pixels.offset(target_offset as isize),
                bmp_width as usize,
            );
        }
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
    surface: *mut *mut sdl::SDL_Surface,
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
        for atlas in font.glyphs.atlas[glyph_format_idx].iter() {
            for entry in atlas.surfaces.iter() {
                sdl::SDL_DestroySurface(entry.surface);
            }
        }
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
/// `face` is an open face; `font.size` is set.
unsafe fn font_set_face_metrics(font: &mut RenFont) -> ft::FT_Error {
    let pixel_size = font.size;
    #[cfg(feature = "sdl_renderer")]
    let pixel_size = pixel_size * font.scale as f32;
    if let Err(err) = font.face.set_pixel_sizes(0, pixel_size as i32 as u32) {
        return err as ft::FT_Error;
    }

    if font.face.is_scalable() {
        let upem = font.face.raw().units_per_EM as f32;
        font.height = ((font.face.height() as f32 / upem) * font.size) as i16 as u16;
        font.baseline = ((font.face.ascender() as f32 / upem) * font.size) as i16 as u16;
        font.underline_thickness =
            ((font.face.underline_thickness() as f32 / upem) * font.size) as u16;
    } else {
        // C: `(short) metrics.height / 64.0f` -- the cast binds before the division.
        let metrics = font
            .face
            .size_metrics()
            .expect("size metrics after set_pixel_sizes");
        font.height = ((metrics.height as i16) as f32 / 64.0) as u16;
        font.baseline = ((metrics.ascender as i16) as f32 / 64.0) as u16;
    }
    if font.underline_thickness == 0 {
        font.underline_thickness = (font.height as f64 / 14.0).ceil() as u16;
    }

    let flags = (font_set_load_options(font)
        | LoadFlag::from_bits_retain(FT_LOAD_BITMAP_METRICS_ONLY)
        | LoadFlag::NO_HINTING)
        & !LoadFlag::FORCE_AUTOHINT;
    if let Err(err) = font.face.load_char(b' ' as usize, flags) {
        return err as ft::FT_Error;
    }
    font.space_advance = font.face.glyph().advance().x as f32 / 64.0;
    0
}

// --------------------------------------------------------------- font lifecycle

/// `ren_font_load`.
///
/// # Safety
/// `path` is a NUL-terminated font path (not NULL).
#[no_mangle]
pub unsafe extern "C" fn ren_font_load(
    path: *const c_char,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: u8,
) -> *mut RenFont {
    let file = sdl::SDL_IOFromFile(path, c"rb".as_ptr());
    if file.is_null() {
        return ptr::null_mut(); // error set by SDL_IOFromFile
    }

    let stream = crate::ft::font_stream_new(file);
    let mut face: ft::FT_Face = ptr::null_mut();
    let args = ft::FT_Open_Args {
        flags: ft::FT_OPEN_STREAM,
        memory_base: ptr::null(),
        memory_size: 0,
        pathname: ptr::null_mut(),
        stream,
        driver: ptr::null_mut(),
        num_params: 0,
        params: ptr::null_mut(),
    };
    let err = ft::FT_Open_Face(lib(), &args, 0, &mut face);
    if err != 0 {
        if !face.is_null() {
            ft::FT_Done_Face(face);
        }
        set_error(get_ft_error(err));
        return ptr::null_mut();
    }

    let mut font = Box::new(RenFont {
        face: Face::from_raw(lib(), face, None),
        charmap: CharMap::new(),
        glyphs: GlyphMap::new(),
        #[cfg(feature = "sdl_renderer")]
        scale: 1,
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

    let err = font_set_face_metrics(&mut font);
    if err != 0 {
        set_error(get_ft_error(err));
        return ptr::null_mut(); // dropping `font` runs the owned `Face`'s FT_Done_Face
    }
    Box::into_raw(font)
}

/// `ren_font_copy`; `-1` means "keep the original".
///
/// # Safety
/// `font` is a live font from [`ren_font_load`].
#[no_mangle]
pub unsafe extern "C" fn ren_font_copy(
    font: *mut RenFont,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: c_int,
) -> *mut RenFont {
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
    // `font` drops here, running the owned `Face`'s FT_Done_Face and its library reference release.
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
pub unsafe extern "C" fn ren_font_group_set_size(
    fonts: *mut *mut RenFont,
    size: f32,
    surface_scale: c_int,
) {
    #[cfg(not(feature = "sdl_renderer"))]
    let _ = surface_scale;
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !(*fonts.offset(i as isize)).is_null() {
        let font = &mut *(*fonts.offset(i as isize));
        font_clear_glyph_cache(font);
        font.size = size;
        font.tab_size = 2;
        #[cfg(feature = "sdl_renderer")]
        {
            font.scale = surface_scale;
        }
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
    #[cfg(feature = "sdl_renderer")]
    {
        width / (**fonts).scale as f64
    }
    #[cfg(not(feature = "sdl_renderer"))]
    {
        width
    }
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
    let surface = (*rs).surface;
    let mut clip = sdl::SDL_Rect::default();
    sdl::SDL_GetSurfaceClipRect(surface, &mut clip);

    let surface_scale = (*rs).scale;
    let mut pen_x = (x * surface_scale as f32) as f64;
    let original_pen_x = pen_x;
    let y = y * surface_scale; // scaled below; `y / surface_scale` recovers the point value
    let end = (text as *const u8).add(len);
    let mut p = text as *const u8;
    let destination_pixels = (*surface).pixels as *mut u8;
    let clip_end_x = clip.x + clip.w;
    let clip_end_y = clip.y + clip.h;

    // Glyph loading mutably borrows these fonts; do not retain a shared reference across it.
    let first = *fonts;
    let mut last: *mut RenFont = ptr::null_mut();
    let mut last_pen_x = x as f64;
    let underline = (*first).style & FONT_STYLE_UNDERLINE != 0;
    let strikethrough = (*first).style & FONT_STYLE_STRIKETHROUGH != 0;

    while p < end {
        let mut codepoint: u32 = 0;
        p = utf8_to_codepoint(p, end, &mut codepoint);
        let mut font_surface: *mut sdl::SDL_Surface = ptr::null_mut();
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
            let source_pixels = (*font_surface).pixels as *mut u8;
            let format = sdl::SDL_GetPixelFormatDetails((*surface).format);
            let font_format = sdl::SDL_GetPixelFormatDetails((*font_surface).format);
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
                    ((*surface).pitch * target_y + start_x * (*format).bytes_per_pixel as c_int)
                        as isize,
                ) as *mut u32;
                let mut source_pixel = source_pixels.offset(
                    (line as c_int * (*font_surface).pitch
                        + glyph_start * (*font_format).bytes_per_pixel as c_int)
                        as isize,
                );

                let mut cursor = glyph_start;
                while cursor < glyph_end {
                    let destination_color = *destination_pixel;
                    // Not SDL_GetRGBA: the C unpacks manually for speed, and the value is
                    // truncated to a byte before blending.
                    let dst_r =
                        (((destination_color & (*format).Rmask) >> (*format).Rshift) as u8) as u32;
                    let dst_g =
                        (((destination_color & (*format).Gmask) >> (*format).Gshift) as u8) as u32;
                    let dst_b =
                        (((destination_color & (*format).Bmask) >> (*format).Bshift) as u8) as u32;
                    let dst_a =
                        (((destination_color & (*format).Amask) >> (*format).Ashift) as u8) as u32;

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

                    *destination_pixel = (dst_a << (*format).Ashift)
                        | (r << (*format).Rshift)
                        | (g << (*format).Gshift)
                        | (b << (*format).Bshift);
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

    let surface = (*rs).surface;
    let surface_scale = (*rs).scale;
    let dest_rect = sdl::SDL_Rect {
        x: rect.x * surface_scale,
        y: rect.y * surface_scale,
        w: rect.width * surface_scale,
        h: rect.height * surface_scale,
    };

    if color.a == 0xff {
        let translated = sdl::SDL_MapSurfaceRGB(surface, color.r, color.g, color.b);
        sdl::SDL_FillSurfaceRect(surface, &dest_rect, translated);
    } else {
        // SDL does not clip scaled blits the way we expect, so clip manually.
        let mut clip = sdl::SDL_Rect::default();
        sdl::SDL_GetSurfaceClipRect(surface, &mut clip);
        let mut clipped = sdl::SDL_Rect::default();
        if !sdl::SDL_GetRectIntersection(&clip, &dest_rect, &mut clipped) {
            return;
        }

        let pixel = (*DRAW_RECT_SURFACE).pixels as *mut u32;
        *pixel = sdl::SDL_MapSurfaceRGBA(DRAW_RECT_SURFACE, color.r, color.g, color.b, color.a);
        sdl::SDL_BlitSurfaceScaled(
            DRAW_RECT_SURFACE,
            ptr::null(),
            surface,
            &clipped,
            sdl::SDL_SCALEMODE_LINEAR,
        );
    }
}

// --------------------------------------------------------------- init / teardown

/// `video_init`.
#[no_mangle]
pub unsafe extern "C" fn video_init() -> c_int {
    static REN_INITED: AtomicBool = AtomicBool::new(false);
    if !REN_INITED.load(Ordering::SeqCst) {
        if !sdl::SDL_InitSubSystem(sdl::SDL_INIT_VIDEO) {
            return -1;
        }
        sdl::SDL_EnableScreenSaver();
        sdl::SDL_SetHint(
            sdl::SDL_HINT_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR,
            c"0".as_ptr(),
        );
        sdl::SDL_SetHint(sdl::SDL_HINT_MOUSE_FOCUS_CLICKTHROUGH, c"1".as_ptr());
        sdl::SDL_SetHint(sdl::SDL_HINT_IME_IMPLEMENTED_UI, c"1".as_ptr());
        // Respect borderless windows as normal windows, and allow resizing/aero-snap.
        sdl::SDL_SetHint(c"SDL_BORDERLESS_WINDOWED_STYLE".as_ptr(), c"1".as_ptr());
        sdl::SDL_SetHint(c"SDL_BORDERLESS_RESIZABLE_STYLE".as_ptr(), c"1".as_ptr());
        sdl::SDL_SetHint(sdl::SDL_HINT_MOUSE_DOUBLE_CLICK_RADIUS, c"4".as_ptr());
        sdl::SDL_SetHint(sdl::SDL_HINT_RENDER_DRIVER, c"software".as_ptr());
        REN_INITED.store(true, Ordering::SeqCst);
    }
    0
}

/// `ren_init`.
#[no_mangle]
pub unsafe extern "C" fn ren_init() -> c_int {
    let surface = sdl::SDL_CreateSurface(1, 1, sdl::SDL_PixelFormat::RGBA32);
    if surface.is_null() {
        return -1;
    }
    DRAW_RECT_SURFACE = surface;

    let library = match Library::init() {
        Ok(library) => library,
        Err(err) => {
            return sdl::SDL_SetError(c"%s".as_ptr(), get_ft_error(err as ft::FT_Error).as_ptr())
                as c_int
        }
    };
    LIBRARY = Box::into_raw(Box::new(library));
    0
}

/// `ren_free`.
#[no_mangle]
pub unsafe extern "C" fn ren_free() {
    sdl::SDL_DestroySurface(DRAW_RECT_SURFACE);
    DRAW_RECT_SURFACE = ptr::null_mut();
    if !LIBRARY.is_null() {
        // `Library` drop runs FT_Done_Library (the FT_Done_FreeType equivalent).
        drop(Box::from_raw(LIBRARY));
        LIBRARY = ptr::null_mut();
    }
}

/// `update_font_scale`: re-rasterise the group when the surface scale changed.
///
/// # Safety
/// `window_renderer` is NULL or a live window; `fonts` is a NULL-terminated font array.
#[cfg(feature = "sdl_renderer")]
#[no_mangle]
pub unsafe extern "C" fn update_font_scale(
    window_renderer: *mut crate::renwindow::RenWindow,
    fonts: *mut *mut RenFont,
) {
    if window_renderer.is_null() {
        return;
    }
    let surface_scale = crate::renwindow::renwin_get_surface(window_renderer).scale;
    let mut i = 0;
    while i < FONT_FALLBACK_MAX && !(*fonts.offset(i as isize)).is_null() {
        if (*(*fonts.offset(i as isize))).scale != surface_scale {
            ren_font_group_set_size(fonts, (**fonts).size, surface_scale);
            return;
        }
        i += 1;
    }
}
