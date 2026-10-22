//! `src/renwindow.c`, plus the window-management block of `src/renderer.c`.
//!
//! A `RenWindow` owns a native registry id, an opaque packed Rust pixel store, the
//! scale between window points and physical pixels, and the frame's command buffer that
//! `rencache.rs` fills. The struct lives here and is opaque across the `ren_*` interface used
//! by `renderer.rs`, `rencache.rs` and the Lua bindings; callers must not mirror it in a header.
//!
//! `renderer.c`'s window bookkeeping came along too (the list, `ren_create`/`ren_destroy`, the
//! target window, `ren_get_size`, `ren_update_rects`), because all of it reads the struct's
//! fields. The Lua binding is `renwindow_api.rs`.

use std::ffi::c_int;
use std::sync::Mutex;


/// `RenRect` from `src/renderer.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RenRect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

/// One colour channel of a packed pixel.
#[derive(Clone, Copy)]
pub struct Channel {
    pub mask: u32,
    pub shift: u32,
}

impl Channel {
    pub fn get(self, pixel: u32) -> u32 {
        (pixel & self.mask) >> self.shift
    }
}

/// `RenSurface` from `src/renderer.h`: one frame's view of a window's pixels plus the scale its
/// coordinates are in. The clip lives here, not in the pixel storage: drawing only happens inside
/// rencache's redraw loop, which sets it before every replayed rect.
#[derive(Clone, Copy)]
pub struct RenSurface {
    pub pixels: *mut u8,
    pub w: c_int,
    pub h: c_int,
    pub pitch: c_int,
    pub bytes_per_pixel: c_int,
    pub r: Channel,
    pub g: Channel,
    pub b: Channel,
    pub a: Channel,
    pub clip: RenRect,
    pub scale: c_int,
}

impl RenSurface {
    /// `SDL_SetSurfaceClipRect`: the rect within the surface, kept unchanged on overflow.
    pub fn set_clip(&mut self, rect: RenRect) {
        let full = RenRect { x: 0, y: 0, width: self.w, height: self.h };
        if let Some(clip) = rect_intersection(scaled_rect(rect, self.scale), full) {
            self.clip = clip;
        }
    }
}

/// `SDL_RectEmpty`.
pub fn rect_empty(r: RenRect) -> bool {
    r.width <= 0 || r.height <= 0
}

/// `SDL_GetRectIntersection`'s arithmetic: `None` when either rect could overflow (SDL then
/// leaves its result untouched), otherwise the intersection, which may be empty.
pub fn rect_intersection(a: RenRect, b: RenRect) -> Option<RenRect> {
    let can_overflow = |r: RenRect| {
        r.x <= i32::MIN / 2
            || r.x >= i32::MAX / 2
            || r.y <= i32::MIN / 2
            || r.y >= i32::MAX / 2
            || r.width >= i32::MAX / 2
            || r.height >= i32::MAX / 2
    };
    if can_overflow(a) || can_overflow(b) {
        return None;
    }
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Some(RenRect {
        x,
        y,
        width: (a.x + a.width).min(b.x + b.width) - x,
        height: (a.y + a.height).min(b.y + b.height) - y,
    })
}

/// `struct RenWindow`.
pub struct RenWindow {
    window: u32,
    pixels: Vec<u32>,
    width: i32,
    height: i32,
    /// The frame's command buffer, allocated by `rencache` through `command_buf_reserve`.
    pub command_buf: *mut u8,
    pub command_buf_idx: usize,
    pub command_buf_size: usize,
    scale_x: f32,
    scale_y: f32,
}

// ------------------------------------------------------------ the window list

/// `renderer.c`'s `window_list` / `window_count` / `target_window`.
struct State {
    list: Vec<*mut RenWindow>,
    target: *mut RenWindow,
}

// Single-threaded: Lua and native events run on the main thread.
unsafe impl Send for State {}
static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE.lock().unwrap();
    let state = guard.get_or_insert_with(|| State { list: Vec::new(), target: std::ptr::null_mut() });
    f(state)
}

/// `ren_create`: allocate, initialise the surface and command buffer, add to the list.
///
/// # Safety
/// `win` is a live native registry id not already owned by a `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_create(win: u32) -> *mut RenWindow {
    assert_ne!(win, 0);
    let ren = Box::into_raw(Box::new(RenWindow {
        window: win,
        pixels: Vec::new(), width: 0, height: 0,
        command_buf: std::ptr::null_mut(),
        command_buf_idx: 0,
        command_buf_size: 0,
        scale_x: 1.0,
        scale_y: 1.0,
    }));
    init_surface(ren);
    (*ren).command_buf_idx = 0;
    (*ren).command_buf_size = 0;
    with_state(|state| state.list.push(ren));
    ren
}

/// `ren_destroy`.
///
/// # Safety
/// `ren` came from `ren_create` and is not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn ren_destroy(ren: *mut RenWindow) {
    assert!(!ren.is_null());
    with_state(|state| {
        if let Some(index) = state.list.iter().position(|w| *w == ren) {
            state.list.remove(index);
        }
        if state.target == ren { state.target = std::ptr::null_mut(); }
    });
    free_surface(ren);
    free_command_buffer(ren);
    drop(Box::from_raw(ren));
}

/// `ren_get_window_list`: the C pointer is to the list's buffer, valid until the next add or
/// remove.
///
/// # Safety
/// `window_list_dest` is a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn ren_get_window_list(window_list_dest: *mut *mut *mut RenWindow) -> usize {
    with_state(|state| {
        *window_list_dest = state.list.as_mut_ptr();
        state.list.len()
    })
}

/// Find a live app-owned window, never a producer's pointer.
#[no_mangle]
pub unsafe extern "C" fn ren_find_window_from_id(id: u32) -> *mut RenWindow {
    with_state(|state| state.list.iter().copied().find(|w| (**w).window == id).unwrap_or(std::ptr::null_mut()))
}

#[no_mangle]
pub extern "C" fn ren_get_target_window() -> *mut RenWindow {
    with_state(|state| state.target)
}

/// # Safety
/// `window` is NULL or a window that outlives the target-window period.
#[no_mangle]
pub unsafe extern "C" fn ren_set_target_window(window: *mut RenWindow) {
    with_state(|state| state.target = window);
}

/// `ren_get_size`: the physical draw-target size (as in the previous SDL surface path).
///
/// # Safety
/// `ren` is a live `RenWindow`; `x` and `y` are valid pointers.
#[no_mangle]
pub unsafe extern "C" fn ren_get_size(ren: *mut RenWindow, x: *mut c_int, y: *mut c_int) {
    let rs = get_surface(ren);
    *x = rs.w;
    *y = rs.h;
}

/// `ren_resize_window`: synchronize the owned pixel store and point-to-pixel scale.
///
/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_resize_window(ren: *mut RenWindow) {
    update_scale(ren);
}

/// `ren_update_rects`: shows the window on the first frame.
///
/// # Safety
/// `rects` points to `count` `RenRect`s.
#[no_mangle]
pub unsafe extern "C" fn ren_update_rects(ren: *mut RenWindow, rects: *mut RenRect, count: c_int) {
    update_rects(ren, rects, count);
}

// --------------------------------------------------------------- the Rust API

/// `renwin_get_surface`: valid until the window surface changes (resize) or the window goes.
///
/// # Safety
/// `ren` is a live `RenWindow`.
pub unsafe fn renwin_get_surface(ren: *mut RenWindow) -> RenSurface {
    get_surface(ren)
}

/// Accessors for the Rust event layer (`sysevents.rs`).
///
/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_get_scale_x(ren: *mut RenWindow) -> f32 {
    (*ren).scale_x
}

/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_get_scale_y(ren: *mut RenWindow) -> f32 {
    (*ren).scale_y
}

/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_get_window_id(ren: *mut RenWindow) -> u32 {
    (*ren).window
}

// ------------------------------------------------------------- surface handling

unsafe fn init_surface(ren: *mut RenWindow) { update_scale(ren); }

unsafe fn free_surface(ren: *mut RenWindow) { crate::native::destroy((*ren).window); }

/// A fresh view is acquired after every resize; no pointer into the store crosses a pump.
unsafe fn get_surface(ren: *mut RenWindow) -> RenSurface {
    update_scale(ren);
    let ren = &mut *ren;
    RenSurface {
        pixels: ren.pixels.as_mut_ptr() as *mut u8, w: ren.width, h: ren.height,
        pitch: ren.width * 4, bytes_per_pixel: 4,
        r: Channel { mask: 0xff0000, shift: 16 },
        g: Channel { mask: 0xff00, shift: 8 },
        b: Channel { mask: 0xff, shift: 0 },
        a: Channel { mask: 0xff000000, shift: 24 },
        clip: RenRect { x: 0, y: 0, width: ren.width, height: ren.height }, scale: 1,
    }
}

fn scaled_rect(rect: RenRect, scale: c_int) -> RenRect {
    RenRect { x: rect.x * scale, y: rect.y * scale, width: rect.width * scale, height: rect.height * scale }
}

unsafe fn update_scale(ren: *mut RenWindow) {
    let (w, h) = crate::native::physical_size((*ren).window);
    let ren = &mut *ren;
    if (ren.width, ren.height) != (w, h) {
        // Rows remain tightly packed; resize invalidates the whole cache through its size check.
        let length = (w.max(0) as usize).checked_mul(h.max(0) as usize).expect("pixel store size overflow");
        assert!(w <= i32::MAX / 4, "pixel store pitch overflow");
        ren.pixels = vec![0xff000000; length];
        ren.width = w; ren.height = h;
    }
    ren.scale_x = crate::native::scale(ren.window);
    ren.scale_y = ren.scale_x;
}

unsafe fn update_rects(ren: *mut RenWindow, _rects: *mut RenRect, _count: c_int) {
    let ren = &*ren;
    if let Err(error) = crate::native::present(ren.window, &ren.pixels, ren.width, ren.height, true) {
        crate::platform::report_error(error);
    }
}

/// Replay the persistent store after foreground restoration, without retaining a platform borrow.
#[no_mangle]
pub extern "C" fn ren_refresh_after_foreground() {
    let windows = with_state(|state| state.list.clone());
    for ren in windows {
        unsafe {
            let ren = &*ren;
            if let Err(error) = crate::native::present(ren.window, &ren.pixels, ren.width, ren.height, false) {
                crate::platform::report_error(error);
            }
        }
    }
}

/// The command buffer's alignment: `max_align_t`'s, which rencache pads every command to and
/// which the C got from `realloc`.
const COMMAND_BUF_ALIGN: usize = 16;

/// `expand_command_buffer` from `rencache.c`: grow the frame's command buffer to `new_size`
/// bytes, keeping its contents. Returns false when the allocation fails (the old buffer stays).
///
/// # Safety
/// `ren` is a live `RenWindow`.
pub(crate) unsafe fn expand_command_buffer(ren: *mut RenWindow, new_size: usize) -> bool {
    let Ok(new_layout) = std::alloc::Layout::from_size_align(new_size, COMMAND_BUF_ALIGN) else {
        return false;
    };
    let new_buf = if (*ren).command_buf.is_null() {
        std::alloc::alloc(new_layout)
    } else {
        let old = std::alloc::Layout::from_size_align_unchecked((*ren).command_buf_size, COMMAND_BUF_ALIGN);
        std::alloc::realloc((*ren).command_buf, old, new_size)
    };
    if new_buf.is_null() {
        return false;
    }
    (*ren).command_buf_size = new_size;
    (*ren).command_buf = new_buf;
    true
}

/// Frees what `expand_command_buffer` allocated; `command_buf_size` is always its layout size.
///
/// # Safety
/// `ren` is a live `RenWindow`.
unsafe fn free_command_buffer(ren: *mut RenWindow) {
    if !(*ren).command_buf.is_null() {
        let layout = std::alloc::Layout::from_size_align_unchecked((*ren).command_buf_size, COMMAND_BUF_ALIGN);
        std::alloc::dealloc((*ren).command_buf, layout);
        (*ren).command_buf = std::ptr::null_mut();
        (*ren).command_buf_size = 0;
    }
}

#[cfg(test)]
mod command_buffer {
    use super::*;

    fn bare_window() -> RenWindow {
        RenWindow {
            window: 0, pixels: Vec::new(), width: 0, height: 0,
            command_buf: std::ptr::null_mut(),
            command_buf_idx: 0,
            command_buf_size: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }

    /// rencache's growth from empty (512 KiB, then x1.2) well past a typical frame: every step
    /// keeps the bytes written so far and the alignment, and the buffer is freed once.
    #[test]
    fn grows_keeps_contents_and_frees() {
        let mut ren = bare_window();
        let ren: *mut RenWindow = &mut ren;
        unsafe {
            let mut size = 0usize;
            let mut written = 0usize;
            while size < 64 << 20 {
                size = if size == 0 { 1024 * 512 } else { (size as f64 * 1.2) as usize };
                assert!(expand_command_buffer(ren, size));
                assert_eq!((*ren).command_buf_size, size);
                assert_eq!((*ren).command_buf as usize % COMMAND_BUF_ALIGN, 0);
                for i in 0..written {
                    assert_eq!(*(*ren).command_buf.add(i), (i % 251) as u8, "byte {i} after growing to {size}");
                }
                for i in written..size {
                    *(*ren).command_buf.add(i) = (i % 251) as u8;
                }
                written = size;
            }
            free_command_buffer(ren);
            assert!((*ren).command_buf.is_null());
            assert_eq!((*ren).command_buf_size, 0);
            free_command_buffer(ren); // a second teardown is a no-op, not a double free
        }
    }

    #[test]
    fn impossible_size_fails_and_keeps_the_buffer() {
        let mut ren = bare_window();
        let ren: *mut RenWindow = &mut ren;
        unsafe {
            assert!(expand_command_buffer(ren, 1024));
            *(*ren).command_buf = 7;
            assert!(!expand_command_buffer(ren, usize::MAX - 8));
            assert_eq!((*ren).command_buf_size, 1024);
            assert_eq!(*(*ren).command_buf, 7);
            free_command_buffer(ren);
        }
    }
}

#[cfg(test)]
mod clip {
    use super::*;

    fn rect(x: c_int, y: c_int, width: c_int, height: c_int) -> RenRect {
        RenRect { x, y, width, height }
    }

    #[test]
    fn matches_sdl_rect_intersection() {
        let full = rect(0, 0, 800, 480);
        assert_eq!(rect_intersection(rect(790, 10, 100, 10), full), Some(rect(790, 10, 10, 10)));
        // Disjoint rects give SDL's negative-extent (empty) result, not None.
        let apart = rect_intersection(rect(5000, 5000, 20, 20), full).unwrap();
        assert!(rect_empty(apart));
        assert_eq!(rect_intersection(rect(i32::MAX / 2, 0, 1, 1), full), None);
    }

    #[test]
    fn set_clip_keeps_previous_on_overflow() {
        let mut rs = RenSurface {
            pixels: std::ptr::null_mut(),
            w: 800,
            h: 480,
            pitch: 3200,
            bytes_per_pixel: 4,
            r: Channel { mask: 0xff0000, shift: 16 },
            g: Channel { mask: 0xff00, shift: 8 },
            b: Channel { mask: 0xff, shift: 0 },
            a: Channel { mask: 0, shift: 0 },
            clip: rect(0, 0, 800, 480),
            scale: 1,
        };
        rs.set_clip(rect(10, 20, 30, 40));
        assert_eq!(rs.clip, rect(10, 20, 30, 40));
        rs.set_clip(rect(0, 0, i32::MAX / 2, 1));
        assert_eq!(rs.clip, rect(10, 20, 30, 40));
    }
}
