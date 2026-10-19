//! Clipboard, wake and file-dialog probes for the loop prototype.
//!
//! Clipboard results never expose clipboard text: summaries carry only byte
//! counts and a marker match. Dialog results never expose paths or URIs.
//! Clipboard writes use fixed public probe markers, never private content.

use arboard::{Clipboard, GetExtLinux, LinuxClipboardKind, SetExtLinux};
use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use ashpd::desktop::{Request, ResponseError};
use ashpd::WindowIdentifier;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

/// Fixed public clipboard probe markers; not private content.
const CLIPBOARD_MARKER: &str = "lite-xl clipboard probe";
const PRIMARY_MARKER: &str = "lite-xl primary probe";
const DIALOG_TITLE: &str = "lite-xl dialog probe";
const SAVE_NAME: &str = "probe-output.txt";
const WAKE_DELAY: Duration = Duration::from_millis(150);
static RFD_FALLBACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct RfdDiagnostics;
impl log::Log for RfdDiagnostics {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.target().starts_with("rfd") && metadata.level() <= log::Level::Warn
    }
    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if record.args().as_str() == Some("Using zenity fallback") {
            RFD_FALLBACK.store(true, std::sync::atomic::Ordering::Relaxed);
            println!("probe: rfd FALLBACK zenity (not portal evidence)");
        } else {
            println!("probe: rfd backend warning/error (private details suppressed)");
        }
    }
    fn flush(&self) {}
}
pub fn init_diagnostics() {
    log::set_logger(&RfdDiagnostics).expect("probe owns its logger");
    log::set_max_level(log::LevelFilter::Warn);
}

/// Events posted back to the event loop through [`EventLoopProxy`].
#[derive(Debug)]
pub enum Event {
    /// Wake completion; `simulated` marks a probe-generated wake.
    Wake { sent: Instant, simulated: bool },
    /// Clipboard or dialog result. `summary` never contains clipboard text or paths.
    Result {
        sent: Instant,
        summary: String,
        dialog: bool,
    },
}

/// Clipboard operation requested from the worker thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipAction {
    SetClipboard,
    GetClipboard,
    SetPrimary,
    GetPrimary,
}

impl ClipAction {
    fn is_set(self) -> bool {
        matches!(self, ClipAction::SetClipboard | ClipAction::SetPrimary)
    }

    fn kind(self) -> LinuxClipboardKind {
        match self {
            ClipAction::SetClipboard | ClipAction::GetClipboard => LinuxClipboardKind::Clipboard,
            ClipAction::SetPrimary | ClipAction::GetPrimary => LinuxClipboardKind::Primary,
        }
    }

    fn marker(self) -> &'static str {
        match self {
            ClipAction::SetClipboard | ClipAction::GetClipboard => CLIPBOARD_MARKER,
            ClipAction::SetPrimary | ClipAction::GetPrimary => PRIMARY_MARKER,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ClipAction::SetClipboard => "set-clipboard",
            ClipAction::GetClipboard => "get-clipboard",
            ClipAction::SetPrimary => "set-primary",
            ClipAction::GetPrimary => "get-primary",
        }
    }
}

/// Lazy clipboard worker.
///
/// The worker thread starts without touching the clipboard. The single
/// [`Clipboard`] is created on the first action and kept until the channel
/// closes, so ownership survives short operations.
pub struct ClipboardWorker {
    tx: Sender<(Instant, ClipAction)>,
}

impl ClipboardWorker {
    /// Start the worker thread. No clipboard connection is opened here.
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || clipboard_worker(proxy, rx));
        Self { tx }
    }

    /// Queue a clipboard action. Fails only when the worker has stopped.
    pub fn send(&self, action: ClipAction) -> Result<(), String> {
        self.tx
            .send((Instant::now(), action))
            .map_err(|_| "clipboard worker stopped".to_string())
    }
}

fn clipboard_worker(proxy: EventLoopProxy<Event>, rx: Receiver<(Instant, ClipAction)>) {
    let mut clipboard: Option<Clipboard> = None;
    while let Ok((sent, action)) = rx.recv() {
        let summary = run_clip_action(&mut clipboard, action);
        proxy
            .send_event(Event::Result {
                sent,
                summary,
                dialog: false,
            })
            .ok();
    }
    // Sender gone: drop the clipboard so ownership is released cleanly.
    drop(clipboard);
}

fn run_clip_action(clipboard: &mut Option<Clipboard>, action: ClipAction) -> String {
    if clipboard.is_none() {
        match Clipboard::new() {
            Ok(created) => *clipboard = Some(created),
            Err(error) => return error_summary(action, arboard_error_category(&error)),
        }
    }
    let clipboard = clipboard.as_mut().expect("clipboard initialized above");
    if action.is_set() {
        let marker = action.marker();
        match clipboard.set().clipboard(action.kind()).text(marker) {
            Ok(()) => set_summary(action, marker.len()),
            Err(error) => error_summary(action, arboard_error_category(&error)),
        }
    } else {
        match clipboard.get().clipboard(action.kind()).text() {
            Ok(text) => get_summary(action, &text, action.marker()),
            Err(error) => error_summary(action, arboard_error_category(&error)),
        }
    }
}

fn set_summary(action: ClipAction, bytes: usize) -> String {
    format!("{} bytes={bytes}", action.label())
}

fn get_summary(action: ClipAction, observed: &str, marker: &str) -> String {
    format!(
        "{} bytes={} marker-match={}",
        action.label(),
        observed.len(),
        observed == marker
    )
}

fn error_summary(action: ClipAction, category: &str) -> String {
    format!("{} error={category}", action.label())
}

/// Error category without the raw message, which may contain clipboard data.
fn arboard_error_category(error: &arboard::Error) -> &'static str {
    match error {
        arboard::Error::ContentNotAvailable => "content-not-available",
        arboard::Error::ClipboardNotSupported => "clipboard-not-supported",
        arboard::Error::ClipboardOccupied => "clipboard-occupied",
        arboard::Error::ConversionFailure => "conversion-failure",
        _ => "unknown",
    }
}

/// Post a synthetic wake after a fixed delay. `simulated` stays on the event so
/// synthetic and real wakes remain distinguishable.
pub fn wake(proxy: EventLoopProxy<Event>, simulated: bool) {
    std::thread::spawn(move || {
        std::thread::sleep(WAKE_DELAY);
        let sent = Instant::now();
        proxy.send_event(Event::Wake { sent, simulated }).ok();
    });
}

/// Which dialog implementation to run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogApi {
    Rfd,
    Ashpd,
}

/// Requested dialog shape.
#[derive(Clone, Copy, Debug)]
pub enum DialogKind {
    Open,
    Save,
    Directory,
}

/// Run a dialog on its own thread and post the result through `proxy`.
///
/// The `Arc<Window>` is held for the whole worker lifetime so the parent handle
/// stays valid through the portal response.
pub fn dialog(proxy: EventLoopProxy<Event>, window: Arc<Window>, api: DialogApi, kind: DialogKind) {
    let sent = Instant::now();
    std::thread::spawn(move || {
        let summary = match api {
            DialogApi::Rfd => run_dialog_rfd(window.as_ref(), kind),
            DialogApi::Ashpd => run_dialog_ashpd(window.as_ref(), kind),
        };
        proxy
            .send_event(Event::Result {
                sent,
                summary,
                dialog: true,
            })
            .ok();
    });
}

fn run_dialog_rfd(window: &Window, kind: DialogKind) -> String {
    RFD_FALLBACK.store(false, std::sync::atomic::Ordering::Relaxed);
    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return "rfd runtime-unavailable".to_string(),
    };
    runtime.block_on(async move {
        let dialog = rfd::AsyncFileDialog::new()
            .set_parent(window)
            .set_title(DIALOG_TITLE)
            .set_file_name(SAVE_NAME)
            .add_filter("Rust", &["rs"])
            .add_filter("Text", &["txt"])
            .add_filter("All files", &["*"]);
        // rfd's Option only distinguishes "picked" from "not picked"; a cancel
        // and a backend failure look the same here. The handle is dropped
        // without reading or writing the file.
        let picked = match kind {
            DialogKind::Open => dialog.pick_file().await.is_some(),
            DialogKind::Save => dialog.save_file().await.is_some(),
            DialogKind::Directory => dialog.pick_folder().await.is_some(),
        };
        rfd_summary(
            picked,
            RFD_FALLBACK.load(std::sync::atomic::Ordering::Relaxed),
        )
    })
}

fn rfd_summary(picked: bool, fallback: bool) -> String {
    format!(
        "rfd {} fallback-zenity={fallback}",
        if picked {
            "selected count=1"
        } else {
            "no-selection(cancel-or-error)"
        }
    )
}

fn run_dialog_ashpd(window: &Window, kind: DialogKind) -> String {
    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return "ashpd runtime-unavailable".to_string(),
    };
    runtime.block_on(async move {
        let window_handle = match window.window_handle() {
            Ok(handle) => handle,
            Err(_) => return "ashpd parent-unavailable".to_string(),
        };
        let display_handle = match window.display_handle() {
            Ok(handle) => handle,
            Err(_) => return "ashpd parent-unavailable".to_string(),
        };
        let raw_window = window_handle.as_raw();
        let raw_display = display_handle.as_raw();
        // `parent` stays alive until the request is sent (ashpd's builder owns
        // it during `send`), which is also when the portal resolves the export.
        let parent = match WindowIdentifier::from_raw_handle(&raw_window, Some(&raw_display)).await
        {
            Some(parent) => parent,
            None => return "ashpd parent-unavailable".to_string(),
        };
        let filters = [
            FileFilter::new("Rust").glob("*.rs"),
            FileFilter::new("Text").glob("*.txt"),
            FileFilter::new("All files").glob("*"),
        ];
        let response = match kind {
            DialogKind::Open | DialogKind::Directory => {
                SelectedFiles::open_file()
                    .identifier(parent)
                    .title(DIALOG_TITLE)
                    .modal(true)
                    .directory(matches!(kind, DialogKind::Directory))
                    .filters(filters)
                    .send()
                    .await
            }
            DialogKind::Save => {
                SelectedFiles::save_file()
                    .identifier(parent)
                    .title(DIALOG_TITLE)
                    .modal(true)
                    .current_name(SAVE_NAME)
                    .filters(filters)
                    .send()
                    .await
            }
        };
        ashpd_result_summary(response)
    })
}

fn ashpd_result_summary(result: Result<Request<SelectedFiles>, ashpd::Error>) -> String {
    match result {
        Ok(request) => match request.response() {
            Ok(files) if files.uris().is_empty() => "ashpd no-selection(empty)".to_string(),
            Ok(files) => format!("ashpd selected count={}", files.uris().len()),
            Err(error) => ashpd_error_summary(&error),
        },
        Err(error) => ashpd_error_summary(&error),
    }
}

fn ashpd_cancelled(error: &ashpd::Error) -> bool {
    matches!(error, ashpd::Error::Response(ResponseError::Cancelled))
}

fn ashpd_error_summary(error: &ashpd::Error) -> String {
    if ashpd_cancelled(error) {
        "ashpd no-selection(cancelled)".to_string()
    } else {
        format!("ashpd no-selection(error={})", ashpd_error_category(error))
    }
}

/// Error category without raw portal messages, which may contain paths.
fn ashpd_error_category(error: &ashpd::Error) -> &'static str {
    match error {
        ashpd::Error::Response(ResponseError::Cancelled) => "cancelled",
        ashpd::Error::Response(ResponseError::Other) => "response-other",
        ashpd::Error::Portal(_) => "portal",
        ashpd::Error::Zbus(_) => "zbus",
        ashpd::Error::NoResponse => "no-response",
        ashpd::Error::PortalNotFound(_) => "portal-not-found",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfd_cancel_or_error_and_fallback_are_not_hidden() {
        assert_eq!(
            rfd_summary(false, false),
            "rfd no-selection(cancel-or-error) fallback-zenity=false"
        );
        assert_eq!(
            rfd_summary(true, true),
            "rfd selected count=1 fallback-zenity=true"
        );
    }

    #[test]
    fn clip_action_labels_and_markers_are_fixed() {
        assert_eq!(ClipAction::SetClipboard.marker(), CLIPBOARD_MARKER);
        assert_eq!(ClipAction::GetPrimary.marker(), PRIMARY_MARKER);
        assert_eq!(ClipAction::GetClipboard.label(), "get-clipboard");
        assert!(ClipAction::SetPrimary.is_set());
        assert!(!ClipAction::GetPrimary.is_set());
    }

    #[test]
    fn get_summary_never_contains_clipboard_text() {
        let secret = "user-private-text-do-not-log";
        let summary = get_summary(ClipAction::GetClipboard, secret, CLIPBOARD_MARKER);
        assert!(summary.starts_with("get-clipboard"));
        assert!(summary.contains(&format!("bytes={}", secret.len())));
        assert!(summary.contains("marker-match=false"));
        assert!(!summary.contains(secret));

        let matched = get_summary(ClipAction::GetClipboard, CLIPBOARD_MARKER, CLIPBOARD_MARKER);
        assert!(matched.contains("marker-match=true"));
    }

    #[test]
    fn arboard_errors_map_to_safe_categories() {
        assert_eq!(
            arboard_error_category(&arboard::Error::ContentNotAvailable),
            "content-not-available"
        );
        assert_eq!(
            arboard_error_category(&arboard::Error::ClipboardNotSupported),
            "clipboard-not-supported"
        );
        assert_eq!(
            arboard_error_category(&arboard::Error::ClipboardOccupied),
            "clipboard-occupied"
        );
        assert_eq!(
            arboard_error_category(&arboard::Error::ConversionFailure),
            "conversion-failure"
        );
        assert_eq!(
            arboard_error_category(&arboard::Error::Unknown {
                description: "private detail".to_string()
            }),
            "unknown"
        );
    }

    #[test]
    fn ashpd_cancel_is_separated_from_other_errors() {
        let cancelled = ashpd::Error::Response(ResponseError::Cancelled);
        assert!(ashpd_cancelled(&cancelled));
        assert_eq!(
            ashpd_error_summary(&cancelled),
            "ashpd no-selection(cancelled)"
        );

        let other = ashpd::Error::Response(ResponseError::Other);
        assert!(!ashpd_cancelled(&other));
        assert_eq!(
            ashpd_error_summary(&other),
            "ashpd no-selection(error=response-other)"
        );

        assert_eq!(
            ashpd_error_summary(&ashpd::Error::NoResponse),
            "ashpd no-selection(error=no-response)"
        );
    }
}
