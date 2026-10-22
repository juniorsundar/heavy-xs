//! The owned-input queue `poll_event`/`wait_event` read.
//!
//! Every input reaches Lua through here as an owned Rust [`Input`] record that points into no
//! producer's memory, so no allocator is shared and no payload outlives its producer's rules.
//!
//! shortcut: SDL is still the native producer (`sysevents::next_input` converts one event per
//! poll) and carries custom events as plain integer tokens, which keeps today's ordering and
//! cross-thread wake-ups; the winit cutover replaces both with a native pump and proxy wake.

use std::collections::VecDeque;
use std::sync::Mutex;

/// A custom event's owned payload: a finished dialog's paths (native bytes) or error message.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    Paths(Vec<Vec<u8>>),
    Message(Vec<u8>),
}

/// A custom event: its registered type, SDL's `user.code`/`data1`, and an owned payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    pub event_type: u32,
    pub code: i32,
    pub data1: usize,
    pub payload: Option<Payload>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FingerKind {
    Down,
    Up,
    Motion,
}

/// One owned input record. Window ids are the producer's; coordinates are in window points
/// (as SDL reports them) and are scaled by the consumer with the window's current scale.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Quit,
    Resized { window: u32, w: i32, h: i32 },
    /// Pixel size or display scale changed: the window rescales, Lua sees nothing.
    Rescaled { window: u32 },
    Exposed,
    Minimized,
    Maximized,
    Restored,
    MouseLeft,
    FocusLost,
    FocusGained,
    FileDropped { window: u32, path: Vec<u8> },
    Key { down: bool, scancode: i32, key: u32, numlock: bool },
    TextInput(Vec<u8>),
    TextEditing { text: Vec<u8>, start: i32, length: i32 },
    MouseButton { window: u32, down: bool, button: u8, x: f32, y: f32, clicks: u8 },
    MouseMotion { window: u32, x: f32, y: f32, xrel: f32, yrel: f32 },
    MouseWheel { x: f32, y: f32 },
    Finger { window: u32, kind: FingerKind, x: f32, y: f32, dx: f32, dy: f32, finger: u64 },
    EnteringForeground,
    EnteredForeground,
    EnteringBackground,
    EnteredBackground,
    Custom(Custom),
    /// Anything else the producer saw: no Lua event.
    Other,
}

static QUEUE: Mutex<VecDeque<Input>> = Mutex::new(VecDeque::new());

/// Appends a record (main thread: producers convert at the pump).
pub fn push(input: Input) {
    QUEUE.lock().unwrap().push_back(input);
}

pub fn pop() -> Option<Input> {
    QUEUE.lock().unwrap().pop_front()
}

pub fn is_empty() -> bool {
    QUEUE.lock().unwrap().is_empty()
}

/// `SDL_FlushEvent`: removes every queued record `f` matches, keeping the rest in order.
pub fn remove_all(f: impl Fn(&Input) -> bool) {
    QUEUE.lock().unwrap().retain(|input| !f(input));
}

/// The SDL_PeepEvents coalescing loop: removes every queued motion record of `first`'s kind,
/// wherever it sits, folding each in order into `first` (last position, summed deltas; the
/// first record's window and finger id are kept, as SDL kept its event).
pub fn coalesce(first: &mut Input) {
    let mut queue = QUEUE.lock().unwrap();
    queue.retain(|next| {
        match (&mut *first, next) {
            (
                Input::MouseMotion { x, y, xrel, yrel, .. },
                Input::MouseMotion { x: x2, y: y2, xrel: dx, yrel: dy, .. },
            ) => {
                (*x, *y) = (*x2, *y2);
                *xrel += *dx;
                *yrel += *dy;
            }
            (
                Input::Finger { kind: FingerKind::Motion, x, y, dx, dy, .. },
                Input::Finger { kind: FingerKind::Motion, x: x2, y: y2, dx: ddx, dy: ddy, .. },
            ) => {
                (*x, *y) = (*x2, *y2);
                *dx += *ddx;
                *dy += *ddy;
            }
            _ => return true,
        }
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(x: f32, xrel: f32) -> Input {
        Input::MouseMotion { window: 1, x, y: x, xrel, yrel: -xrel }
    }

    #[test]
    fn queue_keeps_order_flushes_and_coalesces() {
        // The queue is process-global; this is the only test that touches it.
        while pop().is_some() {}
        push(Input::Key { down: true, scancode: 43, key: 9, numlock: false });
        push(Input::TextInput(b"a".to_vec()));
        push(Input::Key { down: false, scancode: 43, key: 9, numlock: false });
        remove_all(|i| matches!(i, Input::Key { down: true, .. }));
        assert_eq!(pop(), Some(Input::TextInput(b"a".to_vec())));
        assert!(matches!(pop(), Some(Input::Key { down: false, .. })));

        // Coalescing reaches past other records, as SDL_PeepEvents did.
        push(motion(20.0, 2.0));
        push(Input::Quit);
        push(motion(30.0, 3.0));
        let mut first = motion(10.0, 1.0);
        coalesce(&mut first);
        assert_eq!(first, Input::MouseMotion { window: 1, x: 30.0, y: 30.0, xrel: 6.0, yrel: -6.0 });
        assert_eq!(pop(), Some(Input::Quit));
        assert!(is_empty());
    }
}
