//! Rust-native glyph backend for `renderer.rs`: Skrifa 0.48 parses, measures and hints
//! (`Engine::Auto(None)`, mirroring FreeType's FORCE_AUTOHINT); Zeno 0.3 rasterizes
//! grayscale/subpixel; tiny-skia 0.12 non-AA fill rasterizes mono. Bold and smoothing are a
//! stroke-and-fill approximation of FreeType's outline embolden, italic is the same shear.
//! Only face 0 of glyf/CFF/CFF2 fonts loads; other formats fail to load.

use core::ffi::c_int;
use std::ffi::CStr;

use std::ptr;

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, SmoothMode, Target,
};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Format, Mask, Point, Stroke, Vector};

// Values shared with renderer.rs (FONT_ANTIALIASING_*, FONT_HINTING_*, FONT_STYLE_*).
const AA_NONE: c_int = 0;
const AA_SUBPIXEL: c_int = 2;
const HINT_NONE: c_int = 0;
const HINT_SLIGHT: c_int = 1;
const STYLE_BOLD: u8 = 1;
const STYLE_ITALIC: u8 = 2;
const STYLE_SMOOTH: u8 = 8;
/// FreeType's embolden strength `1 << 5` in 26.6 is half a pixel of total growth (smoothing).
const EMBOLDEN_PX: f32 = 0.5;
/// User-approved: synthetic bold grows with size (0.75 px total at 15 px, ink close to real Bold faces) so it is
/// visible; FreeType's fixed half pixel was imperceptible on a HiDPI panel.
const BOLD_PER_PX: f32 = 1.0 / 20.0;
const MAX_GLYPH_PX: f32 = 4096.0;

/// A read-only, private file mapping: font bytes are paged in on demand and shared with the
/// page cache, like FreeType's stream reads, instead of being copied onto the heap.
struct FontData {
    ptr: *mut libc::c_void,
    len: usize,
}

impl FontData {
    fn open(path: &CStr) -> Result<Self, String> {
        // Errors keep SDL_IOFromFile's wording, so callers see the same messages as before.
        if path.is_empty() {
            return Err("Parameter 'file' is invalid".to_string());
        }
        unsafe {
            let fd = libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC);
            if fd < 0 {
                let error = CStr::from_ptr(libc::strerror(*libc::__errno_location()));
                return Err(format!("Couldn't open {}: {}", path.to_string_lossy(), error.to_string_lossy()));
            }
            let mut st: libc::stat = std::mem::zeroed();
            let len = if libc::fstat(fd, &mut st) == 0 {
                if (st.st_mode & libc::S_IFMT) == libc::S_IFDIR {
                    libc::close(fd);
                    return Err(format!("{} is a directory", path.to_string_lossy()));
                }
                st.st_size as usize
            } else {
                0
            };
            let ptr = if len == 0 {
                libc::MAP_FAILED
            } else {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ,
                    libc::MAP_PRIVATE,
                    fd,
                    0,
                )
            };
            libc::close(fd);
            if ptr == libc::MAP_FAILED {
                return Err("unknown file format".to_string());
            }
            Ok(Self { ptr, len })
        }
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the mapping lives as long as `self` and is never written.
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}

impl Drop for FontData {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr, self.len) };
    }
}

pub struct Font {
    data: FontData,
    pixel_size: f32,
    hint: Option<HintingInstance>,
}

pub struct FaceMetrics {
    pub height: f32,
    pub ascent: f32,
    pub underline_thickness: f32,
    pub space_advance: f32,
    pub line_px: c_int,
}

/// A rendered glyph: 1 byte per pixel (gray/mono) or 3 (RGB subpixel), rows top-down.
pub struct GlyphImage {
    pub width: u32,
    pub rows: u32,
    pub left: i32,
    pub top: i32,
    pub subpixel: bool,
    pub data: Vec<u8>,
}

/// FreeType reports advances in 26.6; rounding to 1/64 keeps measured widths comparable.
fn quantize(v: f32) -> f32 {
    (v * 64.0).round() / 64.0
}

impl Font {
    fn font(&self) -> FontRef<'_> {
        FontRef::from_index(self.data.bytes(), 0).expect("face 0 validated at load")
    }

    pub fn load(path: &CStr) -> Result<Self, String> {
        let data = FontData::open(path)?;
        {
            let font = FontRef::from_index(data.bytes(), 0)
                .map_err(|_| "unknown file format".to_string())?;
            if font.outline_glyphs().format().is_none() {
                return Err("unknown file format".into());
            }
        }
        Ok(Self {
            data,
            pixel_size: 0.0,
            hint: None,
        })
    }

    /// Sets the size and hinting route; `metrics_size` is the logical size, `pixel_size` the
    /// raster size. They are equal now that the drawing surface is always at scale 1.
    pub fn configure(
        &mut self,
        metrics_size: f32,
        pixel_size: f32,
        antialiasing: c_int,
        hinting: c_int,
    ) -> Result<FaceMetrics, String> {
        // FT_Set_Pixel_Sizes clamps to 1..=0xFFFF (the caller already truncated to an integer).
        let pixel_size = pixel_size.clamp(1.0, 65535.0);
        let target = if hinting == HINT_NONE {
            None
        } else if antialiasing == AA_NONE {
            Some(Target::Mono)
        } else if hinting == HINT_SLIGHT {
            Some(Target::from(SmoothMode::Light))
        } else {
            Some(Target::from(SmoothMode::Normal))
        };
        let (line, metrics, space) = {
            let font = self.font();
            let location = LocationRef::default();
            let hint = match target {
                Some(target) => Some(
                    HintingInstance::new(
                        &font.outline_glyphs(),
                        Size::new(pixel_size),
                        location,
                        HintingOptions {
                            engine: Engine::Auto(None),
                            target,
                        },
                    )
                    .map_err(|e| format!("hinting setup failed ({e})"))?,
                ),
                None => None,
            };
            let m = font.metrics(Size::unscaled(), location);
            let scale = metrics_size / m.units_per_em as f32;
            let raster = font.metrics(Size::new(pixel_size), location);
            let space_id = font.charmap().map(' ').unwrap_or(GlyphId::new(0));
            let space = font
                .glyph_metrics(Size::new(pixel_size), location)
                .advance_width(space_id)
                .unwrap_or(0.0);
            self.hint = hint;
            (
                (raster.ascent - raster.descent + raster.leading) as c_int,
                (m, scale),
                space,
            )
        };
        self.pixel_size = pixel_size;
        let (metrics, scale) = metrics;
        Ok(FaceMetrics {
            height: (metrics.ascent - metrics.descent + metrics.leading) * scale,
            ascent: metrics.ascent * scale,
            underline_thickness: metrics.underline.map_or(0.0, |d| d.thickness) * scale,
            space_advance: quantize(space),
            line_px: line,
        })
    }

    pub fn glyph_id(&self, codepoint: u32) -> u32 {
        self.font()
            .charmap()
            .map(codepoint)
            .map_or(0, |g| g.to_u32())
    }

    /// Unhinted advance at the raster size (the reference strips hinting for advances too).
    pub fn advance(&self, glyph_id: u32) -> Option<f32> {
        self.font()
            .glyph_metrics(Size::new(self.pixel_size), LocationRef::default())
            .advance_width(GlyphId::new(glyph_id))
            .map(quantize)
    }

    /// `x_shift` is the subpixel pen offset in pixels; it is applied after hinting, as the
    /// reference translates the loaded outline.
    pub fn render(
        &self,
        glyph_id: u32,
        x_shift: f32,
        antialiasing: c_int,
        style: u8,
    ) -> Option<GlyphImage> {
        let font = self.font();
        let outlines = font.outline_glyphs();
        let glyph = outlines.get(GlyphId::new(glyph_id))?;
        let mut pen = Pen {
            cmds: Vec::new(),
            shift: x_shift,
            shear: if style & STYLE_ITALIC != 0 { 0.25 } else { 0.0 },
        };
        let settings = match &self.hint {
            Some(hint) => DrawSettings::hinted(hint, false),
            None => DrawSettings::unhinted(Size::new(self.pixel_size), LocationRef::default()),
        };
        glyph.draw(settings, &mut pen).ok()?;
        if pen.cmds.is_empty() {
            return None;
        }
        let mut embolden = 0.0;
        if style & STYLE_SMOOTH != 0 {
            embolden += EMBOLDEN_PX;
        }
        if style & STYLE_BOLD != 0 {
            embolden += self.pixel_size * BOLD_PER_PX;
        }

        let bounds = zeno::bounds(&pen.cmds, zeno::Fill::NonZero, None);
        // One spare pixel covers the stroke growth and Zeno's +-0.3 px subpixel sampling.
        let pad = embolden / 2.0 + 1.0;
        let min_x = (bounds.min.x - pad).floor();
        let min_y = (bounds.min.y - pad).floor();
        let max_x = (bounds.max.x + pad).ceil();
        let max_y = (bounds.max.y + pad).ceil();
        if !(min_x.is_finite() && max_y.is_finite())
            || max_x - min_x > MAX_GLYPH_PX
            || max_y - min_y > MAX_GLYPH_PX
        {
            return None;
        }
        let (w, h) = ((max_x - min_x) as u32, (max_y - min_y) as u32);
        let offset = Vector::new(-min_x, -min_y);

        let data = if antialiasing == AA_NONE {
            mono(&pen.cmds, w, h, -min_x, -min_y, embolden)?
        } else {
            let format = if antialiasing == AA_SUBPIXEL {
                Format::Subpixel
            } else {
                Format::Alpha
            };
            let mut fill = vec![0u8; format.buffer_size(w, h)];
            Mask::new(&pen.cmds)
                .format(format)
                .size(w, h)
                .offset(offset)
                .render_into(&mut fill, None);
            if embolden > 0.0 {
                let mut stroke = vec![0u8; fill.len()];
                Mask::new(&pen.cmds)
                    .style(Stroke::new(embolden))
                    .format(format)
                    .size(w, h)
                    .offset(offset)
                    .render_into(&mut stroke, None);
                fill.iter_mut()
                    .zip(stroke)
                    .for_each(|(a, b)| *a = (*a).max(b));
            }
            if antialiasing == AA_SUBPIXEL {
                // Zeno writes RGBA; the atlas is RGB24.
                fill.chunks_exact(4)
                    .flat_map(|px| [px[0], px[1], px[2]])
                    .collect()
            } else {
                fill
            }
        };
        Some(GlyphImage {
            width: w,
            rows: h,
            left: min_x as i32,
            top: -min_y as i32,
            subpixel: antialiasing == AA_SUBPIXEL,
            data,
        })
    }
}

/// tiny-skia non-AA fill (the user-accepted mono construction), plus the stroke for embolden.
fn mono(cmds: &[Command], w: u32, h: u32, dx: f32, dy: f32, embolden: f32) -> Option<Vec<u8>> {
    let mut builder = tiny_skia::PathBuilder::new();
    for command in cmds {
        match *command {
            Command::MoveTo(p) => builder.move_to(p.x + dx, p.y + dy),
            Command::LineTo(p) => builder.line_to(p.x + dx, p.y + dy),
            Command::QuadTo(c, p) => builder.quad_to(c.x + dx, c.y + dy, p.x + dx, p.y + dy),
            Command::CurveTo(c0, c1, p) => builder.cubic_to(
                c0.x + dx,
                c0.y + dy,
                c1.x + dx,
                c1.y + dy,
                p.x + dx,
                p.y + dy,
            ),
            Command::Close => builder.close(),
        }
    }
    let path = builder.finish()?;
    let mut mask = tiny_skia::Mask::new(w, h)?;
    let identity = tiny_skia::Transform::identity();
    mask.fill_path(&path, tiny_skia::FillRule::Winding, false, identity);
    if embolden > 0.0 {
        let stroke = tiny_skia::Stroke {
            width: embolden,
            ..Default::default()
        };
        if let Some(stroked) = path.stroke(&stroke, 1.0) {
            mask.fill_path(&stroked, tiny_skia::FillRule::Winding, false, identity);
        }
    }
    Some(mask.data().to_vec())
}

/// Skrifa outlines are y-up from the glyph origin; the atlas is y-down from the baseline.
struct Pen {
    cmds: Vec<Command>,
    shift: f32,
    shear: f32,
}

impl Pen {
    fn point(&self, x: f32, y: f32) -> Point {
        Point::new(x + self.shear * y + self.shift, -y)
    }
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.cmds.push(Command::MoveTo(self.point(x, y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.cmds.push(Command::LineTo(self.point(x, y)));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.cmds
            .push(Command::QuadTo(self.point(cx0, cy0), self.point(x, y)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.cmds.push(Command::CurveTo(
            self.point(cx0, cy0),
            self.point(cx1, cy1),
            self.point(x, y),
        ));
    }
    fn close(&mut self) {
        self.cmds.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("lite-xl-font-test-{}-{}", std::process::id(), name));
        p
    }

    fn err_of(r: Result<FontData, String>) -> String {
        match r {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        }
    }

    #[test]
    fn empty_path() {
        assert_eq!(err_of(FontData::open(c"")), "Parameter 'file' is invalid");
    }

    #[test]
    fn missing_path() {
        assert_eq!(
            err_of(FontData::open(c"/nonexistent/font.ttf")),
            "Couldn't open /nonexistent/font.ttf: No such file or directory"
        );
    }

    #[test]
    fn directory_path() {
        let dir = temp_path("dir");
        std::fs::create_dir_all(&dir).unwrap();
        let c = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(
            err_of(FontData::open(&c)),
            format!("{} is a directory", dir.display())
        );
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn mapping_survives_unlink() {
        let path = temp_path("data").with_extension(std::ffi::OsStr::from_bytes(b"\xff"));
        std::fs::File::create(&path).unwrap().write_all(b"fontdata").unwrap();
        let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        let data = match FontData::open(&c) {
            Ok(d) => d,
            Err(e) => panic!("open failed: {}", e),
        };
        std::fs::remove_file(&path).unwrap();
        assert_eq!(data.bytes(), b"fontdata");
        drop(data);
    }
}
