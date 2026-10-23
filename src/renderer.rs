//! `src/renderer.c`: the glyph rasteriser and the drawing primitives.
//!
//! Fonts are shared as [`FontRef`]s: the Lua `Font` userdata holds one, and so does every
//! recorded draw command until the frame ends. A font group is a [`FontGroup`], the up to
//! `FONT_FALLBACK_MAX` fonts the C passed as a NULL-terminated `RenFont *` array.
//!
//! Glyphs are rasterised by the Rust-native backend in [`crate::font`]; this module keeps the
//! atlas bump-allocator, glyph cache, fallback and blend maths ported line by line from
//! `src/renderer.c`. Where the C relied on unsigned wrap-around or `char`/`float` truncation,
//! the Rust mirrors it with explicit casts and `wrapping_*` so debug builds do not panic and
//! release output matches.

use core::ffi::c_int;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;

use crate::font::Font;
use crate::rencache::{RenColor, RenTab, FONT_FALLBACK_MAX};
use crate::renwindow::{rect_empty, rect_intersection, RenRect, RenSurface, A, B, G, R};

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

impl Coverage {
    /// `count` pixels of atlas row `line`, starting `first` pixels in, or `None` if that is not
    /// inside the surface.
    fn span(&self, line: u32, first: usize, count: usize, bytes_per_pixel: usize) -> Option<&[u8]> {
        let start = line as usize * self.pitch as usize + first * bytes_per_pixel;
        self.pixels.get(start..start + count * bytes_per_pixel)
    }
}

/// One atlas surface: its coverage plus the bottom (`y1`) of the last glyph placed in it. The C
/// keeps a pointer to that glyph's metric in an SDL property and reads only its `y1`, which never
/// changes once the glyph is placed.
struct AtlasSurface {
    surface: Coverage,
    last_y1: Option<u32>,
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

/// `struct RenFont`.
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

pub type FontRef = Rc<RefCell<RenFont>>;

/// A font group: up to `FONT_FALLBACK_MAX` fonts, never empty, tried in order for each glyph.
#[derive(Clone)]
pub struct FontGroup {
    fonts: [Option<FontRef>; FONT_FALLBACK_MAX],
}

impl FontGroup {
    /// The first `FONT_FALLBACK_MAX` of `fonts`. `None` when there are none.
    pub fn new(fonts: impl IntoIterator<Item = FontRef>) -> Option<Self> {
        let mut group = FontGroup { fonts: Default::default() };
        for (slot, font) in group.fonts.iter_mut().zip(fonts) {
            *slot = Some(font);
        }
        group.fonts[0].is_some().then_some(group)
    }

    pub fn iter(&self) -> impl Iterator<Item = &FontRef> {
        self.fonts.iter().map_while(Option::as_ref)
    }

    fn first(&self) -> &FontRef {
        self.fonts[0].as_ref().expect("a font group is never empty")
    }

    fn get(&self, i: usize) -> &FontRef {
        self.fonts[i].as_ref().expect("an index from this group")
    }

    /// The fonts' identities, which is what the C hashed (it hashed the pointers).
    pub fn identities(&self) -> [usize; FONT_FALLBACK_MAX] {
        std::array::from_fn(|i| self.fonts[i].as_ref().map_or(0, |f| Rc::as_ptr(f) as usize))
    }
}

/// Where a glyph's coverage lives: `atlas[format][atlas_idx].surfaces[surface_idx]`.
#[derive(Clone, Copy)]
struct SurfaceIdx {
    format: usize,
    atlas: usize,
    surface: usize,
}

impl SurfaceIdx {
    fn of(metric: &GlyphMetric) -> Self {
        SurfaceIdx { format: metric.format as usize, atlas: metric.atlas_idx as usize, surface: metric.surface_idx as usize }
    }
}

impl RenFont {
    fn coverage(&self, at: SurfaceIdx) -> &Coverage {
        &self.glyphs.atlas[at.format][at.atlas].surfaces[at.surface].surface
    }

    pub fn path(&self) -> &CStr {
        &self.path
    }

    pub fn height(&self) -> c_int {
        self.height as c_int
    }
}

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

/// `utf8_to_codepoint`: consume one (possibly malformed) sequence starting at `p`, return the
/// codepoint and the next index. Like the C, a sequence cut short by the end reads the string's
/// terminating NUL as its last continuation byte, and the next index may then pass `text.len()`.
fn utf8_to_codepoint(text: &[u8], p: usize) -> (u32, usize) {
    let byte = text[p];
    let (mut res, mut n): (u32, u32) = match byte & 0xf0 {
        0xf0 => ((byte & 0x07) as u32, 3),
        0xe0 => ((byte & 0x0f) as u32, 2),
        0xd0 | 0xc0 => ((byte & 0x1f) as u32, 1),
        _ => (byte as u32, 0),
    };
    let mut up = p;
    while up < text.len() && n > 0 {
        n -= 1;
        up += 1;
        res = (res << 6) | (text.get(up).copied().unwrap_or(0) & 0x3f) as u32;
    }
    (res, up + 1)
}

fn is_whitespace(codepoint: u32) -> bool {
    match codepoint {
        0x20 | 0x85 | 0xA0 | 0x1680 | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 => true,
        _ => (0x9..=0xD).contains(&codepoint) || (0x2000..=0x200A).contains(&codepoint),
    }
}

// --------------------------------------------------------------- glyph cache

fn font_get_glyph_id(font: &mut RenFont, codepoint: u32) -> u32 {
    if codepoint > MAX_UNICODE {
        return 0;
    }
    let row = codepoint as usize / CHARMAP_COL;
    let col = codepoint as usize - row * CHARMAP_COL;
    let face = &font.face;
    let cells = font.charmap.rows[row].get_or_insert_with(|| Box::new([0u32; CHARMAP_COL]));
    if cells[col] == 0 {
        let glyph_id = face.glyph_id(codepoint);
        // 0 marks "not available"; store the sentinel so the row is only probed once.
        cells[col] = if glyph_id == 0 { u32::MAX } else { glyph_id };
    }
    if cells[col] == u32::MAX {
        0
    } else {
        cells[col]
    }
}

/// `font_allocate_glyph_surface`: pick or create the atlas surface with the least waste, and
/// place `metric` in it (its `atlas_idx`, `surface_idx`, `y0` and `y1`).
fn font_allocate_glyph_surface(
    font: &mut RenFont,
    glyph_format: usize,
    bitmap_rows: c_int,
    metric: &mut GlyphMetric,
) {
    let atlas_idx = match font.glyphs.atlas[glyph_format].iter().position(|atlas| atlas.width >= metric.x1) {
        Some(i) => i,
        None => {
            font.glyphs.atlas[glyph_format].push(GlyphAtlas {
                width: metric.x1 + FONT_WIDTH_OVERFLOW_PX,
                surfaces: Vec::new(),
            });
            font.glyphs.bytesize += std::mem::size_of::<GlyphAtlas>();
            font.glyphs.atlas[glyph_format].len() - 1
        }
    };
    metric.atlas_idx = atlas_idx as u16;

    let (line_px, size) = (font.line_px, font.size);
    let atlas = &mut font.glyphs.atlas[glyph_format][atlas_idx];

    // Find the surface with the minimum height that can fit the glyph (limited to the last 100).
    let mut surface_idx: c_int = -1;
    let max_surface_idx = atlas.surfaces.len() as c_int - 100;
    let mut min_waste = c_int::MAX;
    let mut i = atlas.surfaces.len() as c_int - 1;
    while i >= 0 && i > max_surface_idx {
        let entry = &atlas.surfaces[i as usize];
        let last_y1 = entry.last_y1.expect("atlas surface metric");
        let new_min_waste = entry.surface.h - last_y1 as c_int;
        if new_min_waste >= metric.y1 as c_int && new_min_waste < min_waste {
            surface_idx = i;
            min_waste = new_min_waste;
        }
        i -= 1;
    }

    if surface_idx < 0 {
        let mut h = FONT_HEIGHT_OVERFLOW_PX + line_px;
        if h <= FONT_HEIGHT_OVERFLOW_PX {
            h += bitmap_rows;
        }
        if h <= FONT_HEIGHT_OVERFLOW_PX {
            h += size as c_int;
        }
        let pitch = atlas.width as c_int * glyphformat_bytes_per_pixel(glyph_format);
        let h = GLYPHS_PER_ATLAS * h;
        atlas.surfaces.push(AtlasSurface {
            surface: Coverage {
                pixels: vec![0; pitch as usize * h as usize],
                pitch,
                h,
            },
            last_y1: None,
        });
        font.glyphs.bytesize += std::mem::size_of::<AtlasSurface>()
            + atlas.width as usize * GLYPHS_PER_ATLAS as usize * h as usize * glyph_format;
        surface_idx = atlas.surfaces.len() as c_int - 1;
    }

    metric.surface_idx = surface_idx as u16;
    let entry = &mut atlas.surfaces[surface_idx as usize];
    if let Some(last_y1) = entry.last_y1 {
        metric.y0 = last_y1;
        metric.y1 = metric.y1.wrapping_add(last_y1);
    }
    entry.last_y1 = Some(metric.y1);
}

/// The cached metric slot of a glyph's subpixel index. Its row exists once
/// `font_load_glyph_metric` has succeeded for the glyph.
fn metric_slot(font: &mut RenFont, glyph_id: u32, bitmap_idx: usize) -> &mut GlyphMetric {
    let row = glyph_id as usize / GLYPHMAP_COL;
    let col = glyph_id as usize - row * GLYPHMAP_COL;
    &mut font.glyphs.metrics[bitmap_idx][row].as_mut().expect("a loaded glyph metric row")[col]
}

/// `font_load_glyph_metric`: ensure all subpixel metrics (xadvance) for a glyph are cached.
/// Returns the metric of `bitmap_idx`, or `None` if the backend has no advance for the glyph.
fn font_load_glyph_metric(font: &mut RenFont, glyph_id: u32, bitmap_idx: usize) -> Option<GlyphMetric> {
    let row = glyph_id as usize / GLYPHMAP_COL;
    let col = glyph_id as usize - row * GLYPHMAP_COL;
    let bitmaps = font_bitmap_count(font);

    let needs_load = font.glyphs.metrics[0][row]
        .as_ref()
        .is_none_or(|metrics| metrics[col].flags & EGLYPH_XADVANCE == 0);
    if needs_load {
        let advance = font.face.advance(glyph_id)?;
        for i in 0..bitmaps {
            if font.glyphs.metrics[i][row].is_none() {
                font.glyphs.metrics[i][row] = Some(Box::new([GlyphMetric::default(); GLYPHMAP_COL]));
                font.glyphs.bytesize += std::mem::size_of::<GlyphMetric>() * GLYPHMAP_COL;
            }
            let metric = &mut font.glyphs.metrics[i][row].as_mut().unwrap()[col];
            metric.flags |= EGLYPH_XADVANCE;
            metric.xadvance = advance;
        }
    }

    Some(*metric_slot(font, glyph_id, bitmap_idx))
}

/// `font_load_glyph_bitmap`: render one subpixel index and place it in an atlas surface.
/// Returns where its coverage is, or `None` if it has no metric or could not be rendered.
fn font_load_glyph_bitmap(font: &mut RenFont, glyph_id: u32, bitmap_idx: usize) -> Option<SurfaceIdx> {
    let mut metric = font_load_glyph_metric(font, glyph_id, bitmap_idx)?;
    if metric.flags & EGLYPH_BITMAP != 0 {
        return Some(SurfaceIdx::of(&metric));
    }

    let x_shift = bitmap_idx as f32 / SUBPIXEL_BITMAPS_CACHED as f32;
    let image = font.face.render(glyph_id, x_shift, font.antialiasing, font.style)?;
    let format = if image.subpixel {
        EGLYPH_FORMAT_SUBPIXEL
    } else {
        EGLYPH_FORMAT_GRAYSCALE
    };
    metric.x1 = image.width;
    metric.y1 = image.rows;
    // The FreeType mono copy writes column c at atlas x = c + 1; keep mono placement identical.
    metric.bitmap_left = image.left + (font.antialiasing == FONT_ANTIALIASING_NONE) as i32;
    metric.bitmap_top = image.top;
    metric.flags |= EGLYPH_BITMAP;
    metric.format = format as u8;
    font_allocate_glyph_surface(font, format, image.rows as c_int, &mut metric);
    *metric_slot(font, glyph_id, bitmap_idx) = metric;

    let at = SurfaceIdx::of(&metric);
    let coverage = &mut font.glyphs.atlas[at.format][at.atlas].surfaces[at.surface].surface;
    let row_bytes = image.width as usize * glyphformat_bytes_per_pixel(format) as usize;
    for line in 0..image.rows as usize {
        let target = coverage.pitch as usize * (line + metric.y0 as usize);
        coverage.pixels[target..target + row_bytes]
            .copy_from_slice(&image.data[line * row_bytes..(line + 1) * row_bytes]);
    }
    Some(at)
}

/// What `font_group_get_glyph` found: the font (an index into the group), the glyph's metric
/// as it is after any bitmap load, and where its coverage is when that was asked for and exists.
struct Glyph {
    font: usize,
    metric: Option<GlyphMetric>,
    surface: Option<SurfaceIdx>,
}

/// `font_group_get_glyph`: pick the first font in the group that has the codepoint.
fn font_group_get_glyph(fonts: &FontGroup, codepoint: u32, mut subpixel_idx: c_int, want_surface: bool) -> Glyph {
    if subpixel_idx < 0 {
        subpixel_idx += SUBPIXEL_BITMAPS_CACHED as c_int;
    }

    let mut font = 0;
    let mut glyph_id: u32 = 0;
    for (i, candidate) in fonts.iter().enumerate() {
        font = i;
        glyph_id = font_get_glyph_id(&mut candidate.borrow_mut(), codepoint);
        // Use the first font that has a glyph, but whitespace always uses the first font.
        if glyph_id != 0 || is_whitespace(codepoint) {
            break;
        }
    }

    let mut chosen = fonts.get(font).borrow_mut();
    let subpixel_idx = if font_is_subpixel(&chosen) { subpixel_idx } else { 0 };
    let metric = font_load_glyph_metric(&mut chosen, glyph_id, subpixel_idx as usize);
    // Fall back to the box-drawing character, unless this is the box or whitespace.
    if metric.is_none_or(|m| m.flags == 0) && codepoint != 0x25A1 && !is_whitespace(codepoint) {
        drop(chosen);
        return font_group_get_glyph(fonts, 0x25A1, subpixel_idx, want_surface);
    }
    let mut glyph = Glyph { font, metric, surface: None };
    if want_surface && metric.is_some() {
        glyph.surface = font_load_glyph_bitmap(&mut chosen, glyph_id, subpixel_idx as usize);
        // The bitmap load fills in the metric's bitmap fields; report it as it is now.
        glyph.metric = Some(*metric_slot(&mut chosen, glyph_id, subpixel_idx as usize));
    }
    glyph
}

fn font_clear_glyph_cache(font: &mut RenFont) {
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

/// `font_set_face_metrics`. Needs `font.size` and the font backend set.
fn font_set_face_metrics(font: &mut RenFont) -> c_int {
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
pub fn ren_font_load(
    path: &CStr,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: u8,
) -> Result<RenFont, String> {
    let face = Font::load(path)?;

    let mut font = RenFont {
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
        path: path.to_owned(),
    };

    if font_set_face_metrics(&mut font) != 0 {
        // The backend hides the hinting-failure detail behind FreeType's pixel-size error.
        return Err("invalid pixel size".to_string());
    }
    Ok(font)
}

/// `ren_font_copy`; `-1` means "keep the original".
pub fn ren_font_copy(
    font: &RenFont,
    size: f32,
    antialiasing: c_int,
    hinting: c_int,
    style: c_int,
) -> Result<RenFont, String> {
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
    ren_font_load(&font.path, size, antialiasing, hinting, style as u8)
}

pub fn ren_font_group_set_tab_size(fonts: &FontGroup, n: c_int) {
    for font in fonts.iter() {
        font.borrow_mut().tab_size = n as u16;
    }
}

pub fn ren_font_group_get_tab_size(fonts: &FontGroup) -> c_int {
    fonts.first().borrow().tab_size as c_int
}

pub fn ren_font_group_get_size(fonts: &FontGroup) -> f32 {
    fonts.first().borrow().size
}

/// `ren_font_group_set_size`.
pub fn ren_font_group_set_size(fonts: &FontGroup, size: f32) {
    for font in fonts.iter() {
        let font = &mut *font.borrow_mut();
        font_clear_glyph_cache(font);
        font.size = size;
        font.tab_size = 2;
        font_set_face_metrics(font);
    }
}

pub fn ren_font_group_get_height(fonts: &FontGroup) -> c_int {
    fonts.first().borrow().height as c_int
}

/// `font_get_xadvance`.
fn font_get_xadvance(
    font: &RenFont,
    codepoint: u32,
    metric: Option<&GlyphMetric>,
    curr_x: f64,
    tab: RenTab,
) -> f32 {
    // Some fonts (e.g. Unifont) provide xadvance for whitespace; ignore it.
    if let Some(metric) = metric.filter(|m| !is_whitespace(codepoint) && m.xadvance != 0.0) {
        return metric.xadvance;
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

/// One glyph of a [`GlyphRun`]: the codepoint, what the cache has for it, and where the pen is.
struct RunGlyph {
    codepoint: u32,
    glyph: Glyph,
    /// The pen before this glyph, in the run's own units (including the run's origin).
    pen: f64,
    /// How far this glyph moves the pen.
    advance: f32,
    /// This was the last glyph of the text (a sequence cut short does not count).
    at_end: bool,
}

/// The one traversal of a text: decode, pick the font, take its metric (and, when drawing, its
/// coverage), and advance the pen. Measuring and drawing walk the same glyphs the same way, so
/// what is drawn is where it was measured.
///
/// The pen is only advanced past a glyph when the next one is asked for, so a caller that stops
/// at a glyph reads the pen as it was before that glyph.
struct GlyphRun<'a> {
    fonts: &'a FontGroup,
    text: &'a [u8],
    p: usize,
    origin: f64,
    pen: f64,
    pending: f64,
    tab: RenTab,
    /// Drawing: load coverage, and choose the subpixel bitmap from the pen's fraction. Measuring
    /// needs neither and always uses the first subpixel bitmap.
    drawing: bool,
}

impl<'a> GlyphRun<'a> {
    fn measure(fonts: &'a FontGroup, text: &'a [u8], tab: RenTab) -> Self {
        Self { fonts, text, p: 0, origin: 0.0, pen: 0.0, pending: 0.0, tab, drawing: false }
    }

    /// `origin` is the pen's start, which tab stops are measured from.
    fn draw(fonts: &'a FontGroup, text: &'a [u8], origin: f64, tab: RenTab) -> Self {
        Self { fonts, text, p: 0, origin, pen: origin, pending: 0.0, tab, drawing: true }
    }

    /// The pen before the glyph last returned, or after the last glyph once the run is done.
    fn pen(&self) -> f64 {
        self.pen
    }
}

impl Iterator for GlyphRun<'_> {
    type Item = RunGlyph;

    fn next(&mut self) -> Option<RunGlyph> {
        self.pen += self.pending;
        self.pending = 0.0;
        if self.p >= self.text.len() {
            return None;
        }
        let (codepoint, next) = utf8_to_codepoint(self.text, self.p);
        self.p = next;
        let subpixel_idx = if self.drawing { (self.pen % 1.0 * SUBPIXEL_BITMAPS_CACHED as f64) as c_int } else { 0 };
        let glyph = font_group_get_glyph(self.fonts, codepoint, subpixel_idx, self.drawing);
        let advance = font_get_xadvance(
            &self.fonts.first().borrow(),
            codepoint,
            glyph.metric.as_ref(),
            self.pen - self.origin,
            self.tab,
        );
        self.pending = advance as f64;
        Some(RunGlyph { codepoint, glyph, pen: self.pen, advance, at_end: next == self.text.len() })
    }
}

/// `ren_font_group_get_width`. `x_offset`, when asked for, receives the first glyph's
/// `bitmap_left` (or 0).
pub fn ren_font_group_get_width(fonts: &FontGroup, text: &[u8], tab: RenTab, x_offset: Option<&mut c_int>) -> f64 {
    let mut run = GlyphRun::measure(fonts, text, tab);
    let mut first_left: Option<c_int> = None;
    for placed in run.by_ref() {
        if first_left.is_none() {
            first_left = placed.glyph.metric.map(|m| m.bitmap_left);
        }
    }
    if let Some(x_offset) = x_offset {
        *x_offset = first_left.unwrap_or(0);
    }
    run.pen()
}

// --------------------------------------------------------------- text drawing

/// `ren_draw_text`.
pub fn ren_draw_text(
    rs: &mut RenSurface,
    fonts: &FontGroup,
    text: &[u8],
    x: f32,
    y: c_int,
    color: RenColor,
    tab: RenTab,
) -> f64 {
    let clip = rs.clip;

    let surface_scale = rs.scale;
    let y = y * surface_scale; // scaled below; `y / surface_scale` recovers the point value
    let clip_end_x = clip.x + clip.width;
    let clip_end_y = clip.y + clip.height;

    let first = fonts.first();
    let mut last: Option<usize> = None;
    let mut last_pen_x = x as f64;
    let (underline, strikethrough, baseline) = {
        let first = first.borrow();
        (first.style & FONT_STYLE_UNDERLINE != 0, first.style & FONT_STYLE_STRIKETHROUGH != 0, first.baseline)
    };

    let mut run = GlyphRun::draw(fonts, text, (x * surface_scale as f32) as f64, tab);
    for RunGlyph { codepoint, glyph, pen: pen_x, advance: adv, at_end } in run.by_ref() {
        let Some(metric) = glyph.metric else {
            break;
        };
        let font = fonts.get(glyph.font);

        let mut start_x = (pen_x.floor() as c_int).wrapping_add(metric.bitmap_left);
        let end_x = (metric.x1 as c_int).wrapping_add(start_x);
        let mut glyph_end = metric.x1 as c_int;
        let mut glyph_start: c_int = 0;

        if glyph.surface.is_none() && !is_whitespace(codepoint) {
            let space_advance = font.borrow().space_advance;
            ren_draw_rect(
                rs,
                RenRect {
                    x: start_x.wrapping_add(1),
                    y,
                    width: (space_advance - 1.0) as c_int,
                    height: ren_font_group_get_height(fonts),
                },
                color,
            );
        }

        if let Some(at) = glyph.surface.filter(|_| {
            !is_whitespace(codepoint) && color.a > 0 && end_x >= clip.x && start_x < clip_end_x
        }) {
            let font = font.borrow();
            let coverage = font.coverage(at);
            let font_bytes_per_pixel = glyphformat_bytes_per_pixel(metric.format as usize) as usize;
            let subpixel = metric.format as usize == EGLYPH_FORMAT_SUBPIXEL;

            for line in metric.y0..metric.y1 {
                let target_y = line
                    .wrapping_sub(metric.y0)
                    .wrapping_add(y as u32)
                    .wrapping_sub(metric.bitmap_top as u32)
                    .wrapping_add((baseline as c_int * surface_scale) as u32)
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
                if glyph_end <= glyph_start {
                    continue;
                }

                // Both ranges lie inside their stores: the clip is inside the surface and the
                // glyph inside its atlas row. A miss would be a bug; skip the row, never crash.
                let count = (glyph_end - glyph_start) as usize;
                let Some(source) = coverage.span(line, glyph_start as usize, count, font_bytes_per_pixel) else {
                    continue;
                };
                let Some(destination) = rs.row(target_y, start_x, count) else {
                    continue;
                };
                if subpixel {
                    for (pixel, src) in destination.iter_mut().zip(source.as_chunks::<3>().0) {
                        *pixel = blend_glyph_pixel(*pixel, src[0] as u32, src[1] as u32, src[2] as u32, color);
                    }
                } else {
                    for (pixel, &src) in destination.iter_mut().zip(source) {
                        *pixel = blend_glyph_pixel(*pixel, src as u32, src as u32, src as u32, color);
                    }
                }
            }
        }

        match last {
            None => last = Some(glyph.font),
            Some(last_font) if !Rc::ptr_eq(fonts.get(last_font), font) || at_end => {
                let local_pen_x = if at_end { pen_x + adv as f64 } else { pen_x };
                let (height, underline_thickness) = {
                    let last = fonts.get(last_font).borrow();
                    (last.height as c_int, last.underline_thickness as c_int)
                };
                let line_at = |y_offset: c_int| RenRect {
                    x: last_pen_x as c_int,
                    y: y / surface_scale + y_offset,
                    width: ((local_pen_x - last_pen_x) / surface_scale as f64) as c_int,
                    height: underline_thickness * surface_scale,
                };
                if underline {
                    ren_draw_rect(rs, line_at(height - 1), color);
                }
                if strikethrough {
                    ren_draw_rect(rs, line_at(height / 2), color);
                }
                last = Some(glyph.font);
                last_pen_x = pen_x;
            }
            Some(_) => {}
        }
    }

    run.pen() / surface_scale as f64
}

/// One glyph pixel over `destination`: coverage `src_*` of `color`, per channel. Not
/// SDL_GetRGBA: the C unpacks manually for speed, and the value is truncated to a byte before
/// blending.
#[inline]
fn blend_glyph_pixel(destination: u32, src_r: u32, src_g: u32, src_b: u32, color: RenColor) -> u32 {
    let dst_r = (R.get(destination) as u8) as u32;
    let dst_g = (G.get(destination) as u8) as u32;
    let dst_b = (B.get(destination) as u8) as u32;
    let dst_a = (A.get(destination) as u8) as u32;
    let a = color.a as u32;

    let r = (color.r as u32 * src_r * a + dst_r * (65025 - src_r * a) + 32767) / 65025;
    let g = (color.g as u32 * src_g * a + dst_g * (65025 - src_g * a) + 32767) / 65025;
    let b = (color.b as u32 * src_b * a + dst_b * (65025 - src_b * a) + 32767) / 65025;

    (dst_a << A.shift) | (r << R.shift) | (g << G.shift) | (b << B.shift)
}

// --------------------------------------------------------------- rectangles

/// `ren_draw_rect`.
pub fn ren_draw_rect(rs: &mut RenSurface, rect: RenRect, color: RenColor) {
    if color.a == 0 {
        return;
    }

    let surface_scale = rs.scale;
    let dest_rect = RenRect {
        x: rect.x * surface_scale,
        y: rect.y * surface_scale,
        width: rect.width * surface_scale,
        height: rect.height * surface_scale,
    };
    // SDL_FillSurfaceRect intersects with the clip itself; the blend path clipped manually
    // because scaled blits clip differently. Either way: SDL's rect intersection.
    let Some(clipped) = rect_intersection(dest_rect, rs.clip).filter(|r| !rect_empty(*r)) else {
        return;
    };
    if rs.w == 0 || rs.h == 0 {
        return;
    }
    let rows = clipped.y..clipped.y + clipped.height;
    let width = clipped.width as usize;

    if color.a == 0xff {
        // `SDL_MapSurfaceRGB` for an 8-bit-per-channel format: alpha opaque under its mask.
        let pixel = ((color.r as u32) << R.shift)
            | ((color.g as u32) << G.shift)
            | ((color.b as u32) << B.shift)
            | ((0xff << A.shift) & A.mask);
        for y in rows {
            if let Some(row) = rs.row(y, clipped.x, width) {
                row.fill(pixel);
            }
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
        for y in rows {
            let Some(row) = rs.row(y, clipped.x, width) else { continue };
            for pixel in row {
                let d = *pixel;
                let mut out = (blend(color.r as u32, R.get(d)) << R.shift)
                    | (blend(color.g as u32, G.get(d)) << G.shift)
                    | (blend(color.b as u32, B.get(d)) << B.shift);
                if A.mask != 0 {
                    out |= blend(255, A.get(d)) << A.shift;
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
        let err = ren_font_load(c"/nonexistent/font.ttf", 16.0, FONT_ANTIALIASING_SUBPIXEL, 1, 0)
            .err()
            .expect("a missing font must fail");
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
        let font = ren_font_load(&path, 16.0, FONT_ANTIALIASING_SUBPIXEL, 1, 0)
            .ok()
            .expect("the staged fixture must load");
        std::fs::remove_file(&tmp).expect("drop the staged fixture");
        // The copy re-loads by path, so the vanished file surfaces through `ren_font_copy`.
        let err = ren_font_copy(&font, 16.0, -1, -1, -1).err().expect("a vanished font must fail");
        assert!(err.contains(tmp.to_str().unwrap()), "{err}");
    }

    fn fixture_group(antialiasing: c_int) -> FontGroup {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/fonts");
        let load = |name: &str| {
            let path = CString::new(dir.join(name).to_str().unwrap()).unwrap();
            let font = ren_font_load(&path, 16.0, antialiasing, 1, 0).ok().expect("fixture font loads");
            Rc::new(RefCell::new(font))
        };
        FontGroup::new([load("JetBrainsMono-Regular.ttf"), load("icons.ttf")]).unwrap()
    }

    /// Everything the traversal decides, for `text`: measured width and offset, the pen after
    /// drawing, and a hash of the pixels drawn (at a fractional x, clipped on both sides).
    fn geometry_snapshot(antialiasing: c_int, text: &[u8], tab: RenTab) -> (f64, c_int, f64, u64) {
        let fonts = fixture_group(antialiasing);
        let mut x_offset = 0;
        let width = ren_font_group_get_width(&fonts, text, tab, Some(&mut x_offset));
        let (w, h) = (400, 40);
        let mut pixels = vec![0xff20_2020u32; (w * h) as usize];
        let mut rs = RenSurface {
            pixels: &mut pixels,
            w,
            h,
            clip: RenRect { x: 7, y: 2, width: 380, height: 36 },
            scale: 1,
        };
        let color = RenColor { r: 230, g: 200, b: 90, a: 255 };
        let pen = ren_draw_text(&mut rs, &fonts, text, 10.3, 5, color, tab);
        let hash = pixels.iter().fold(0xcbf29ce484222325u64, |h, &p| (h ^ p as u64).wrapping_mul(0x100000001b3));
        (width, x_offset, pen, hash)
    }

    const SAMPLES: &[&[u8]] = &[
        b"fn main() { 1 + 1 }",
        b"a\tbb\tccc\t d",
        "caf\u{e9} \u{4e2d}\u{6587} \u{1f600}\u{3000}x".as_bytes(),
        b"trailing cut \xc3",
        b"\xff\xfe bad bytes",
        b"",
    ];

    #[test]
    fn text_geometry_is_unchanged() {
        let mut out = String::new();
        for aa in [FONT_ANTIALIASING_NONE, 1, FONT_ANTIALIASING_SUBPIXEL] {
            for tab in [RenTab { offset: f64::NAN }, RenTab { offset: 3.5 }] {
                for text in SAMPLES {
                    let (w, o, p, h) = geometry_snapshot(aa, text, tab);
                    out += &format!("{w:.4} {o} {p:.4} {h:016x}\n");
                }
            }
        }
        assert_eq!(out, GOLDEN);
    }

    /// Recorded from the traversal before it was shared between measuring and drawing: width,
    /// first glyph offset, pen after drawing and a pixel hash, per antialiasing mode, tab mode
    /// and sample.
    const GOLDEN: &str = "\
182.2812 0 192.5813 68cd600a7b0879a7
134.3125 0 144.6125 9ca8495b764c07b5
124.7500 0 135.0500 3dba89daaa87b63d
134.3125 0 144.6125 6a980367ba6715f5
92.7500 0 103.0500 3eee470d02951ab7
0.0000 0 10.3000 f4102e5ca2a90525
182.2812 0 192.5813 68cd600a7b0879a7
130.8125 0 141.1125 8f4e58bf5b8a87d5
124.7500 0 135.0500 3dba89daaa87b63d
134.3125 0 144.6125 6a980367ba6715f5
92.7500 0 103.0500 3eee470d02951ab7
0.0000 0 10.3000 f4102e5ca2a90525
182.2812 0 192.5813 cf2924af42dba23a
134.3125 0 144.6125 4cd34bd5f1be8b6f
124.7500 0 135.0500 924ceee8d4754040
134.3125 0 144.6125 c5839d332e1076c2
92.7500 0 103.0500 af4f8e1a01ca76f9
0.0000 0 10.3000 f4102e5ca2a90525
182.2812 0 192.5813 cf2924af42dba23a
130.8125 0 141.1125 15804e644ec61f17
124.7500 0 135.0500 924ceee8d4754040
134.3125 0 144.6125 c5839d332e1076c2
92.7500 0 103.0500 af4f8e1a01ca76f9
0.0000 0 10.3000 f4102e5ca2a90525
182.2812 0 192.5813 0028d9bddff21249
134.3125 0 144.6125 437063705397adca
124.7500 0 135.0500 7f4a49ed281067e4
134.3125 0 144.6125 6f0e2d87500ff685
92.7500 0 103.0500 a8af61bc851aa1f0
0.0000 0 10.3000 f4102e5ca2a90525
182.2812 0 192.5813 0028d9bddff21249
130.8125 0 141.1125 08c871b526b62d5f
124.7500 0 135.0500 7f4a49ed281067e4
134.3125 0 144.6125 6f0e2d87500ff685
92.7500 0 103.0500 a8af61bc851aa1f0
0.0000 0 10.3000 f4102e5ca2a90525
";

    #[test]
    fn utf8_reads_the_terminator_for_a_cut_sequence() {
        assert_eq!(utf8_to_codepoint(b"A", 0), (0x41, 1));
        assert_eq!(utf8_to_codepoint("é".as_bytes(), 0), (0xe9, 2));
        // A lead byte at the end: the C reads its NUL terminator as the continuation.
        assert_eq!(utf8_to_codepoint(b"\xc3", 0), (0x03 << 6, 2));
    }
}
