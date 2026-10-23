//! `src/renwindow.c`, plus the window-management block of `src/renderer.c`.
//!
//! A `RenWindow` owns a native registry id, a packed pixel store, the scale between window
//! points and physical pixels, and the frame's commands that `rencache.rs` records. It is
//! shared as a [`WindowRef`]: the Lua userdata, the frame's target and the persistent-window
//! slot each hold a reference, and the native window goes when the last one does.
//!
//! `renderer.c`'s window bookkeeping came along too (the list, the target window, the size and
//! presentation), because all of it reads the struct's fields. The Lua binding is
//! `renwindow_api.rs`.

use std::cell::RefCell;
use std::ffi::c_int;
use std::rc::{Rc, Weak};

use crate::rencache::Command;

/// `RenRect` from `src/renderer.h`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
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

/// The pixel store's layout: 0xAARRGGBB.
pub const R: Channel = Channel { mask: 0xff0000, shift: 16 };
pub const G: Channel = Channel { mask: 0xff00, shift: 8 };
pub const B: Channel = Channel { mask: 0xff, shift: 0 };
pub const A: Channel = Channel { mask: 0xff000000, shift: 24 };

/// `RenSurface` from `src/renderer.h`: one frame's view of a window's pixels plus the scale its
/// coordinates are in. The clip lives here, not in the pixel storage: drawing only happens inside
/// rencache's redraw loop, which sets it before every replayed rect.
pub struct RenSurface<'a> {
    /// `w * h` pixels, rows tightly packed.
    pub pixels: &'a mut [u32],
    pub w: c_int,
    pub h: c_int,
    pub clip: RenRect,
    pub scale: c_int,
}

impl RenSurface<'_> {
    /// `SDL_SetSurfaceClipRect`: the rect within the surface, kept unchanged on overflow.
    pub fn set_clip(&mut self, rect: RenRect) {
        let full = RenRect { x: 0, y: 0, width: self.w, height: self.h };
        if let Some(clip) = rect_intersection(scaled_rect(rect, self.scale), full) {
            self.clip = clip;
        }
    }

    /// Pixels `x..x + len` of row `y`, or `None` if that is not inside the surface. The callers
    /// have clipped to the surface already, so `None` would be a bug that is better not drawn.
    pub fn row(&mut self, y: c_int, x: c_int, len: usize) -> Option<&mut [u32]> {
        if y < 0 || x < 0 || y >= self.h || x as usize + len > self.w as usize {
            return None;
        }
        let start = y as usize * self.w as usize + x as usize;
        self.pixels.get_mut(start..start + len)
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
    /// The frame's commands, recorded by `rencache` and replayed at the end of the frame.
    pub(crate) commands: Vec<Command>,
    /// The text of the frame's `DrawText` commands, which hold ranges into it.
    pub(crate) text: Vec<u8>,
    scale_x: f32,
    scale_y: f32,
}

pub type WindowRef = Rc<RefCell<RenWindow>>;

// ------------------------------------------------------------ the window list

/// `renderer.c`'s `window_list` / `target_window`. Lua and native events run on the main
/// thread, so this is per-thread state. The list does not keep windows alive.
#[derive(Default)]
struct State {
    list: Vec<Weak<RefCell<RenWindow>>>,
    target: Option<WindowRef>,
}

thread_local! { static STATE: RefCell<State> = RefCell::default(); }

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|state| f(&mut state.borrow_mut()))
}

/// `ren_create`: wrap a native registry id not already owned by a `RenWindow`, and add it to
/// the list.
pub fn create(win: u32) -> WindowRef {
    assert_ne!(win, 0);
    let mut ren = RenWindow {
        window: win,
        pixels: Vec::new(),
        width: 0,
        height: 0,
        commands: Vec::new(),
        text: Vec::new(),
        scale_x: 1.0,
        scale_y: 1.0,
    };
    ren.update_scale();
    let ren = Rc::new(RefCell::new(ren));
    with_state(|state| {
        state.list.retain(|w| w.strong_count() > 0);
        state.list.push(Rc::downgrade(&ren));
    });
    ren
}

impl Drop for RenWindow {
    /// `ren_destroy`, once the last reference is gone.
    fn drop(&mut self) {
        crate::native::destroy(self.window);
    }
}

/// The live window with this native id, if any.
pub fn find(id: u32) -> Option<WindowRef> {
    with_state(|state| state.list.iter().filter_map(Weak::upgrade).find(|w| w.borrow().window == id))
}

pub fn target() -> Option<WindowRef> {
    with_state(|state| state.target.clone())
}

pub fn set_target(window: Option<WindowRef>) {
    // Dropped outside the borrow: the last reference going runs `Drop`.
    let _previous = with_state(|state| std::mem::replace(&mut state.target, window));
}

/// Replay the persistent store after foreground restoration, without retaining a platform borrow.
pub fn refresh_after_foreground() {
    let windows: Vec<WindowRef> = with_state(|state| state.list.iter().filter_map(Weak::upgrade).collect());
    for ren in windows {
        let ren = ren.borrow();
        if let Err(error) = crate::native::present(ren.window, &ren.pixels, ren.width, ren.height, false) {
            crate::platform::report_error(error);
        }
    }
}

impl RenWindow {
    pub fn id(&self) -> u32 {
        self.window
    }

    pub fn scale_x(&self) -> f32 {
        self.scale_x
    }

    pub fn scale_y(&self) -> f32 {
        self.scale_y
    }

    /// `ren_get_size`: the physical draw-target size (as in the previous SDL surface path).
    pub fn size(&mut self) -> (c_int, c_int) {
        self.update_scale();
        (self.width, self.height)
    }

    /// `ren_resize_window`: synchronize the owned pixel store and point-to-pixel scale.
    pub fn resize(&mut self) {
        self.update_scale();
    }

    /// `renwin_get_surface`: a fresh view after every resize; it borrows the window, so no view
    /// outlives a pump.
    pub fn surface(&mut self) -> RenSurface<'_> {
        self.update_scale();
        RenSurface {
            w: self.width,
            h: self.height,
            clip: RenRect { x: 0, y: 0, width: self.width, height: self.height },
            scale: 1,
            pixels: &mut self.pixels,
        }
    }

    /// `ren_update_rects`: presents the store, which shows the window on the first frame.
    pub fn update_rects(&self) {
        if let Err(error) = crate::native::present(self.window, &self.pixels, self.width, self.height, true) {
            crate::platform::report_error(error);
        }
    }

    fn update_scale(&mut self) {
        let (w, h) = crate::native::physical_size(self.window);
        if (self.width, self.height) != (w, h) {
            // Rows remain tightly packed; resize invalidates the whole cache through its size check.
            let length = (w.max(0) as usize).checked_mul(h.max(0) as usize).expect("pixel store size overflow");
            assert!(w <= i32::MAX / 4, "pixel store pitch overflow");
            self.pixels = vec![0xff000000; length];
            self.width = w;
            self.height = h;
        }
        self.scale_x = crate::native::scale(self.window);
        self.scale_y = self.scale_x;
    }
}

fn scaled_rect(rect: RenRect, scale: c_int) -> RenRect {
    RenRect { x: rect.x * scale, y: rect.y * scale, width: rect.width * scale, height: rect.height * scale }
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
        let mut pixels = vec![0u32; 800 * 480];
        let mut rs = RenSurface { pixels: &mut pixels, w: 800, h: 480, clip: rect(0, 0, 800, 480), scale: 1 };
        rs.set_clip(rect(10, 20, 30, 40));
        assert_eq!(rs.clip, rect(10, 20, 30, 40));
        rs.set_clip(rect(0, 0, i32::MAX / 2, 1));
        assert_eq!(rs.clip, rect(10, 20, 30, 40));
    }
}
