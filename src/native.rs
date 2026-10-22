//! Main-thread winit/softbuffer owner. Lua drives the pump; callbacks only publish owned input.
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use softbuffer::{AlphaMode, Context, Pixel, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::platform::wayland::EventLoopExtWayland;
use winit::window::{CursorIcon, Icon, ResizeDirection, Window, WindowId};

use crate::hit;
use crate::native_input::InputState;
use crate::platform::{self, Input};

#[derive(Debug)]
pub enum Wake {
    Input,
    ReleaseParent(u64),
    #[cfg(test)]
    Panic,
}

// Fields drop in declaration order: presentation resources before their window lease.
struct NativeWindow {
    surface: Surface<Arc<Window>, Arc<Window>>,
    _context: Context<Arc<Window>>,
    window: Arc<Window>,
    shown: bool,
    hit_test: bool,
    minimized: bool,
    maximized: bool,
}

struct Create { title: String, width: f32, height: f32 }
struct Owner {
    event_loop: Option<EventLoop<Wake>>,
    windows: HashMap<u32, NativeWindow>,
    ids: HashMap<WindowId, u32>,
    next_id: u32,
    parents: HashMap<u64, Arc<Window>>,
    next_parent: u64,
    create: Option<Create>,
    created: Option<Result<u32, String>>,
    input: InputState,
    cursor: CursorIcon,
    hit_info: [i32; 3],
    wayland: bool,
}

// Detached chooser threads may still borrow the display when main-thread TLS is destroyed.
impl Drop for Owner {
    fn drop(&mut self) {
        if !self.parents.is_empty() {
            std::mem::forget(self.event_loop.take());
            std::mem::forget(std::mem::take(&mut self.windows));
            std::mem::forget(std::mem::take(&mut self.parents));
        }
    }
}

pub struct Parent {
    window: Option<Arc<Window>>,
    token: u64,
    proxy: winit::event_loop::EventLoopProxy<Wake>,
}

impl Parent {
    pub fn window(&self) -> &Window { self.window.as_deref().unwrap() }
}

impl Drop for Parent {
    fn drop(&mut self) {
        // The main-thread registry keeps the last Arc until this borrow has gone.
        drop(self.window.take());
        let _ = self.proxy.send_event(Wake::ReleaseParent(self.token));
    }
}

pub fn retain_parent(id: u32) -> Result<Parent, String> {
    with_owner(|owner| {
        let window = owner.windows.get(&id).ok_or("dialog parent is no longer registered")?.window.clone();
        let token = owner.next_parent;
        owner.next_parent = token.checked_add(1).ok_or("dialog request tokens exhausted")?;
        let proxy = owner.event_loop.as_ref().unwrap().create_proxy();
        owner.parents.insert(token, window.clone());
        Ok(Parent { window: Some(window), token, proxy })
    }).unwrap_or_else(|| Err("native dialog parent is not available".into()))
}

enum Slot { Empty, Ready(Box<Owner>), Failed(String), Pumping }
thread_local! { static OWNER: RefCell<Slot> = const { RefCell::new(Slot::Empty) }; }

fn ensure() -> Result<(), String> {
    OWNER.with(|cell| {
        let mut slot = cell.borrow_mut();
        match &*slot {
            Slot::Ready(_) => return Ok(()),
            Slot::Failed(error) => return Err(error.clone()),
            Slot::Pumping => return Err("native event loop is already pumping".into()),
            Slot::Empty => {}
        }
        let mut builder = EventLoop::<Wake>::with_user_event();
        #[cfg(test)]
        winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
        let event_loop = match builder.build() {
            Ok(event_loop) => event_loop,
            Err(error) => {
                let error = format!("cannot create native event loop: {error}");
                *slot = Slot::Failed(error.clone());
                return Err(error);
            }
        };
        let wayland = event_loop.is_wayland();
        #[cfg(feature = "test_hooks")]
        eprintln!("Lite XL native backend '{}'", if wayland { "wayland" } else { "x11" });
        platform::publish_proxy(event_loop.create_proxy());
        *slot = Slot::Ready(Box::new(Owner {
            event_loop: Some(event_loop), windows: HashMap::new(), ids: HashMap::new(),
            next_id: 1, parents: HashMap::new(), next_parent: 1,
            create: None, created: None, input: InputState::new(),
            cursor: CursorIcon::Default, hit_info: [0; 3], wayland,
        }));
        Ok(())
    })
}

pub fn initialized() -> bool {
    OWNER.with(|cell| matches!(*cell.borrow(), Slot::Ready(_)))
}

fn with_owner<R>(f: impl FnOnce(&mut Owner) -> R) -> Option<R> {
    OWNER.with(|cell| match &mut *cell.borrow_mut() {
        Slot::Ready(owner) => Some(f(owner)),
        _ => None,
    })
}

pub fn pump(timeout: Option<Duration>) -> Result<(), String> {
    // Take the owner out, releasing the RefCell before any callback can run.
    let mut owner = OWNER.with(|cell| {
        let mut slot = cell.borrow_mut();
        match std::mem::replace(&mut *slot, Slot::Pumping) {
            Slot::Ready(owner) => Ok(owner),
            other => { *slot = other; Err("native event loop is not available".to_string()) }
        }
    })?;
    let mut event_loop = owner.event_loop.take().expect("event loop belongs to owner");
    let status = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        event_loop.pump_app_events(timeout, &mut *owner)
    }));
    // Restore ownership even on unwind: outstanding choosers must not lose their display.
    owner.event_loop = Some(event_loop);
    OWNER.with(|cell| *cell.borrow_mut() = Slot::Ready(owner));
    let status = match status {
        Ok(status) => status,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    match status {
        PumpStatus::Continue => Ok(()),
        PumpStatus::Exit(code) => Err(format!("native event loop exited ({code})")),
    }
}

pub fn create(title: &str, width: f32, height: f32) -> Result<u32, String> {
    ensure()?;
    with_owner(|owner| {
        owner.create = Some(Create { title: title.into(), width, height });
        owner.created = None;
    });
    // Window creation is only legal inside an ActiveEventLoop callback.
    pump(Some(Duration::ZERO))?;
    with_owner(|owner| owner.created.take()).flatten()
        .unwrap_or_else(|| Err("native event loop did not service window creation".into()))
}

pub fn window(id: u32) -> Option<Arc<Window>> {
    with_owner(|owner| owner.windows.get(&id).map(|w| w.window.clone())).flatten()
}

pub fn destroy(id: u32) {
    with_owner(|owner| {
        if let Some(window) = owner.windows.remove(&id) {
            window.window.set_visible(false);
            owner.ids.remove(&window.window.id());
            owner.input.remove(id);
        }
    });
}

pub fn logical_size(id: u32) -> (i32, i32) {
    window(id).map_or((0, 0), |w| {
        let size = w.inner_size().to_logical::<i32>(w.scale_factor());
        (size.width, size.height)
    })
}

pub fn physical_size(id: u32) -> (i32, i32) {
    window(id).map_or((0, 0), |w| {
        let size = w.inner_size(); (size.width as i32, size.height as i32)
    })
}

pub fn scale(id: u32) -> f32 { window(id).map_or(1.0, |w| w.scale_factor() as f32) }
pub fn pointer(id: u32) -> (f32, f32) { with_owner(|o| o.input.pointer(id)).unwrap_or_default() }

pub fn retire_generation() {
    if initialized() {
        if let Err(error) = pump(Some(Duration::ZERO)) { platform::report_error(error); }
    }
    with_owner(|owner| {
        for (&id, window) in &owner.windows {
            owner.input.set_text_input(id, false);
            window.window.set_ime_allowed(false);
        }
    });
}

pub fn set_cursor(cursor: CursorIcon) {
    with_owner(|owner| {
        owner.cursor = cursor;
        for window in owner.windows.values() { window.window.set_cursor(cursor); }
    });
}

pub fn set_hit_test(id: u32, info: Option<[i32; 3]>) {
    with_owner(|owner| {
        if let Some(info) = info { owner.hit_info = info; }
        if let Some(window) = owner.windows.get_mut(&id) { window.hit_test = info.is_some(); }
    });
}

pub fn set_text_input(id: u32, enabled: bool) {
    with_owner(|owner| {
        owner.input.set_text_input(id, enabled);
        if let Some(window) = owner.windows.get(&id) { window.window.set_ime_allowed(enabled); }
    });
}

pub fn set_ime_rect(id: u32, rect: [i32; 4]) {
    if let Some(window) = window(id) {
        window.set_ime_cursor_area(
            PhysicalPosition::new(rect[0], rect[1]),
            PhysicalSize::new(rect[2].max(0) as u32, rect[3].max(0) as u32),
        );
    }
}

pub fn clear_ime(id: u32) {
    with_owner(|owner| {
        let enabled = owner.input.clear_ime(id);
        if let Some(window) = owner.windows.get(&id).filter(|_| enabled) {
            window.window.set_ime_allowed(false);
            window.window.set_ime_allowed(true);
        }
    });
}

pub fn present(id: u32, pixels: &[u32], width: i32, height: i32, show: bool) -> Result<(), String> {
    with_owner(|owner| {
        let native = owner.windows.get_mut(&id).ok_or("window is no longer registered")?;
        if width <= 0 || height <= 0 || native.window.is_minimized() == Some(true) { return Ok(()); }
        let (w, h) = (width as u32, height as u32);
        if pixels.len() != w as usize * h as usize { return Err("invalid pixel store length".into()); }
        let alpha_mode = if owner.wayland { AlphaMode::Premultiplied } else { AlphaMode::Ignored };
        if !native.surface.supports_alpha_mode(alpha_mode) {
            return Err(format!("unsupported presentation alpha mode: {alpha_mode:?}"));
        }
        native.surface.configure(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap(), alpha_mode)
            .map_err(|e| format!("cannot configure presentation: {e}"))?;
        let mut buffer = native.surface.next_buffer().map_err(|e| format!("cannot get presentation buffer: {e}"))?;
        // Buffer age is irrelevant: write every visible pixel, leaving padding untouched.
        for (src, dst) in pixels.chunks_exact(w as usize).zip(buffer.pixel_rows()) {
            for (&pixel, out) in src.iter().zip(dst) {
                *out = Pixel::new_rgba((pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8, 255);
            }
        }
        native.window.pre_present_notify();
        buffer.present().map_err(|e| format!("cannot present window: {e}"))?;
        if show && !native.shown { native.window.set_visible(true); native.shown = true; }
        Ok(())
    }).unwrap_or_else(|| Err("native owner is not available".into()))
}

impl Owner {
    fn create_pending(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(request) = self.create.take() {
            self.created = Some(self.create_window(event_loop, request));
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, request: Create) -> Result<u32, String> {
        let (mut width, mut height) = (request.width, request.height);
        if width < 1.0 || height < 1.0 {
            let monitor = event_loop.primary_monitor().or_else(|| event_loop.available_monitors().next())
                .ok_or("no display mode is available")?;
            let size = monitor.size().to_logical::<f32>(monitor.scale_factor());
            if width < 1.0 { width = size.width * 0.8; }
            if height < 1.0 { height = size.height * 0.8; }
        }
        let icon = Icon::from_rgba(include_bytes!(concat!(env!("OUT_DIR"), "/icon.rgba")).to_vec(), 64, 64)
            .map_err(|e| format!("invalid window icon: {e}"))?;
        let attrs = Window::default_attributes().with_title(request.title)
            .with_inner_size(LogicalSize::new(width as i32, height as i32))
            .with_resizable(true).with_visible(false).with_window_icon(Some(icon));
        // Both extensions write the same Linux name field: choose once for the active backend.
        let attrs = if self.wayland {
            winit::platform::wayland::WindowAttributesExtWayland::with_name(attrs, "com.lite_xl.LiteXL", "lite-xl")
        } else {
            winit::platform::x11::WindowAttributesExtX11::with_name(attrs, "Lite XL", "lite-xl")
        };
        let window = Arc::new(event_loop.create_window(attrs).map_err(|e| format!("cannot create window: {e}"))?);
        window.set_cursor(self.cursor);
        let context = Context::new(window.clone()).map_err(|e| format!("cannot create presentation context: {e}"))?;
        let surface = Surface::new(&context, window.clone()).map_err(|e| format!("cannot create presentation surface: {e}"))?;
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("window id space exhausted")?;
        self.ids.insert(window.id(), id);
        self.windows.insert(id, NativeWindow {
            surface, _context: context, window, shown: false, hit_test: false,
            minimized: false, maximized: false,
        });
        Ok(id)
    }

    fn hit_zone(&self, id: u32, window: &NativeWindow) -> i32 {
        if !window.hit_test || window.window.is_decorated() { return hit::NORMAL; }
        let (x, y) = self.input.pointer(id);
        let size = window.window.inner_size().to_logical::<i32>(window.window.scale_factor());
        hit::classify(x as i32, y as i32, size.width, size.height,
            self.hit_info[0], self.hit_info[1], self.hit_info[2])
    }
}

fn resize_direction(zone: i32) -> Option<ResizeDirection> {
    match zone {
        hit::RESIZE_TOPLEFT => Some(ResizeDirection::NorthWest),
        hit::RESIZE_TOPRIGHT => Some(ResizeDirection::NorthEast),
        hit::RESIZE_BOTTOMRIGHT => Some(ResizeDirection::SouthEast),
        hit::RESIZE_BOTTOM => Some(ResizeDirection::South),
        hit::RESIZE_BOTTOMLEFT => Some(ResizeDirection::SouthWest),
        hit::RESIZE_LEFT => Some(ResizeDirection::West),
        _ => None,
    }
}

impl ApplicationHandler<Wake> for Owner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) { self.create_pending(event_loop); }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) { self.create_pending(event_loop); }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Wake) {
        match event {
            Wake::Input => {},
            Wake::ReleaseParent(token) => { self.parents.remove(&token); },
            #[cfg(test)]
            Wake::Panic => panic!("test native callback unwind"),
        }
    }
    fn window_event(&mut self, _event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(&id) = self.ids.get(&window_id) else { return; };
        let Some(window) = self.windows.get(&id) else { return; };
        if matches!(event, WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }) {
            // Winit only accepts compositor move/resize requests inside this live button callback.
            let zone = self.hit_zone(id, window);
            if zone == hit::DRAGGABLE {
                let _ = window.window.drag_window();
                return;
            }
            if let Some(direction) = resize_direction(zone) {
                let _ = window.window.drag_resize_window(direction);
                return;
            }
        }
        self.input.event(id, &window.window, &event);
        match event {
            WindowEvent::CloseRequested => platform::request_quit(),
            WindowEvent::Resized(size) => {
                let logical = size.to_logical::<i32>(window.window.scale_factor());
                platform::push(Input::Resized { window: id, w: logical.width, h: logical.height });
                let minimized = window.window.is_minimized() == Some(true);
                let maximized = window.window.is_maximized();
                let window = self.windows.get_mut(&id).unwrap();
                if minimized != window.minimized || maximized != window.maximized {
                    platform::push(if minimized { Input::Minimized } else if maximized { Input::Maximized } else { Input::Restored });
                    window.minimized = minimized; window.maximized = maximized;
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => platform::push(Input::Rescaled { window: id }),
            WindowEvent::RedrawRequested => platform::push(Input::Exposed),
            WindowEvent::CursorMoved { .. } => {
                let zone = self.hit_zone(id, window);
                let cursor = match resize_direction(zone) {
                    Some(ResizeDirection::NorthWest | ResizeDirection::SouthEast) => CursorIcon::NwseResize,
                    Some(ResizeDirection::NorthEast | ResizeDirection::SouthWest) => CursorIcon::NeswResize,
                    Some(ResizeDirection::West) => CursorIcon::EwResize,
                    Some(ResizeDirection::South) => CursorIcon::NsResize,
                    _ => self.cursor,
                };
                window.window.set_cursor(cursor);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "fault subject; dialog-check.sh supplies an isolated private X server"]
    fn pump_unwind_keeps_parent_alive() {
        use winit::raw_window_handle::HasWindowHandle;
        assert_eq!(std::env::var("LXL_PRIVATE_DISPLAY_TEST").as_deref(), Ok("1"), "run via dialog-check.sh");
        let id = create("unwind-parent", 120.0, 80.0).unwrap();
        let parent = retain_parent(id).unwrap();
        destroy(id);
        parent.proxy.send_event(Wake::Panic).unwrap();
        assert!(std::panic::catch_unwind(|| pump(Some(Duration::ZERO))).is_err());
        assert!(with_owner(|o| o.event_loop.is_some() && o.parents.contains_key(&parent.token)).unwrap());
        assert!(parent.window().window_handle().is_ok());
        drop(parent); // the main owner retains resources until process exit, without another pump
    }

    #[test]
    fn resize_zones_keep_editor_quirks() {
        assert_eq!(resize_direction(hit::NORMAL), None);
        assert_eq!(resize_direction(hit::DRAGGABLE), None);
        assert_eq!(resize_direction(hit::RESIZE_BOTTOM), Some(ResizeDirection::South));
        assert_eq!(resize_direction(hit::RESIZE_TOPLEFT), Some(ResizeDirection::NorthWest));
    }
}
