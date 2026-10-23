//! `src/api/renwindow.c`: the `renwindow` Lua module (`RenWindow` userdata).
//!
//! The userdata holds one owning [`WindowRef`] (as `Rc::into_raw`), released by `__gc`. One
//! window may be marked persistent so it survives the Lua state being recreated on restart
//! (`core.restart_request`): the persistent slot keeps its own reference, and `_restore` hands
//! the new state another.
//!
//! The native owner creates the winit window; this module only asks for it and wraps the
//! returned registry id in a `RenWindow`.

use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr};
use std::rc::Rc;

use mlua::ffi::{
    luaL_checkstring, luaL_checkudata, luaL_error, luaL_newmetatable, luaL_optnumber,
    luaL_setmetatable, lua_CFunction, lua_State, lua_newuserdata, lua_pushlstring, lua_pushnil,
    lua_pushnumber, lua_pushvalue, lua_setfield, lua_tostring,
};

use crate::native;
use crate::renwindow::{RenWindow, WindowRef};

const RENWINDOW_TYPE: &CStr = c"RenWindow";

thread_local! { static PERSISTENT_WINDOW: RefCell<Option<WindowRef>> = const { RefCell::new(None) }; }

/// The userdata's payload: an `Rc::into_raw` pointer, NULL once `__gc` has released it.
type Slot = *const RefCell<RenWindow>;

/// The window in a `RenWindow` userdata at `idx` (raising if it is not one), or `None` once
/// collected. Raises before taking a reference, so a Lua error never skips its release.
///
/// # Safety
/// `l` is a live Lua state.
pub unsafe fn window_at(l: *mut lua_State, idx: c_int) -> Option<WindowRef> {
    let slot = *(luaL_checkudata(l, idx, RENWINDOW_TYPE.as_ptr()) as *const Slot);
    if slot.is_null() {
        return None;
    }
    // SAFETY: a non-NULL slot holds one strong count, which stays until `__gc`.
    Rc::increment_strong_count(slot);
    Some(Rc::from_raw(slot))
}

/// The native id of the window at `idx`, 0 once collected.
///
/// # Safety
/// `l` is a live Lua state.
pub unsafe fn window_id_at(l: *mut lua_State, idx: c_int) -> u32 {
    let slot = *(luaL_checkudata(l, idx, RENWINDOW_TYPE.as_ptr()) as *const Slot);
    // SAFETY: as in `window_at`; the reference is only borrowed for the read.
    if slot.is_null() { 0 } else { (*slot).borrow().id() }
}

/// `window_at` for the C functions that dereferenced the userdata unchecked: raises instead.
///
/// # Safety
/// `l` is a live Lua state.
pub unsafe fn check_window_at(l: *mut lua_State, idx: c_int) -> WindowRef {
    match window_at(l, idx) {
        Some(window) => window,
        None => {
            luaL_error(l, c"attempt to use a closed window".as_ptr());
            unreachable!()
        }
    }
}

unsafe fn check_window(l: *mut lua_State) -> WindowRef {
    check_window_at(l, 1)
}

unsafe fn push_window(l: *mut lua_State, window: WindowRef) {
    let slot = lua_newuserdata(l, std::mem::size_of::<Slot>()) as *mut Slot;
    slot.write(std::ptr::null());
    luaL_setmetatable(l, RENWINDOW_TYPE.as_ptr());
    slot.write(Rc::into_raw(window));
}

unsafe extern "C-unwind" fn f_renwin_create(l: *mut lua_State) -> c_int {
    // luaL_checkstring is first-NUL: the native title takes the same bytes as SDL did.
    let title = luaL_checkstring(l, 1);
    let width = luaL_optnumber(l, 2, 0.0) as f32;
    let height = luaL_optnumber(l, 3, 0.0) as f32;
    let title = CStr::from_ptr(title).to_string_lossy();

    let created = native::create(&title, width, height);
    drop(title);
    match created {
        Ok(id) => {
            push_window(l, crate::renwindow::create(id));
            1
        }
        Err(message) => {
            // luaL_error longjmps, which would skip `message`'s destructor: park the text on
            // the Lua stack first, then drop the Rust allocation before throwing.
            let (ptr, len) = (message.as_ptr() as *const c_char, message.len());
            lua_pushlstring(l, ptr, len);
            drop(message);
            luaL_error(
                l,
                c"Error creating lite-xl window: %s".as_ptr(),
                lua_tostring(l, -1),
            )
        }
    }
}

unsafe extern "C-unwind" fn f_renwin_gc(l: *mut lua_State) -> c_int {
    let slot = luaL_checkudata(l, 1, RENWINDOW_TYPE.as_ptr()) as *mut Slot;
    let window = slot.replace(std::ptr::null());
    if !window.is_null() {
        // SAFETY: the slot's own strong count, taken exactly once.
        drop(Rc::from_raw(window));
    }
    0
}

unsafe extern "C-unwind" fn f_renwin_get_size(l: *mut lua_State) -> c_int {
    let window = check_window(l);
    let (w, h) = window.borrow_mut().size();
    drop(window);
    lua_pushnumber(l, w as f64);
    lua_pushnumber(l, h as f64);
    2
}

unsafe extern "C-unwind" fn f_renwin_persist(l: *mut lua_State) -> c_int {
    let window = check_window(l);
    // The previous persistent window, if any, is released outside the borrow.
    let _previous = PERSISTENT_WINDOW.with(|slot| slot.replace(Some(window)));
    0
}

unsafe extern "C-unwind" fn f_renwin_restore(l: *mut lua_State) -> c_int {
    match PERSISTENT_WINDOW.with(|slot| slot.borrow().clone()) {
        None => lua_pushnil(l),
        Some(window) => push_window(l, window),
    }
    1
}

/// `luaopen_renwindow`.
///
/// # Safety
/// `l` is a live Lua state.
pub unsafe extern "C-unwind" fn luaopen_renwindow(l: *mut lua_State) -> c_int {
    let lib: [(&CStr, lua_CFunction); 5] = [
        (c"create", f_renwin_create),
        (c"__gc", f_renwin_gc),
        (c"get_size", f_renwin_get_size),
        (c"_persist", f_renwin_persist),
        (c"_restore", f_renwin_restore),
    ];
    luaL_newmetatable(l, RENWINDOW_TYPE.as_ptr());
    for (name, func) in lib {
        mlua::ffi::lua_pushcfunction(l, func);
        lua_setfield(l, -2, name.as_ptr());
    }
    lua_pushvalue(l, -1);
    lua_setfield(l, -2, c"__index".as_ptr());
    1
}
