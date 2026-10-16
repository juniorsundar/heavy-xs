//! `src/renwindow.c`, plus the window-management block of `src/renderer.c`.
//!
//! A `RenWindow` is the per-window state: the SDL window, its surface (or SDL renderer +
//! texture), the scale between window points and surface pixels, and the frame's command
//! buffer that `rencache.rs` fills. The struct lives here and is **opaque to C** — the C that
//! remains (`renderer.c` for FreeType and the drawing primitives, `main.c`, `api/renderer.c`)
//! goes through the `ren_*` functions below. That is deliberate: the struct's layout changes
//! with `LITE_USE_SDL_RENDERER`, and mirroring it in a header would be a silent
//! memory-corruption bug when the two drift.
//!
//! `renderer.c`'s window bookkeeping came along too (the list, `ren_create`/`ren_destroy`, the
//! target window, `ren_get_size`, `ren_update_rects`), because all of it reads the struct's
//! fields. `renderer.c` keeps what it is good at (FreeType and pixels); the `ren_*` declarations
//! live on in `src/renderer.h`, and `api/renwindow.c` (the Lua binding) still calls them.

use std::ffi::{c_int, c_void};
use std::sync::Mutex;

use sdl3_sys::everything as sdl;

/// `RenRect` from `src/renderer.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RenRect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

/// `RenSurface` from `src/renderer.h`: a surface plus the scale its coordinates are in.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RenSurface {
    pub surface: *mut sdl::SDL_Surface,
    pub scale: c_int,
}

/// `struct RenWindow`.
pub struct RenWindow {
    window: *mut sdl::SDL_Window,
    /// The frame's command buffer, allocated by `rencache` through `command_buf_reserve`.
    pub command_buf: *mut u8,
    pub command_buf_idx: usize,
    pub command_buf_size: usize,
    scale_x: f32,
    scale_y: f32,
    #[cfg(feature = "sdl_renderer")]
    renderer: *mut sdl::SDL_Renderer,
    #[cfg(feature = "sdl_renderer")]
    texture: *mut sdl::SDL_Texture,
    #[cfg(feature = "sdl_renderer")]
    rensurface: RenSurface,
}

// ------------------------------------------------------------ the window list

/// `renderer.c`'s `window_list` / `window_count` / `target_window`.
struct State {
    list: Vec<*mut RenWindow>,
    target: *mut RenWindow,
}

// Single-threaded: Lua and SDL events all run on the main thread.
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
/// `win` is a window from `SDL_CreateWindow` that is not already owned by a `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_create(win: *mut sdl::SDL_Window) -> *mut RenWindow {
    assert!(!win.is_null());
    let ren = Box::into_raw(Box::new(RenWindow {
        window: win,
        command_buf: std::ptr::null_mut(),
        command_buf_idx: 0,
        command_buf_size: 0,
        scale_x: 1.0,
        scale_y: 1.0,
        #[cfg(feature = "sdl_renderer")]
        renderer: std::ptr::null_mut(),
        #[cfg(feature = "sdl_renderer")]
        texture: std::ptr::null_mut(),
        #[cfg(feature = "sdl_renderer")]
        rensurface: RenSurface { surface: std::ptr::null_mut(), scale: 1 },
    }));
    init_surface(ren);
    (*ren).command_buf_idx = 0;
    (*ren).command_buf_size = 0;
    clip_to_surface(ren);
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
    });
    free_surface(ren);
    if !(*ren).command_buf.is_null() {
        sdl::SDL_free((*ren).command_buf as *mut c_void);
        (*ren).command_buf = std::ptr::null_mut();
        (*ren).command_buf_size = 0;
    }
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

/// `ren_find_window`.
///
/// # Safety
/// `window` is NULL or a live SDL window.
#[no_mangle]
pub unsafe extern "C" fn ren_find_window(window: *mut sdl::SDL_Window) -> *mut RenWindow {
    with_state(|state| {
        state.list.iter().copied().find(|w| (**w).window == window).unwrap_or(std::ptr::null_mut())
    })
}

/// `ren_find_window_from_id`.
#[no_mangle]
pub unsafe extern "C" fn ren_find_window_from_id(id: sdl::SDL_WindowID) -> *mut RenWindow {
    let window = sdl::SDL_GetWindowFromID(id);
    ren_find_window(window)
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

/// `ren_get_size`: the surface size, in points.
///
/// # Safety
/// `ren` is a live `RenWindow`; `x` and `y` are valid pointers.
#[no_mangle]
pub unsafe extern "C" fn ren_get_size(ren: *mut RenWindow, x: *mut c_int, y: *mut c_int) {
    let rs = get_surface(ren);
    *x = (*rs.surface).w;
    *y = (*rs.surface).h;
    #[cfg(feature = "sdl_renderer")]
    {
        *x /= rs.scale;
        *y /= rs.scale;
    }
}

/// `ren_resize_window`.
///
/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_resize_window(ren: *mut RenWindow) {
    resize_surface(ren);
    update_scale(ren);
}

/// `ren_update_rects`: shows the window on the first frame.
///
/// # Safety
/// `rects` points to `count` `RenRect`s.
#[no_mangle]
pub unsafe extern "C" fn ren_update_rects(ren: *mut RenWindow, rects: *mut RenRect, count: c_int) {
    static INITIAL_FRAME: Mutex<bool> = Mutex::new(true);
    update_rects(ren, rects, count);
    let mut initial = INITIAL_FRAME.lock().unwrap();
    if *initial {
        show_window(ren);
        *initial = false;
    }
}

/// `ren_set_clip_rect`.
///
/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn ren_set_clip_rect(ren: *mut RenWindow, rect: RenRect) {
    set_clip_rect(ren, rect);
}

// --------------------------------------------------------------- the Rust API

/// `renwin_get_surface`. Also used by `renderer.c` (`update_font_scale`).
///
/// # Safety
/// `ren` is a live `RenWindow`.
#[no_mangle]
pub unsafe extern "C" fn renwin_get_surface(ren: *mut RenWindow) -> RenSurface {
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
pub unsafe extern "C" fn ren_get_sdl_window(ren: *mut RenWindow) -> *mut sdl::SDL_Window {
    (*ren).window
}

// ------------------------------------------------------------- surface handling

#[cfg(feature = "sdl_renderer")]
unsafe fn query_surface_scale(ren: *mut RenWindow) -> c_int {
    let mut w_pixels = 0;
    let mut h_pixels = 0;
    let mut w_points = 0;
    let mut h_points = 0;
    sdl::SDL_GetWindowSizeInPixels((*ren).window, &mut w_pixels, &mut h_pixels);
    sdl::SDL_GetWindowSize((*ren).window, &mut w_points, &mut h_points);
    // The ratio pixel/point is an integer and the same on both axes (as the C asserts).
    assert!(w_pixels % w_points == 0 && h_pixels % h_points == 0);
    assert_eq!(w_pixels / w_points, h_pixels / h_points);
    w_pixels / w_points
}

#[cfg(feature = "sdl_renderer")]
unsafe fn setup_renderer(ren: *mut RenWindow, w: c_int, h: c_int) {
    // w and h are in pixels, from SDL_GetWindowSizeInPixels.
    if (*ren).renderer.is_null() {
        (*ren).renderer = sdl::SDL_CreateRenderer((*ren).window, std::ptr::null());
    }
    if !(*ren).texture.is_null() {
        sdl::SDL_DestroyTexture((*ren).texture);
    }
    (*ren).texture = sdl::SDL_CreateTexture(
        (*ren).renderer,
        (*(*ren).rensurface.surface).format,
        sdl::SDL_TextureAccess::STREAMING,
        w,
        h,
    );
    (*ren).rensurface.scale = query_surface_scale(ren);
}

unsafe fn init_surface(ren: *mut RenWindow) {
    (*ren).scale_x = 1.0;
    (*ren).scale_y = 1.0;
    #[cfg(feature = "sdl_renderer")]
    {
        if !(*ren).rensurface.surface.is_null() {
            sdl::SDL_DestroySurface((*ren).rensurface.surface);
        }
        let mut w = 0;
        let mut h = 0;
        sdl::SDL_GetWindowSizeInPixels((*ren).window, &mut w, &mut h);
        let format = sdl::SDL_GetWindowPixelFormat((*ren).window);
        (*ren).rensurface.surface = sdl::SDL_CreateSurface(
            w,
            h,
            if format == sdl::SDL_PixelFormat::UNKNOWN { sdl::SDL_PixelFormat::BGRA32 } else { format },
        );
        if (*ren).rensurface.surface.is_null() {
            eprintln!("Error creating surface: {}", sdl_error());
            std::process::exit(1);
        }
        setup_renderer(ren, w, h);
    }
}

unsafe fn free_surface(ren: *mut RenWindow) {
    #[cfg(feature = "sdl_renderer")]
    {
        sdl::SDL_DestroyTexture((*ren).texture);
        sdl::SDL_DestroyRenderer((*ren).renderer);
        sdl::SDL_DestroySurface((*ren).rensurface.surface);
    }
    sdl::SDL_DestroyWindow((*ren).window);
    (*ren).window = std::ptr::null_mut();
}

unsafe fn get_surface(ren: *mut RenWindow) -> RenSurface {
    #[cfg(feature = "sdl_renderer")]
    {
        (*ren).rensurface
    }
    #[cfg(not(feature = "sdl_renderer"))]
    {
        let surface = sdl::SDL_GetWindowSurface((*ren).window);
        if surface.is_null() {
            eprintln!("Error getting window surface: {}", sdl_error());
            std::process::exit(1);
        }
        RenSurface { surface, scale: 1 }
    }
}

fn scaled_rect(rect: RenRect, scale: c_int) -> RenRect {
    RenRect {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

unsafe fn clip_to_surface(ren: *mut RenWindow) {
    sdl::SDL_SetSurfaceClipRect(get_surface(ren).surface, std::ptr::null());
}

unsafe fn set_clip_rect(ren: *mut RenWindow, rect: RenRect) {
    let rs = get_surface(ren);
    let sr = scaled_rect(rect, rs.scale);
    sdl::SDL_SetSurfaceClipRect(
        rs.surface,
        &sdl::SDL_Rect { x: sr.x, y: sr.y, w: sr.width, h: sr.height },
    );
}

#[cfg_attr(not(feature = "sdl_renderer"), allow(unused_variables))]
unsafe fn resize_surface(ren: *mut RenWindow) {
    #[cfg(feature = "sdl_renderer")]
    {
        let mut new_w = 0;
        let mut new_h = 0;
        sdl::SDL_GetWindowSizeInPixels((*ren).window, &mut new_w, &mut new_h);
        let new_scale = query_surface_scale(ren);
        // (w, h) may differ from (new_w, new_h) on retina displays.
        if new_scale != (*ren).rensurface.scale
            || new_w != (*(*ren).rensurface.surface).w
            || new_h != (*(*ren).rensurface.surface).h
        {
            init_surface(ren);
            clip_to_surface(ren);
            setup_renderer(ren, new_w, new_h);
        }
    }
}

unsafe fn update_scale(ren: *mut RenWindow) {
    #[cfg(not(feature = "sdl_renderer"))]
    {
        let surface = sdl::SDL_GetWindowSurface((*ren).window);
        let mut window_w = 0;
        let mut window_h = 0;
        sdl::SDL_GetWindowSize((*ren).window, &mut window_w, &mut window_h);
        (*ren).scale_x = (*surface).w as f32 / window_w as f32;
        (*ren).scale_y = (*surface).h as f32 / window_h as f32;
    }
    #[cfg(feature = "sdl_renderer")]
    {
        let _ = ren;
    }
}

unsafe fn show_window(ren: *mut RenWindow) {
    sdl::SDL_ShowWindow((*ren).window);
}

unsafe fn update_rects(ren: *mut RenWindow, rects: *mut RenRect, count: c_int) {
    #[cfg(feature = "sdl_renderer")]
    {
        let scale = (*ren).rensurface.scale;
        let surface = (*ren).rensurface.surface;
        for i in 0..count {
            let r = &*rects.offset(i as isize);
            let x = scale * r.x;
            let y = scale * r.y;
            let w = scale * r.width;
            let h = scale * r.height;
            let sr = sdl::SDL_Rect { x, y, w, h };
            let pixels = ((*surface).pixels as *mut u8)
                .offset((y * (*surface).pitch + x * sdl::SDL_BYTESPERPIXEL((*surface).format) as i32) as isize);
            sdl::SDL_UpdateTexture((*ren).texture, &sr, pixels as *const c_void, (*surface).pitch);
        }
        sdl::SDL_RenderTexture((*ren).renderer, (*ren).texture, std::ptr::null(), std::ptr::null());
        sdl::SDL_RenderPresent((*ren).renderer);
    }
    #[cfg(not(feature = "sdl_renderer"))]
    {
        sdl::SDL_UpdateWindowSurfaceRects((*ren).window, rects as *const sdl::SDL_Rect, count);
    }
}

/// `renderer.c`'s `ren_refresh_after_foreground`: the reaction to
/// `SDL_EVENT_{WILL,DID}_ENTER_FOREGROUND`.
#[no_mangle]
pub extern "C" fn ren_refresh_after_foreground() {
    #[cfg(feature = "sdl_renderer")]
    {
        crate::rencache::rencache_invalidate();
    }
    #[cfg(not(feature = "sdl_renderer"))]
    unsafe {
        let mut list: *mut *mut RenWindow = std::ptr::null_mut();
        let mut count = ren_get_window_list(&mut list);
        while count > 0 {
            count -= 1;
            sdl::SDL_UpdateWindowSurface((**list.add(count)).window);
        }
    }
}

pub(crate) fn sdl_error() -> String {
    unsafe {
        let message = sdl::SDL_GetError();
        if message.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(message).to_string_lossy().into_owned()
        }
    }
}

/// `expand_command_buffer` from `rencache.c`: grow the frame's command buffer to `new_size`
/// bytes. Returns false when `SDL_realloc` fails.
///
/// # Safety
/// `ren` is a live `RenWindow`.
pub(crate) unsafe fn expand_command_buffer(ren: *mut RenWindow, new_size: usize) -> bool {
    let new_buf = sdl::SDL_realloc((*ren).command_buf as *mut c_void, new_size);
    if new_buf.is_null() {
        return false;
    }
    (*ren).command_buf_size = new_size;
    (*ren).command_buf = new_buf as *mut u8;
    true
}
