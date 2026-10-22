//! Converts winit window events into the owned [`Input`] records `poll_event` reads.
//!
//! `native::InputState` owns the small amount of per-window state SDL used to keep
//! (last pointer, text-input request, IME preedit and touch positions) plus the global
//! per-device click counters. Native window lifecycle, resizing, redraw, hit testing and
//! the implicit left-button drag stay in `native.rs`.

use std::collections::{BTreeMap, HashMap};
use std::os::unix::ffi::OsStrExt;
use std::time::Instant;

use winit::dpi::PhysicalPosition;
use winit::event::{
    DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::Window;
use winit::keyboard::ModifiersState;

use crate::clicks::Clicks;
use crate::keymap::key_event_codes;
use crate::platform::{self, FingerKind, Input};

/// SDL's Wayland `WAYLAND_WHEEL_AXIS_UNIT`: continuous axis pixels per scroll line.
const WHEEL_AXIS_UNIT: f32 = 10.0;

/// Window-local state that SDL kept in its `SDL_Window`/`SDL_Mouse` structs.
#[derive(Default)]
struct WindowState {
    pointer: (f32, f32),
    has_pointer: bool,
    text_enabled: bool,
    modifiers: ModifiersState,
    ime_enabled: bool,
    preedit_active: bool,
    /// Last finger position, in normalized window fractions, keyed by winit's finger id.
    touch_prev: HashMap<u64, (f32, f32)>,
}

/// Keys Lua has seen pressed and not released, by scancode, with the key code and NumLock
/// state of the press so the release names the same key.
///
/// SDL resets the keyboard when its window loses keyboard focus (`SDL_ResetKeyboard`): every
/// held key gets a release, in scancode order, before the focus-lost event. Winit's X11 backend
/// sends those releases itself; its Wayland backend only clears the modifiers, which left Lua
/// holding Ctrl after a Ctrl+O dialog took focus. Scancode 0 is untracked, as in SDL.
#[derive(Default)]
struct HeldKeys(BTreeMap<i32, (u32, bool)>);

impl HeldKeys {
    fn note(&mut self, scancode: i32, key: u32, numlock: bool, down: bool) {
        if scancode == 0 {
            return;
        }
        if down {
            self.0.insert(scancode, (key, numlock));
        } else {
            self.0.remove(&scancode);
        }
    }

    fn release_all(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.0)
            .into_iter()
            .map(|(scancode, (key, numlock))| Input::Key { down: false, scancode, key, numlock })
            .collect()
    }
}

/// The winit input producer. One instance lives in `native.rs` for the process lifetime.
pub struct InputState {
    start: Instant,
    clicks: Clicks,
    sources: HashMap<DeviceId, u32>,
    next_source: u32,
    windows: HashMap<u32, WindowState>,
    held: HeldKeys,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            clicks: Clicks::default(),
            sources: HashMap::new(),
            next_source: 0,
            windows: HashMap::new(),
            held: HeldKeys::default(),
        }
    }

    /// Last cursor position for `id`, in window logical points.
    pub fn pointer(&self, id: u32) -> (f32, f32) {
        self.windows.get(&id).map_or((0.0, 0.0), |window| window.pointer)
    }

    /// SDL_StartTextInput/SDL_StopTextInput for one window.
    pub fn set_text_input(&mut self, id: u32, enabled: bool) {
        let window = self.windows.entry(id).or_default();
        window.text_enabled = enabled;
        if !enabled {
            window.preedit_active = false;
        }
    }

    /// SDL_ClearComposition: drop the current preedit and tell Lua it ended.
    pub fn clear_ime(&mut self, id: u32) -> bool {
        let Some(window) = self.windows.get_mut(&id) else {
            return false;
        };
        let had_preedit = window.preedit_active;
        window.preedit_active = false;
        if had_preedit && window.text_enabled {
            push_editing("");
        }
        window.text_enabled
    }

    /// Forget everything about a window that has been destroyed.
    pub fn remove(&mut self, id: u32) {
        self.windows.remove(&id);
    }

    /// Converts one winit window event. Events not translated here belong to `native.rs`
    /// (resize, scale, redraw, occlusion, close, window mode and hit-test drag).
    pub fn event(&mut self, id: u32, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.windows.entry(id).or_default().modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key_without_modifiers = event.key_without_modifiers();
                let (scancode, key, numlock) = key_event_codes(
                    event.physical_key,
                    event.location,
                    &key_without_modifiers,
                    &event.logical_key,
                );
                let down = event.state == ElementState::Pressed;
                self.held.note(scancode, key, numlock, down);
                platform::push(Input::Key { down, scancode, key, numlock });
                if down {
                    if let Some(text) = event.text.as_deref() {
                        self.text_input(id, text, false);
                    }
                }
            }
            WindowEvent::Ime(ime) => self.ime(id, ime),
            WindowEvent::CursorMoved { position, .. } => {
                let scale = window.scale_factor() as f32;
                let (x, y) = logical_point(*position, scale);
                let state = self.windows.entry(id).or_default();
                let had_pointer = state.has_pointer;
                let (xrel, yrel) = if had_pointer {
                    (x - state.pointer.0, y - state.pointer.1)
                } else {
                    (0.0, 0.0)
                };
                state.pointer = (x, y);
                state.has_pointer = true;
                // SDL drops a motion event that did not move the cursor.
                if had_pointer && xrel == 0.0 && yrel == 0.0 {
                    return;
                }
                platform::push(Input::MouseMotion { window: id, x, y, xrel, yrel });
            }
            WindowEvent::CursorLeft { .. } => platform::push(Input::MouseLeft),
            WindowEvent::MouseInput { device_id, state, button } => {
                let down = *state == ElementState::Pressed;
                let button = mouse_button(*button);
                let source = self.source(*device_id);
                let (x, y) = self.windows.entry(id).or_default().pointer;
                let clicks = if down {
                    let now_ms = self.start.elapsed().as_millis().min(u64::MAX as u128) as u64;
                    self.clicks.press(now_ms, source, button, x, y)
                } else {
                    self.clicks.release(source, button)
                };
                platform::push(Input::MouseButton { window: id, down, button, x, y, clicks });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = wheel_delta(delta);
                platform::push(Input::MouseWheel { x, y });
            }
            WindowEvent::DroppedFile(path) => {
                platform::push(Input::FileDropped {
                    window: id,
                    path: path.as_os_str().as_bytes().to_vec(),
                });
            }
            WindowEvent::Touch(touch) => {
                let (x, y) = touch_fraction(touch.location, window.inner_size());
                let state = self.windows.entry(id).or_default();
                let (kind, dx, dy) = match touch.phase {
                    winit::event::TouchPhase::Started => {
                        state.touch_prev.insert(touch.id, (x, y));
                        (FingerKind::Down, 0.0, 0.0)
                    }
                    winit::event::TouchPhase::Moved => {
                        let previous = state.touch_prev.insert(touch.id, (x, y));
                        previous.map_or((FingerKind::Motion, 0.0, 0.0), |(px, py)| {
                            (FingerKind::Motion, x - px, y - py)
                        })
                    }
                    winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                        state.touch_prev.remove(&touch.id);
                        (FingerKind::Up, 0.0, 0.0)
                    }
                };
                platform::push(Input::Finger { window: id, kind, x, y, dx, dy, finger: touch.id });
            }
            WindowEvent::Focused(focused) => {
                if !*focused {
                    for input in self.held.release_all() {
                        platform::push(input);
                    }
                }
                platform::push(if *focused { Input::FocusGained } else { Input::FocusLost });
            }
            _ => {}
        }
    }

    fn ime(&mut self, id: u32, ime: &Ime) {
        match ime {
            Ime::Enabled => self.windows.entry(id).or_default().ime_enabled = true,
            Ime::Disabled => {
                let window = self.windows.entry(id).or_default();
                let had_preedit = window.preedit_active;
                window.ime_enabled = false;
                window.preedit_active = false;
                if had_preedit && window.text_enabled {
                    push_editing("");
                }
            }
            Ime::Preedit(text, cursor) => {
                let (start, length) = preedit_cursor(text, *cursor);
                let window = self.windows.entry(id).or_default();
                window.preedit_active = !text.is_empty();
                if window.text_enabled {
                    platform::push(Input::TextEditing {
                        text: text.as_bytes().to_vec(),
                        start,
                        length,
                    });
                }
            }
            Ime::Commit(text) => {
                self.text_input(id, text, true);
                self.windows.entry(id).or_default().preedit_active = false;
            }
        }
    }

    // Winit's X11 compose path emits either KeyboardInput or Ime::Commit, not both.
    fn text_input(&mut self, id: u32, text: &str, from_ime: bool) {
        let Some(text) = printable(text) else { return; };
        let window = self.windows.entry(id).or_default();
        // SDL suppresses ordinary Ctrl/Alt text; AltGraph uses the separate XKB level modifier.
        let shortcut = window.modifiers.control_key() || window.modifiers.alt_key();
        if window.text_enabled && (from_ime || (!window.preedit_active && !shortcut)) {
            platform::push(Input::TextInput(text.as_bytes().to_vec()));
        }
    }

    fn source(&mut self, device: DeviceId) -> u32 {
        if let Some(&source) = self.sources.get(&device) {
            return source;
        }
        let source = self.next_source;
        self.next_source = self.next_source.wrapping_add(1);
        self.sources.insert(device, source);
        source
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

fn push_editing(text: &str) {
    platform::push(Input::TextEditing { text: text.as_bytes().to_vec(), start: 0, length: 0 });
}

/// Physical pixels to window logical points.
fn logical_point(position: PhysicalPosition<f64>, scale: f32) -> (f32, f32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    (position.x as f32 / scale, position.y as f32 / scale)
}

/// SDL wayland reports touch coordinates as a fraction of the physical window size.
fn touch_fraction(position: PhysicalPosition<f64>, size: winit::dpi::PhysicalSize<u32>) -> (f32, f32) {
    let x = if size.width == 0 { 0.0 } else { position.x as f32 / size.width as f32 };
    let y = if size.height == 0 { 0.0 } else { position.y as f32 / size.height as f32 };
    (x, y)
}

/// SDL text input rejects a string whose first byte is an ASCII control character
/// (`SDL_iscntrl`); AltGr text (for example `€`) is not filtered by modifiers.
fn printable(text: &str) -> Option<&str> {
    let first = *text.as_bytes().first()?;
    if first < 0x20 || first == 0x7f {
        None
    } else {
        Some(text)
    }
}

/// Winit cursor offsets are bytes; SDL's editing event wants character counts and -1 when
/// there is no selection. Round down to a char boundary and clamp beyond the end.
fn preedit_cursor(text: &str, cursor: Option<(usize, usize)>) -> (i32, i32) {
    if text.is_empty() {
        return (0, 0);
    }
    let Some((begin, end)) = cursor else {
        return (-1, -1);
    };
    let start = char_count(text, begin);
    let finish = char_count(text, end).max(start);
    (start as i32, (finish - start) as i32)
}

fn char_count(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    text[..byte].chars().count()
}

/// SDL button numbers: left 1, middle 2, right 3, X1 4, X2 5. Winit's `Other` is the
/// backend's raw value (X11 button, or Linux evdev code on Wayland), so match SDL's
/// conversions for both; SDL maps every larger button to `?` in Lua anyway.
fn mouse_button(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Middle => 2,
        MouseButton::Right => 3,
        MouseButton::Back => 4,
        MouseButton::Forward => 5,
        MouseButton::Other(raw) if raw >= 0x110 => (raw - 0x10f).min(255) as u8,
        MouseButton::Other(raw) => raw.saturating_sub(4).min(255) as u8,
    }
}

/// Line deltas pass through. Pixel deltas come from winit's Wayland continuous axis;
/// SDL divides those pixels by 10 (SDL_waylandevents.c `WAYLAND_WHEEL_AXIS_UNIT`).
fn wheel_delta(delta: &MouseScrollDelta) -> (f32, f32) {
    match *delta {
        MouseScrollDelta::LineDelta(x, y) => (x, y),
        MouseScrollDelta::PixelDelta(position) => {
            (position.x as f32 / WHEEL_AXIS_UNIT, position.y as f32 / WHEEL_AXIS_UNIT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_loss_releases_held_keys_in_scancode_order() {
        let mut held = HeldKeys::default();
        held.note(224, 0x4000_00E0, false, true); // left ctrl
        held.note(18, 0x6F, false, true); // o
        held.note(18, 0x6F, false, true); // a repeat is still one held key
        held.note(89, 0x4000_0059, true, true); // keypad 1 under NumLock
        held.note(0, 0, false, true); // unknown scancodes are not tracked
        held.note(18, 0x6F, false, false);
        let up = |scancode, key, numlock| Input::Key { down: false, scancode, key, numlock };
        assert_eq!(held.release_all(), vec![up(89, 0x4000_0059, true), up(224, 0x4000_00E0, false)]);
        assert!(held.release_all().is_empty(), "released keys are forgotten");
    }

    #[test]
    fn preedit_offsets_are_character_counts() {
        // "aé日": byte offsets 0, 1, 3, 6.
        assert_eq!(preedit_cursor("aé日", Some((0, 0))), (0, 0));
        assert_eq!(preedit_cursor("aé日", Some((1, 3))), (1, 1));
        assert_eq!(preedit_cursor("aé日", Some((3, 6))), (2, 1));
        assert_eq!(preedit_cursor("aé日", Some((1, 6))), (1, 2));
        // Out of range and mid-codepoint offsets round down, never panic or split UTF-8.
        assert_eq!(preedit_cursor("aé日", Some((2, 99))), (1, 2));
        assert_eq!(preedit_cursor("aé日", Some((99, 99))), (3, 0));
        assert_eq!(preedit_cursor("aé日", Some((6, 3))), (3, 0));
        assert_eq!(preedit_cursor("aé日", None), (-1, -1));
        assert_eq!(preedit_cursor("", None), (0, 0));
    }

    #[test]
    fn text_input_filters_control_keys_but_not_altgr() {
        assert_eq!(printable("a"), Some("a"));
        assert_eq!(printable("é"), Some("é"));
        // AltGr produces printable text even though SDL reports Ctrl+Alt modifiers.
        assert_eq!(printable("€"), Some("€"));
        assert_eq!(printable(""), None);
        assert_eq!(printable("\r"), None);
        assert_eq!(printable("\t"), None);
        assert_eq!(printable("\u{7f}"), None);
        // SDL only checks the first byte; an embedded tab still travels in one text event.
        assert_eq!(printable("a\tb"), Some("a\tb"));
    }

    #[test]
    fn coordinate_helpers_convert_to_window_units() {
        let position = PhysicalPosition::new(100.0, 50.0);
        assert_eq!(logical_point(position, 2.0), (50.0, 25.0));
        assert_eq!(logical_point(position, 0.0), (100.0, 50.0));
        assert_eq!(touch_fraction(position, winit::dpi::PhysicalSize::new(200, 100)), (0.5, 0.5));
        assert_eq!(touch_fraction(position, winit::dpi::PhysicalSize::new(0, 0)), (0.0, 0.0));
    }

    #[test]
    fn mouse_buttons_match_sdl_numbers() {
        assert_eq!(mouse_button(MouseButton::Left), 1);
        assert_eq!(mouse_button(MouseButton::Middle), 2);
        assert_eq!(mouse_button(MouseButton::Right), 3);
        assert_eq!(mouse_button(MouseButton::Back), 4);
        assert_eq!(mouse_button(MouseButton::Forward), 5);
        // X11 raw button 10 is SDL 6; Wayland evdev 0x115 is SDL 6.
        assert_eq!(mouse_button(MouseButton::Other(10)), 6);
        assert_eq!(mouse_button(MouseButton::Other(0x115)), 6);
    }

    #[test]
    fn wheel_deltas_follow_sdl_conventions() {
        assert_eq!(wheel_delta(&MouseScrollDelta::LineDelta(1.5, -2.0)), (1.5, -2.0));
        assert_eq!(
            wheel_delta(&MouseScrollDelta::PixelDelta(PhysicalPosition::new(20.0, -10.0))),
            (2.0, -1.0)
        );
    }
}
