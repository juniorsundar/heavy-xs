//! Owned input, process quit requests and cross-thread wake state. No producer calls Lua.
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use winit::event_loop::EventLoopProxy;

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
#[allow(dead_code)] // Historical lifecycle records remain injectable; Linux winit has no mobile lifecycle.
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


struct Queued { input: Input, process_quit: bool }
struct State {
    queue: VecDeque<Queued>,
    pending_quit: bool,
    quit_queued: bool,
    error: Option<String>,
    proxy: Option<EventLoopProxy<crate::native::Wake>>,
    generation: u64,
    closed: bool,
}
static STATE: Mutex<State> = Mutex::new(State {
    queue: VecDeque::new(), pending_quit: false, quit_queued: false,
    error: None, proxy: None, generation: 0, closed: false,
});
static NOTIFY: Notify = Notify::const_new();

fn wake() {
    NOTIFY.notify_one();
    let proxy = STATE.lock().unwrap().proxy.clone();
    if let Some(proxy) = proxy { let _ = proxy.send_event(crate::native::Wake::Input); }
}

#[cfg(test)]
pub(crate) static PUBLISH_GATE: Mutex<Option<(bool, std::sync::Arc<std::sync::Barrier>)>> = Mutex::new(None);

#[cfg(test)]
fn pause_publication(after_store: bool) {
    let gate = PUBLISH_GATE.lock().unwrap().as_ref()
        .filter(|(after, _)| *after == after_store).map(|(_, gate)| gate.clone());
    if let Some(gate) = gate { gate.wait(); gate.wait(); }
}

pub fn publish_proxy(proxy: EventLoopProxy<crate::native::Wake>) {
    #[cfg(test)]
    pause_publication(false);
    {
        let mut state = STATE.lock().unwrap();
        if state.closed { return; }
        state.proxy = Some(proxy);
    }
    #[cfg(test)]
    pause_publication(true);
    // Recheck after publishing: a producer may have arrived before there was a proxy.
    if ready() { wake(); }
}

pub fn push(input: Input) { let _ = post(input); }

pub fn post(input: Input) -> bool {
    post_inner(None, input)
}

pub fn generation() -> u64 { STATE.lock().unwrap().generation }

pub fn accepts(generation: u64) -> bool {
    let state = STATE.lock().unwrap();
    !state.closed && state.generation == generation
}

pub fn post_for_generation(generation: u64, input: Input) -> bool {
    post_inner(Some(generation), input)
}

fn post_inner(generation: Option<u64>, input: Input) -> bool {
    let notify = {
        let mut state = STATE.lock().unwrap();
        if state.closed || generation.is_some_and(|g| g != state.generation) { return false; }
        let notify = state.queue.is_empty();
        state.queue.push_back(Queued { input, process_quit: false });
        notify
    };
    if notify { wake(); }
    true
}

pub fn request_quit() {
    {
        let mut state = STATE.lock().unwrap();
        if state.closed || state.pending_quit { return; }
        state.pending_quit = true;
    }
    wake();
}

pub fn report_error(error: String) {
    {
        let mut state = STATE.lock().unwrap();
        if state.closed { return; }
        state.error.get_or_insert(error);
    }
    wake();
}

pub fn take_error() -> Option<String> { STATE.lock().unwrap().error.take() }

fn admit_quit(state: &mut State) {
    if state.pending_quit && !state.quit_queued {
        state.queue.push_back(Queued { input: Input::Quit, process_quit: true });
        state.quit_queued = true;
    }
}

pub fn pop() -> Option<Input> {
    let mut state = STATE.lock().unwrap();
    if state.closed { return None; }
    admit_quit(&mut state);
    let queued = state.queue.pop_front()?;
    if queued.process_quit {
        state.pending_quit = false;
        state.quit_queued = false;
    }
    Some(queued.input)
}

#[cfg(test)]
pub fn is_empty() -> bool {
    let state = STATE.lock().unwrap();
    state.queue.is_empty() && !state.pending_quit
}

pub fn ready() -> bool {
    let state = STATE.lock().unwrap();
    !state.closed && (!state.queue.is_empty() || state.pending_quit || state.error.is_some())
}

pub fn remove_all(f: impl Fn(&Input) -> bool) {
    STATE.lock().unwrap().queue.retain(|q| q.process_quit || !f(&q.input));
}

/// Retire Lua-owned inputs before destroying its state; process quit survives.
pub fn retire_generation() {
    let mut state = STATE.lock().unwrap();
    state.generation += 1;
    state.queue.retain(|q| q.process_quit);
}

pub fn close() {
    let mut state = STATE.lock().unwrap();
    state.closed = true;
    state.proxy = None;
    state.queue.clear();
    state.pending_quit = false;
    state.quit_queued = false;
    state.error = None;
}

pub fn coalesce(first: &mut Input) {
    let mut state = STATE.lock().unwrap();
    state.queue.retain(|next| {
        match (&mut *first, &next.input) {
            (
                Input::MouseMotion { x, y, xrel, yrel, .. },
                Input::MouseMotion { x: x2, y: y2, xrel: dx, yrel: dy, .. },
            ) => {
                (*x, *y) = (*x2, *y2);
                *xrel += *dx; *yrel += *dy;
            }
            (
                Input::Finger { kind: FingerKind::Motion, x, y, dx, dy, .. },
                Input::Finger { kind: FingerKind::Motion, x: x2, y: y2, dx: ddx, dy: ddy, .. },
            ) => {
                (*x, *y) = (*x2, *y2);
                *dx += *ddx; *dy += *ddy;
            }
            _ => return true,
        }
        false
    });
}

/// A display-free wait uses the same runtime as signals; registration precedes rechecking.
fn wait_before_loop(timeout: Option<Duration>) -> Result<bool, String> {
    let runtime = crate::services::runtime()?;
    Ok(runtime.block_on(async {
        let deadline = timeout.map(|d| tokio::time::Instant::now() + d);
        loop {
            let notified = NOTIFY.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if ready() { return true; }
            match deadline {
                Some(end) => if tokio::time::timeout_at(end, notified).await.is_err() { return ready(); },
                None => notified.await,
            }
        }
    }))
}

/// Non-consuming readiness with one deadline across backend-internal/spurious wakes.
pub fn wait(timeout: Option<Duration>) -> Result<bool, String> {
    if ready() { return Ok(true); }
    if !crate::native::initialized() { return wait_before_loop(timeout); }
    let deadline = timeout.map(|d| Instant::now() + d);
    loop {
        if ready() { return Ok(true); }
        let remaining = deadline.map(|end| end.saturating_duration_since(Instant::now()));
        crate::native::pump(remaining)?;
        if ready() { return Ok(true); }
        if deadline.is_some_and(|end| Instant::now() >= end) { return Ok(false); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(x: f32, xrel: f32) -> Input {
        Input::MouseMotion { window: 1, x, y: x, xrel, yrel: -xrel }
    }

    #[test]
    fn queue_keeps_order_flushes_and_coalesces() {
        // The queue is process-global; other stateful subjects run in separate processes.
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

        // A process request stays pending until consumption, across generation retirement.
        let old = generation();
        push(Input::TextInput(b"old".to_vec()));
        request_quit();
        request_quit();
        assert!(ready());
        retire_generation();
        assert!(!post_for_generation(old, Input::Other));
        assert!(ready());
        assert_eq!(pop(), Some(Input::Quit));
        assert_eq!(pop(), None);

        // Admit at the tail; neither a readiness check nor queue filtering consumes the quit.
        push(Input::TextInput(b"first".to_vec()));
        request_quit();
        assert_eq!(pop(), Some(Input::TextInput(b"first".to_vec())));
        remove_all(|_| true);
        assert!(ready());
        assert_eq!(pop(), Some(Input::Quit));
        assert!(!ready());

        // A pre-loop notifier wakes the main thread without creating a display.
        let worker = std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(20));
            report_error("test driver failure".into());
        });
        assert!(wait(Some(Duration::from_secs(2))).unwrap());
        worker.join().unwrap();
        assert_eq!(take_error().as_deref(), Some("test driver failure"));
        assert!(!ready());

        // Stale/internal notifications cannot invent readiness or extend the one deadline.
        let worker = std::thread::spawn(|| {
            for _ in 0..10 {
                wake();
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let start = Instant::now();
        assert!(!wait(Some(Duration::from_millis(30))).unwrap());
        assert!(start.elapsed() >= Duration::from_millis(20));
        assert!(start.elapsed() < Duration::from_secs(1));
        worker.join().unwrap();

        request_quit();
        report_error("late failure".into());
        close();
        assert!(!ready());
        assert!(pop().is_none());
        assert!(!post(Input::Other));
        assert!(!post_for_generation(generation(), Input::Other));
        assert!(!accepts(generation()));
        assert!(take_error().is_none());
    }
}
