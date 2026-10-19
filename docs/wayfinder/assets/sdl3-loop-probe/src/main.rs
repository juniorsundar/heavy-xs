//! Throwaway loop, clipboard and dialog probe. Not an editor or a selected platform stack.
mod services;
use services::{ClipAction, ClipboardWorker, DialogApi, DialogKind, Event};
use skrifa::instance::{LocationRef, Size as FontSize};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{CursorIcon, Icon, ResizeDirection, Window, WindowId};
const BUNDLED_FONT: &[u8] = include_bytes!("../../../../../data/fonts/FiraSans-Regular.ttf");
const MAX_TEXT_BYTES: usize = 512;
#[derive(Clone, Copy, PartialEq)]
#[rustfmt::skip]
enum Backend { Wayland, X11 }
#[rustfmt::skip]
impl Backend {
    fn name(self) -> &'static str { match self { Backend::Wayland => "wayland", Backend::X11 => "x11" } }
}
#[rustfmt::skip]
struct Args { backend: Backend, borderless: bool, smoke: Option<Duration>, self_check: bool }
#[rustfmt::skip]
fn parse_args() -> Result<Args, String> {
    let (mut backend, mut borderless, mut smoke, mut self_check) = (None, false, None, false);
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--backend" => backend = Some(match it.next().as_deref() {
                Some("wayland") => Backend::Wayland,
                Some("x11") => Backend::X11,
                _ => return Err("--backend wayland|x11 is required".into()),
            }),
            "--borderless" => borderless = true,
            "--self-check" => self_check = true,
            "--smoke-seconds" => {
                let value = it.next().ok_or("--smoke-seconds needs a positive integer")?;
                let secs: u64 = value.parse().map_err(|_| format!("invalid --smoke-seconds {value:?}"))?;
                if secs == 0 { return Err("--smoke-seconds must be greater than zero".into()); }
                smoke = Some(Duration::from_secs(secs));
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    if self_check && smoke.is_none() { smoke = Some(Duration::from_secs(3)); }
    Ok(Args { backend: backend.ok_or("--backend wayland|x11 is required")?, borderless, smoke, self_check })
}
#[rustfmt::skip]
fn main() {
    let code = match run() { Ok(code) => code, Err(error) => { eprintln!("probe: error: {error}"); 1 } };
    std::process::exit(code);
}
#[rustfmt::skip]
fn run() -> Result<i32, String> {
    let args = parse_args()?;
    services::init_diagnostics();
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unset".into());
    println!("probe: requested/forced backend={} borderless={} smoke={:?}", args.backend.name(), args.borderless, args.smoke);
    println!("probe: session XDG_SESSION_TYPE={session} WAYLAND_DISPLAY={} DISPLAY={}",
        std::env::var_os("WAYLAND_DISPLAY").is_some(), std::env::var_os("DISPLAY").is_some());
    if args.backend == Backend::X11 && session == "wayland" {
        println!("probe: backend label: X11 (XWayland expected; not native X11 evidence)");
    }
    let mut builder = EventLoop::<Event>::with_user_event();
    match args.backend { Backend::Wayland => { builder.with_wayland(); } Backend::X11 => { builder.with_x11(); } }
    let mut event_loop = builder.build().map_err(|error| format!(
        "could not create forced {} event loop (backend unavailable or no display): {error}", args.backend.name()))?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new(args, proxy.clone());
    let mut started_worker = false;
    let (mut pumps, mut waited_wakes) = (0_u64, 0_u64);
    loop {
        pumps += 1;
        if let PumpStatus::Exit(code) = event_loop.pump_app_events(Some(Duration::ZERO), &mut app) {
            app.exit_code = app.exit_code.max(code); break;
        }
        if !started_worker && app.window.is_some() {
            services::wake(proxy.clone(), false); started_worker = true;
        }
        let before = app.received;
        let start = Instant::now();
        let status = event_loop.pump_app_events(Some(Duration::from_secs(5)), &mut app);
        let elapsed = start.elapsed();
        if app.received > before {
            println!("probe: wait interrupted by proxy elapsed_ms={:.1} timeout_ms=5000", elapsed.as_secs_f64() * 1000.0);
            if elapsed >= Duration::from_millis(20) && elapsed < Duration::from_secs(2) { waited_wakes += 1; }
        }
        if let PumpStatus::Exit(code) = status { app.exit_code = app.exit_code.max(code); break; }
    }
    println!("probe: loop summary pumps={pumps} waited_wakes={waited_wakes} worker={} simulated_result={} frames={}", app.worker_wakes, app.simulated_wakes, app.frames);
    if app.args.self_check {
        if app.worker_wakes == 0 || app.simulated_wakes == 0 || waited_wakes < 2 || app.frames < 2 || pumps > 100 {
            return Err("self-check failed: missing blocked-pump wake, redraw, or bounded idle behaviour".into());
        }
        println!("probe: SELF-CHECK PASS (no clipboard or real dialog calls)");
    }
    // Drop the clipboard sender before returning; no helper process preserves ownership after exit.
    let code = app.exit_code;
    drop(app);
    Ok(code)
}
#[derive(Clone, Copy, Debug, PartialEq)]
#[rustfmt::skip]
enum Zone { Client, Drag, Resize(ResizeDirection) }
#[rustfmt::skip]
fn classify_zone(x: f64, y: f64, width: u32, height: u32, border: f64, top: f64) -> Zone {
    let (x, y, w, h) = (x, y, width as f64, height as f64);
    let (west, east, north, south) = (x < border, x >= w - border, y < border, y >= h - border);
    match (north, south, west, east) {
        (true, _, true, _) => Zone::Resize(ResizeDirection::NorthWest),
        (true, _, _, true) => Zone::Resize(ResizeDirection::NorthEast),
        (_, true, true, _) => Zone::Resize(ResizeDirection::SouthWest),
        (_, true, _, true) => Zone::Resize(ResizeDirection::SouthEast),
        (true, _, _, _) => Zone::Resize(ResizeDirection::North),
        (_, true, _, _) => Zone::Resize(ResizeDirection::South),
        (_, _, true, _) => Zone::Resize(ResizeDirection::West),
        (_, _, _, true) => Zone::Resize(ResizeDirection::East),
        _ if y < top => Zone::Drag,
        _ => Zone::Client,
    }
}
#[rustfmt::skip]
fn buffer_dims(size: PhysicalSize<u32>) -> Option<(u32, u32)> { (size.width > 0 && size.height > 0).then_some((size.width, size.height)) }
#[rustfmt::skip]
fn logical_position(point: PhysicalPosition<f64>, scale: f64) -> (f64, f64) { let s = if scale > 0.0 { scale } else { 1.0 }; (point.x / s, point.y / s) }
#[rustfmt::skip]
fn clamp_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) { index -= 1; }
    index
}
#[rustfmt::skip]
fn previous_boundary(text: &str, index: usize) -> usize {
    let mut index = clamp_boundary(text, index);
    if index == 0 { return 0; }
    index -= 1;
    while !text.is_char_boundary(index) { index -= 1; }
    index
}
#[rustfmt::skip]
fn normalize_range(text: &str, range: Option<(usize, usize)>) -> Option<(usize, usize)> {
    range.map(|(a, b)| { let (a, b) = (clamp_boundary(text, a), clamp_boundary(text, b)); (a.min(b), a.max(b)) })
}
#[rustfmt::skip]
fn insert_at(text: &mut String, caret: &mut usize, value: &str) {
    let index = clamp_boundary(text, *caret);
    if text.len() + value.len() <= MAX_TEXT_BYTES { text.insert_str(index, value); *caret = index + value.len(); }
}
#[rustfmt::skip]
fn delete_previous(text: &mut String, caret: &mut usize) -> bool {
    let end = clamp_boundary(text, *caret);
    let start = previous_boundary(text, end);
    *caret = end;
    if start == end { return false; }
    text.replace_range(start..end, "");
    *caret = start;
    true
}
#[derive(Clone)]
#[rustfmt::skip]
struct TextState { committed: String, preedit: String, preedit_cursor: Option<(usize, usize)>, caret: usize }
#[rustfmt::skip]
impl TextState {
    fn new() -> Self { Self { committed: String::new(), preedit: String::new(), preedit_cursor: None, caret: 0 } }
    fn insert(&mut self, value: &str) { insert_at(&mut self.committed, &mut self.caret, value); }
    fn clear_preedit(&mut self) { self.preedit.clear(); self.preedit_cursor = None; }
    fn set_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) -> bool {
        let cursor = normalize_range(&text, cursor);
        if self.preedit == text && self.preedit_cursor == cursor { return false; }
        self.preedit = text;
        self.preedit_cursor = cursor;
        true
    }
}
#[rustfmt::skip]
struct Face { data: Vec<u8>, px: f32 }
#[rustfmt::skip]
struct Bitmap { width: u32, height: u32, left: i32, top: i32, data: Vec<u8>, advance: f32 }
#[rustfmt::skip]
impl Face {
    fn new(data: &[u8], px: f32) -> Self { Self { data: data.to_vec(), px } }
    fn font(&self) -> Option<FontRef<'_>> { FontRef::from_index(&self.data, 0).ok() }
    fn glyph(&self, ch: char) -> Option<GlyphId> {
        let glyph = self.font()?.charmap().map(ch as u32)?;
        (glyph.to_u32() != 0).then_some(glyph)
    }
    fn advance(&self, ch: char) -> Option<f32> {
        let glyph = self.glyph(ch)?;
        self.font()?.glyph_metrics(FontSize::new(self.px), LocationRef::default()).advance_width(glyph)
    }
    fn rasterize(&self, ch: char) -> Option<Bitmap> {
        let font = self.font()?;
        let glyph = self.glyph(ch)?;
        let advance = self.advance(ch)?;
        let empty = Bitmap { width: 0, height: 0, left: 0, top: 0, data: Vec::new(), advance };
        let Some(outline) = font.outline_glyphs().get(glyph) else { return Some(empty) };
        let mut pen = Pen { commands: Vec::new() };
        outline.draw(DrawSettings::unhinted(FontSize::new(self.px), LocationRef::default()), &mut pen).ok()?;
        if pen.commands.is_empty() { return Some(empty); }
        let b = zeno::bounds(&pen.commands, zeno::Fill::NonZero, None);
        let (min_x, min_y) = ((b.min.x - 1.0).floor(), (b.min.y - 1.0).floor());
        let (max_x, max_y) = ((b.max.x + 1.0).ceil(), (b.max.y + 1.0).ceil());
        if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite()) { return None; }
        let (width, height) = ((max_x - min_x) as u32, (max_y - min_y) as u32);
        if width == 0 || height == 0 || width > 4096 || height > 4096 { return Some(empty); }
        let mut data = vec![0u8; (width * height) as usize];
        zeno::Mask::new(&pen.commands).format(zeno::Format::Alpha).size(width, height)
            .offset(zeno::Vector::new(-min_x, -min_y)).render_into(&mut data, None);
        Some(Bitmap { width, height, left: min_x as i32, top: (-min_y).round() as i32, data, advance })
    }
}
#[rustfmt::skip]
struct Pen { commands: Vec<zeno::Command> }
#[rustfmt::skip]
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) { self.commands.push(zeno::Command::MoveTo(zeno::Point::new(x, -y))); }
    fn line_to(&mut self, x: f32, y: f32) { self.commands.push(zeno::Command::LineTo(zeno::Point::new(x, -y))); }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.commands.push(zeno::Command::QuadTo(zeno::Point::new(cx, -cy), zeno::Point::new(x, -y)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.commands.push(zeno::Command::CurveTo(
            zeno::Point::new(cx0, -cy0), zeno::Point::new(cx1, -cy1), zeno::Point::new(x, -y)));
    }
    fn close(&mut self) { self.commands.push(zeno::Command::Close); }
}
#[rustfmt::skip]
struct FontSet { px: f32, faces: Vec<Face> }
#[rustfmt::skip]
impl FontSet {
    fn new(primary: &[u8], fallback: Option<&[u8]>, px: f32) -> Self {
        let mut faces = vec![Face::new(primary, px)];
        if let Some(data) = fallback { faces.push(Face::new(data, px)); }
        Self { px, faces }
    }
    fn face_for(&self, ch: char) -> Option<&Face> { self.faces.iter().find(|face| face.glyph(ch).is_some()) }
    fn width(&self, text: &str) -> f32 {
        text.chars().map(|ch| {
            if ch == '\n' || ch == '\r' { return self.px * 0.5; }
            self.face_for(ch).and_then(|f| f.advance(ch)).unwrap_or(self.px * 0.75)
        }).sum()
    }
    fn missing(&self, text: &str) -> usize {
        text.chars().filter(|ch| *ch != '\n' && *ch != '\r' && self.face_for(*ch).is_none()).count()
    }
}
#[rustfmt::skip]
fn fill(buf: &mut [u32], stride: u32, height: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let (x0, y0) = (x.max(0) as u32, y.max(0) as u32);
    let (x1, y1) = (x.saturating_add(w).min(stride as i32).max(0) as u32,
                   y.saturating_add(h).min(height as i32).max(0) as u32);
    if x0 >= x1 || y0 >= y1 { return; }
    for row in y0..y1 { buf[(row * stride + x0) as usize..(row * stride + x1) as usize].fill(color); }
}
#[rustfmt::skip]
fn stroke(buf: &mut [u32], stride: u32, height: u32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    fill(buf, stride, height, x, y, w, 1, color); fill(buf, stride, height, x, y + h - 1, w, 1, color);
    fill(buf, stride, height, x, y, 1, h, color); fill(buf, stride, height, x + w - 1, y, 1, h, color);
}
fn opaque_xrgb(rgb: u32) -> u32 {
    rgb | 0xff000000
}
#[rustfmt::skip]
fn blend(dst: u32, src: u32, alpha: u32) -> u32 {
    let inv = 255 - alpha;
    let ch = |shift: u32| (((src >> shift) & 0xff) * alpha + ((dst >> shift) & 0xff) * inv) / 255;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}
#[rustfmt::skip]
fn draw_text(set: &FontSet, buf: &mut [u32], stride: u32, height: u32, text: &str,
             mut x: f32, baseline: f32, color: u32) -> f32 {
    for ch in text.chars() {
        if ch == '\n' || ch == '\r' { x += set.px * 0.5; continue; }
        if let Some(face) = set.face_for(ch) {
            if let Some(bitmap) = face.rasterize(ch) {
                let (ox, oy) = (x.round() as i32 + bitmap.left, baseline.round() as i32 - bitmap.top);
                for row in 0..bitmap.height as i32 {
                    for col in 0..bitmap.width as i32 {
                        let (px, py) = (ox + col, oy + row);
                        if px < 0 || py < 0 || px >= stride as i32 || py >= height as i32 { continue; }
                        let alpha = bitmap.data[(row * bitmap.width as i32 + col) as usize] as u32;
                        if alpha > 0 { let i = (py as u32 * stride + px as u32) as usize; buf[i] = blend(buf[i], color, alpha); }
                    }
                }
                x += bitmap.advance;
                continue;
            }
        }
        // Visible tofu: the status line also reports the missing count.
        let size = (set.px * 0.7).round().max(4.0) as i32;
        stroke(buf, stride, height, x.round() as i32, (baseline - size as f32).round() as i32, size, size, color);
        x += set.px * 0.75;
    }
    x
}
#[rustfmt::skip]
struct Canvas<'a> { buf: &'a mut [u32], w: u32, h: u32 }
#[rustfmt::skip]
impl Canvas<'_> {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) { fill(self.buf, self.w, self.h, x, y, w, h, color); }
    fn stroke(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) { stroke(self.buf, self.w, self.h, x, y, w, h, color); }
    fn text(&mut self, set: &FontSet, text: &str, x: f32, baseline: f32, color: u32) -> f32 {
        draw_text(set, self.buf, self.w, self.h, text, x, baseline, color)
    }
}
#[rustfmt::skip]
fn make_icon() -> Option<Icon> {
    const N: u32 = 32;
    let mut rgba = vec![0u8; (N * N * 4) as usize];
    for y in 0..N { for x in 0..N {
        let i = ((y * N + x) * 4) as usize;
        let border = x < 3 || y < 3 || x >= N - 3 || y >= N - 3;
        let color = if border { [40, 220, 180] } else if ((x + y) / 4) % 2 == 0 { [30, 30, 44] } else { [70, 100, 160] };
        rgba[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
    }}
    Icon::from_rgba(rgba, N, N).ok()
}
#[rustfmt::skip]
fn file_name(path: &PathBuf) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "<unnamed>".into())
}
#[rustfmt::skip]
struct App {
    args: Args, deadline: Option<Instant>, exit_code: i32,
    window: Option<Arc<Window>>, _context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>, text: TextState,
    fonts: Option<FontSet>, primary: Vec<u8>, fallback: Option<Vec<u8>>, font_scale: f32,
    presented: bool, modifiers: ModifiersState, ime_enabled: bool,
    pointer: PhysicalPosition<f64>, last_cursor: Option<CursorIcon>,
    cursor_cycle: usize, cursor_override: bool,
    last_ime_area: Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)>,
    proxy: EventLoopProxy<Event>, clipboard: ClipboardWorker,
    dialog_api: DialogApi, dialog_pending: bool, service_status: String,
    received: u64, worker_wakes: u64, simulated_wakes: u64, frames: u64,
}
#[rustfmt::skip]
impl App {
    fn new(args: Args, proxy: EventLoopProxy<Event>) -> Self {
        let fallback = std::env::var_os("PROBE_FONT").and_then(|path| std::fs::read(path).ok()).filter(|data| {
            let ok = FontRef::from_index(data, 0).is_ok();
            println!("probe: optional fallback font loaded={ok} (path hidden)");
            ok
        });
        Self {
            deadline: args.smoke.map(|duration| Instant::now() + duration), args, exit_code: 0,
            window: None, _context: None, surface: None, text: TextState::new(), fonts: None,
            primary: BUNDLED_FONT.to_vec(), fallback, font_scale: 0.0, presented: false,
            modifiers: ModifiersState::empty(), ime_enabled: false,
            pointer: PhysicalPosition::new(0.0, 0.0), last_cursor: None, cursor_cycle: 0, cursor_override: false,
            last_ime_area: None,
            clipboard: ClipboardWorker::new(proxy.clone()), proxy,
            dialog_api: DialogApi::Rfd, dialog_pending: false, service_status: "No clipboard or portal access yet".into(),
            received: 0, worker_wakes: 0, simulated_wakes: 0, frames: 0,
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl Into<String>) {
        eprintln!("probe: error: {}", message.into()); self.exit_code = 1; event_loop.exit();
    }
    fn request_redraw(&self) { if let Some(window) = &self.window { window.request_redraw(); } }
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attributes = Window::default_attributes().with_title("sdl3 LOOP / CLIPBOARD / DIALOG probe (throwaway)")
            .with_inner_size(PhysicalSize::new(1000, 700)).with_decorations(!self.args.borderless);
        let window = Arc::new(event_loop.create_window(attributes).map_err(|e| format!("window creation failed: {e}"))?);
        let (scale, size) = (window.scale_factor(), window.inner_size());
        println!("probe: window created scale={scale:.3} physical={}x{} logical={:.1}x{:.1}",
            size.width, size.height, size.width as f64 / scale, size.height as f64 / scale);
        window.set_ime_allowed(true);
        println!("probe: requested IME enabled");
        window.set_window_icon(make_icon());
        println!("probe: window icon requested (generated RGBA)");
        let context = Context::new(window.clone()).map_err(|e| format!("softbuffer context creation failed: {e}"))?;
        let surface = Surface::new(&context, window.clone()).map_err(|e| format!("softbuffer surface creation failed: {e}"))?;
        self._context = Some(context); self.surface = Some(surface); self.window = Some(window);
        self.request_redraw();
        Ok(())
    }
    fn ensure_fonts(&mut self, scale: f32) {
        if self.fonts.is_none() || (self.font_scale - scale).abs() > f32::EPSILON {
            self.fonts = Some(FontSet::new(&self.primary, self.fallback.as_deref(), 18.0 * scale));
            self.font_scale = scale;
        }
    }
    fn zone(&self, width: u32, height: u32, scale: f64) -> Zone {
        classify_zone(self.pointer.x, self.pointer.y, width, height, 8.0 * scale, 34.0 * scale)
    }
    fn set_cursor(&mut self, icon: CursorIcon) {
        if self.last_cursor != Some(icon) {
            if let Some(window) = &self.window { window.set_cursor(icon); }
            self.last_cursor = Some(icon);
            println!("probe: cursor icon {icon:?}");
        }
    }
    fn redraw(&mut self) -> Result<(), String> {
        let Some(window) = self.window.clone() else { return Ok(()) };
        let size = window.inner_size();
        let Some((w, h)) = buffer_dims(size) else {
            println!("probe: skip redraw: zero inner size {}x{}", size.width, size.height); return Ok(());
        };
        let scale = window.scale_factor() as f32;
        self.ensure_fonts(scale);
        let Some(surface) = self.surface.as_mut() else { return Ok(()) };
        let (nw, nh) = (NonZeroU32::new(w).ok_or("zero buffer width")?, NonZeroU32::new(h).ok_or("zero buffer height")?);
        surface.resize(nw, nh).map_err(|e| format!("softbuffer resize failed: {e}"))?;
        let mut buffer = surface.buffer_mut().map_err(|e| format!("softbuffer buffer_mut failed: {e}"))?;
        let Some(fonts) = self.fonts.as_ref() else { return Err("fonts not initialized".into()) };
        let (wi, hi) = (w as i32, h as i32);
        let (committed, preedit, range) = (&self.text.committed, &self.text.preedit, self.text.preedit_cursor);
        let missing = fonts.missing(committed) + fonts.missing(preedit);
        let caret_area;
        {
            let mut c = Canvas { buf: &mut buffer, w, h };
            c.rect(0, 0, wi, hi, 0x20202a);
            let (cell, step) = (((16.0 * scale) as i32).max(8), ((32.0 * scale) as i32).max(16));
            for y in (0..hi).step_by(cell as usize) { for x in (0..wi).step_by(cell as usize) {
                if ((x / cell) + (y / cell)) % 2 == 1 { c.rect(x, y, cell, cell, 0x252531); }
            }}
            for y in (0..hi).step_by(step as usize) { c.rect(0, y, wi, 1, 0x343446); }
            for x in (0..wi).step_by(step as usize) { c.rect(x, 0, 1, hi, 0x343446); }
            let (edge, top) = (((6.0 * scale) as i32).max(4), ((34.0 * scale) as i32).max(20));
            c.rect(0, edge, wi, top - edge, 0x244c3a);
            c.rect(edge, 0, wi - 2 * edge, edge, 0x2c4a70); c.rect(edge, hi - edge, wi - 2 * edge, edge, 0x2c4a70);
            c.rect(0, edge, edge, hi - 2 * edge, 0x2c4a70); c.rect(wi - edge, edge, edge, hi - 2 * edge, 0x2c4a70);
            for (x, y) in [(0, 0), (wi - edge, 0), (0, hi - edge), (wi - edge, hi - edge)] { c.rect(x, y, edge, edge, 0x6a3c78); }
            let margin = (24.0 * scale) as i32;
            let fh = (48.0 * scale).max(24.0) as i32;
            let field = (margin, hi / 2 - fh / 2, wi - 2 * margin, fh);
            c.rect(field.0, field.1, field.2, field.3, 0x15151c);
            c.stroke(field.0, field.1, field.2, field.3, 0x7a8aa0);
            let pad = (12.0 * scale) as i32;
            let baseline = field.1 as f32 + field.3 as f32 * 0.5 + fonts.px * 0.35;
            let text_x = (field.0 + pad) as f32;
            let end = c.text(fonts, committed, text_x, baseline, 0xf0f0f0);
            let (ul0, ul1, caret, visible) = if preedit.is_empty() { (end, end, end, true) } else {
                let drawn_end = c.text(fonts, preedit, end, baseline, 0xffd166);
                match range {
                    Some((a, b)) => {
                        let (a, b) = (clamp_boundary(preedit, a), clamp_boundary(preedit, b));
                        let (x0, x1) = (end + fonts.width(&preedit[..a]), end + fonts.width(&preedit[..b]));
                        (x0, x1, x0, true)
                    }
                    None => (end, drawn_end, end, false),
                }
            };
            if !preedit.is_empty() {
                c.rect(ul0 as i32, (baseline + 4.0 * scale) as i32, (ul1 - ul0).max(1.0) as i32,
                       (2.0 * scale).max(1.0) as i32, 0xffd166);
            }
            let caret_w = (2.0 * scale).max(2.0) as i32;
            let (caret_y, caret_h) = (field.1 + (6.0 * scale) as i32, field.3 - 2 * (6.0 * scale) as i32);
            if visible { c.rect(caret as i32, caret_y, caret_w, caret_h, 0x50e0ff); }
            caret_area = ((caret as i32).clamp(0, wi.saturating_sub(1)), field.1.max(0), caret_w as u32, field.3.max(1) as u32);
            let line = (fonts.px * 1.7) as i32;
            let mut y = top + line;
            let title = format!("backend: {}  scale: {:.2}  physical: {}x{}  logical: {:.0}x{:.0}",
                self.args.backend.name(), scale, w, h, w as f32 / scale, h as f32 / scale);
            c.text(fonts, &title, margin as f32, y as f32, 0xe8e8f0); y += line;
            let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unset".into());
            let mode = if self.args.backend == Backend::X11 && session == "wayland" {
                "forced X11 under Wayland (XWayland expected; not native X11 evidence)"
            } else { "backend forced by --backend" };
            c.text(fonts, mode, margin as f32, y as f32, 0xa8b8d0); y += line;
            let status = format!("IME: {}  preedit bytes: {}  cursor: {:?}  missing glyphs: {missing}",
                self.ime_enabled, preedit.len(), range);
            c.text(fonts, &status, margin as f32, y as f32, 0xa8b8d0); y += line;
            let hint = if self.args.borderless {
                "borderless: drag top zone, resize edges/corners; click field sets caret to end"
            } else { "F7 cycles cursor until pointer moves; Escape closes unless composing; Enter logged" };
            c.text(fonts, hint, margin as f32, y as f32, 0x88a0b8);
            y = field.1 + field.3 + 2 * line;
            for label in ["F1 wake | F2 COPY marker | F3 read clipboard",
                "F4 PRIMARY marker | F5 read primary | F6 API",
                "F8 open | F9 save | F10 folder | F11 sandbox"] {
                c.text(fonts, label, margin as f32, y as f32, 0xb8d8e8); y += line;
            }
            let service = format!("API: {:?} | {}", self.dialog_api, self.service_status);
            c.text(fonts, &service, margin as f32, y as f32, 0xffd166);
        }
        let ime_area = (PhysicalPosition::new(caret_area.0, caret_area.1),
            PhysicalSize::new(caret_area.2, caret_area.3), window.scale_factor());
        if self.ime_enabled && self.last_ime_area != Some(ime_area) {
            // Updating the area can itself produce an empty preedit notification.
            window.set_ime_cursor_area(ime_area.0, ime_area.1);
            self.last_ime_area = Some(ime_area);
        }
        if self.args.backend == Backend::Wayland {
            // shortcut: this Wayland path needs opaque XRGB padding; investigate below softbuffer before production use.
            for pixel in buffer.iter_mut() { *pixel = opaque_xrgb(*pixel); }
        }
        window.pre_present_notify();
        buffer.present().map_err(|e| format!("softbuffer present failed: {e}"))?;
        self.presented = true;
        self.frames += 1;
        println!("probe: presented frame {w}x{h} at scale {scale:.3}");
        Ok(())
    }
}
#[rustfmt::skip]
impl ApplicationHandler<Event> for App {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Event) {
        self.received += 1;
        match event {
            Event::Wake { sent, simulated } => {
                if simulated { self.simulated_wakes += 1; } else { self.worker_wakes += 1; }
                self.service_status = format!("{} proxy wake, latency {:.2} ms; dialog_pending={}",
                    if simulated { "SIMULATED result" } else { "worker" }, sent.elapsed().as_secs_f64() * 1000.0, self.dialog_pending);
                if self.args.self_check && !simulated { services::wake(self.proxy.clone(), true); }
            }
            Event::Result { sent, summary, dialog } => {
                if dialog { self.dialog_pending = false; }
                self.service_status = format!("{summary}; elapsed {:.1} ms", sent.elapsed().as_secs_f64() * 1000.0);
            }
        }
        println!("probe: service {}", self.service_status);
        self.request_redraw();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        if let Err(error) = self.create_window(event_loop) { self.fail(event_loop, error); return; }
        match self.deadline {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else { return };
        if window.id() != window_id { return; }
        match event {
            WindowEvent::CloseRequested => { println!("probe: close requested"); event_loop.exit(); }
            WindowEvent::Destroyed => { println!("probe: window destroyed"); self.window = None; self.surface = None; self._context = None; }
            WindowEvent::Resized(size) => {
                let scale = window.scale_factor();
                println!("probe: resized physical={}x{} logical={:.1}x{:.1} scale={scale:.3}",
                    size.width, size.height, size.width as f64 / scale, size.height as f64 / scale);
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = window.inner_size();
                println!("probe: scale factor changed to {scale_factor:.3} physical={}x{} logical={:.1}x{:.1}",
                    size.width, size.height, size.width as f64 / scale_factor, size.height as f64 / scale_factor);
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => if let Err(error) = self.redraw() { self.fail(event_loop, error); },
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                println!("probe: modifiers {:?}", self.modifiers);
            }
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                println!("probe: key synthetic={is_synthetic} {event:?}");
                if !event.state.is_pressed() { return; }
                let key = event.logical_key.as_ref();
                if !event.repeat {
                    let action = match key {
                        Key::Named(NamedKey::F2) => Some(ClipAction::SetClipboard),
                        Key::Named(NamedKey::F3) => Some(ClipAction::GetClipboard),
                        Key::Named(NamedKey::F4) => Some(ClipAction::SetPrimary),
                        Key::Named(NamedKey::F5) => Some(ClipAction::GetPrimary),
                        _ => None,
                    };
                    if let Some(action) = action {
                        if self.args.self_check { return; }
                        self.service_status = match self.clipboard.send(action) { Ok(()) => format!("queued {action:?}"), Err(error) => error };
                        self.request_redraw(); return;
                    }
                    match key {
                        Key::Named(NamedKey::F1) => { services::wake(self.proxy.clone(), false); return; }
                        Key::Named(NamedKey::F6) => {
                            self.dialog_api = if self.dialog_api == DialogApi::Rfd { DialogApi::Ashpd } else { DialogApi::Rfd };
                            self.service_status = "API changed; no dialog opened".into(); self.request_redraw(); return;
                        }
                        Key::Named(NamedKey::F8 | NamedKey::F9 | NamedKey::F10) => {
                            if self.args.self_check || self.dialog_pending { return; }
                            let kind = match key { Key::Named(NamedKey::F8) => DialogKind::Open,
                                Key::Named(NamedKey::F9) => DialogKind::Save, _ => DialogKind::Directory };
                            self.dialog_pending = true;
                            self.service_status = format!("{:?} {kind:?} pending", self.dialog_api);
                            println!("probe: service {}", self.service_status);
                            services::dialog(self.proxy.clone(), window.clone(), self.dialog_api, kind);
                            services::wake(self.proxy.clone(), false);
                            self.request_redraw(); return;
                        }
                        Key::Named(NamedKey::F11) => {
                            if self.args.self_check { return; }
                            self.service_status = format!("ashpd sandbox heuristic={}; actual sandbox review pending", ashpd::is_sandboxed());
                            println!("probe: service {}", self.service_status); self.request_redraw(); return;
                        }
                        _ => {}
                    }
                }
                match key {
                    Key::Named(NamedKey::Escape) => {
                        if self.text.preedit.is_empty() { println!("probe: Escape with no composition; closing"); event_loop.exit(); }
                        else { println!("probe: Escape during composition; cancellation belongs to the IME"); }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if self.text.preedit.is_empty() {
                            let changed = delete_previous(&mut self.text.committed, &mut self.text.caret);
                            println!("probe: Backspace changed={changed} caret={}", self.text.caret);
                            self.request_redraw();
                        }
                    }
                    Key::Named(NamedKey::Enter) => println!("probe: Enter (single-line field; no newline inserted)"),
                    Key::Named(NamedKey::F7) => {
                        const CURSORS: [CursorIcon; 6] = [CursorIcon::Default, CursorIcon::Text, CursorIcon::Crosshair,
                            CursorIcon::Grab, CursorIcon::Pointer, CursorIcon::Help];
                        self.cursor_cycle = (self.cursor_cycle + 1) % CURSORS.len();
                        self.cursor_override = true;
                        self.set_cursor(CURSORS[self.cursor_cycle]);
                        println!("probe: F7 cursor cycle index={}", self.cursor_cycle);
                    }
                    _ => {
                        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
                        // IME availability does not imply ordinary typing arrives through Commit.
                        if !shortcut && self.text.preedit.is_empty() {
                            if let Some(text) = event.text.as_deref() {
                                self.text.insert(text);
                                println!("probe: inserted {} bytes caret={} total={}", text.len(), self.text.caret, self.text.committed.len());
                                self.request_redraw();
                            }
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => { self.ime_enabled = true; self.last_ime_area = None; println!("probe: IME Enabled"); self.request_redraw(); }
                Ime::Preedit(text, cursor) => {
                    println!("probe: IME Preedit bytes={} cursor={cursor:?}", text.len());
                    if self.text.set_preedit(text, cursor) { self.request_redraw(); }
                }
                Ime::Commit(text) => { println!("probe: IME Commit bytes={}", text.len()); self.text.insert(&text); self.text.clear_preedit(); self.request_redraw(); }
                Ime::Disabled => { self.ime_enabled = false; self.last_ime_area = None; println!("probe: IME Disabled"); self.text.clear_preedit(); self.request_redraw(); }
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = position;
                let (size, scale) = (window.inner_size(), window.scale_factor());
                if !self.cursor_override {
                    self.set_cursor(match self.zone(size.width, size.height, scale) {
                        Zone::Drag => CursorIcon::Move, Zone::Resize(direction) => direction.into(), Zone::Client => CursorIcon::Text,
                    });
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left || !state.is_pressed() { return; }
                let (size, scale) = (window.inner_size(), window.scale_factor());
                let (lx, ly) = logical_position(self.pointer, scale);
                let zone = self.zone(size.width, size.height, scale);
                println!("probe: left press physical=({:.1},{:.1}) logical=({lx:.1},{ly:.1}) zone={zone:?}", self.pointer.x, self.pointer.y);
                self.cursor_override = false;
                match zone {
                    Zone::Drag => {
                        println!("probe: drag_window requested");
                        if let Err(e) = window.drag_window() { println!("probe: drag_window failed: {e}"); }
                    }
                    Zone::Resize(direction) => {
                        println!("probe: drag_resize_window requested {direction:?}");
                        if let Err(e) = window.drag_resize_window(direction) { println!("probe: drag_resize_window failed: {e}"); }
                    }
                    Zone::Client => {
                        let margin = (24.0 * scale) as i32; let fh = (48.0 * scale).max(24.0) as i32;
                        let inside = self.pointer.x >= margin as f64 && self.pointer.x < (size.width as i32 - margin) as f64
                            && self.pointer.y >= (size.height as i32 / 2 - fh / 2) as f64
                            && self.pointer.y < (size.height as i32 / 2 + fh / 2) as f64;
                        if inside { self.text.caret = self.text.committed.len(); println!("probe: text field click; caret set to end of field"); self.request_redraw(); }
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => self.last_cursor = None,
            WindowEvent::Focused(focused) => { println!("probe: focused={focused}"); self.request_redraw(); }
            WindowEvent::DroppedFile(path) => println!("probe: file dropped name={}", file_name(&path)),
            WindowEvent::HoveredFile(path) => println!("probe: file hover name={}", file_name(&path)),
            WindowEvent::HoveredFileCancelled => println!("probe: file hover cancelled"),
            WindowEvent::Touch(touch) => {
                let scale = window.scale_factor(); let (lx, ly) = logical_position(touch.location, scale);
                println!("probe: touch id={} phase={:?} physical=({:.1},{:.1}) logical=({lx:.1},{ly:.1})",
                    touch.id, touch.phase, touch.location.x, touch.location.y);
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                if self.presented { println!("probe: smoke timeout reached; at least one frame was presented"); event_loop.exit(); }
                else { self.fail(event_loop, "smoke timeout before any frame was presented"); }
            }
        }
    }
}
#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    #[test]
    fn corners_and_edges_classify() {
        let (w, h, b, t) = (200, 100, 8.0, 30.0);
        let cases = [
            (1.0, 1.0, ResizeDirection::NorthWest), (199.0, 1.0, ResizeDirection::NorthEast),
            (1.0, 99.0, ResizeDirection::SouthWest), (199.0, 99.0, ResizeDirection::SouthEast),
            (100.0, 1.0, ResizeDirection::North), (100.0, 99.0, ResizeDirection::South),
            (1.0, 50.0, ResizeDirection::West), (199.0, 50.0, ResizeDirection::East),
        ];
        for (x, y, direction) in cases { assert_eq!(classify_zone(x, y, w, h, b, t), Zone::Resize(direction)); }
        assert_eq!(classify_zone(100.0, 20.0, w, h, b, t), Zone::Drag);
        assert_eq!(classify_zone(100.0, 60.0, w, h, b, t), Zone::Client);
    }
    #[test]
    fn unicode_caret_boundaries_are_safe() {
        let text = "aé漢";
        assert_eq!((0..=6).map(|i| clamp_boundary(text, i)).collect::<Vec<_>>(), [0, 1, 1, 3, 3, 3, 6]);
        assert_eq!(clamp_boundary(text, 99), text.len());
        assert_eq!(previous_boundary(text, text.len()), 3);
        assert_eq!(previous_boundary(text, 5), 1);
        assert_eq!(previous_boundary(text, 3), 1);
        assert_eq!(previous_boundary(text, 1), 0);
        assert_eq!(normalize_range(text, Some((4, 2))), Some((1, 3)));
        assert_eq!(normalize_range(text, None), None);
    }
    #[test]
    fn insert_and_delete_keep_caret_on_boundaries() {
        let (mut text, mut caret) = (String::new(), 0);
        insert_at(&mut text, &mut caret, "aé漢");
        assert_eq!((text.as_str(), caret), ("aé漢", 6));
        assert!(delete_previous(&mut text, &mut caret));
        assert_eq!((text.as_str(), caret), ("aé", 3));
        assert!(delete_previous(&mut text, &mut caret));
        assert_eq!((text.as_str(), caret), ("a", 1));
        assert!(delete_previous(&mut text, &mut caret));
        assert!(!delete_previous(&mut text, &mut caret));
        assert_eq!((text.as_str(), caret), ("", 0));
    }
    #[test]
    fn repeated_empty_preedit_does_not_request_another_redraw() {
        let mut text = TextState::new();
        assert!(!text.set_preedit(String::new(), None));
        assert!(text.set_preedit("漢".into(), Some((3, 3))));
        assert!(!text.set_preedit("漢".into(), Some((3, 3))));
        assert!(text.set_preedit(String::new(), None));
        assert!(!text.set_preedit(String::new(), None));
    }
    #[test]
    fn opaque_padding_preserves_every_rgb_channel() {
        for rgb in [0, 0x244c3a, 0x15151c, 0xffffff] {
            let pixel = opaque_xrgb(rgb);
            assert_eq!(pixel >> 24, 0xff);
            assert_eq!(pixel & 0xffffff, rgb);
        }
    }
    #[test]
    fn text_measurement_matches_visible_missing_glyphs() {
        let fonts = FontSet::new(BUNDLED_FONT, None, 18.0);
        let text = "a\u{10ffff}\n";
        let mut buffer = vec![0; 128 * 64];
        let end = draw_text(&fonts, &mut buffer, 128, 64, text, 5.0, 32.0, 0xffffff);
        assert!((end - 5.0 - fonts.width(text)).abs() < 0.001);
        assert_eq!(fonts.missing(text), 1);
        assert!(buffer.iter().any(|&p| p != 0));
        assert!(buffer.iter().all(|&p| p >> 24 == 0));
    }
    #[test]
    fn physical_buffer_size_skips_zero_and_logical_conversion() {
        assert_eq!(buffer_dims(PhysicalSize::new(800, 520)), Some((800, 520)));
        assert_eq!(buffer_dims(PhysicalSize::new(0, 520)), None);
        assert_eq!(buffer_dims(PhysicalSize::new(800, 0)), None);
        assert_eq!(logical_position(PhysicalPosition::new(100.0, 50.0), 1.25), (80.0, 40.0));
        assert_eq!(logical_position(PhysicalPosition::new(100.0, 50.0), 0.0), (100.0, 50.0));
    }
}
