//! Main-thread-only text clipboard and primary selection over arboard, per the
//! UTF-8-only contract in `docs/wayfinder/assets/sdl3-migration-handoff.md`: truncate at the
//! first NUL, never replace lossily, leave the selection alone on an invalid set, hide failures.
//!
//! One `arboard::Clipboard` is created lazily on the calling thread and kept for the whole
//! process lifetime: on Linux the owning process must stay alive to serve the selection.

use std::cell::RefCell;

use arboard::{ClearExtLinux, Clipboard, GetExtLinux, LinuxClipboardKind, SetExtLinux};

/// Which selection to use.
#[derive(Clone, Copy)]
pub enum Kind {
    Clipboard,
    Primary,
}

impl Kind {
    fn arboard(self) -> LinuxClipboardKind {
        match self {
            Kind::Clipboard => LinuxClipboardKind::Clipboard,
            Kind::Primary => LinuxClipboardKind::Primary,
        }
    }
}

thread_local! {
    static CLIPBOARD: RefCell<Option<Clipboard>> = const { RefCell::new(None) };
}

/// Applies `f` to the lazily created clipboard; `None` if it cannot be created (retried later).
fn with<R>(f: impl FnOnce(&mut Clipboard) -> R) -> Option<R> {
    CLIPBOARD.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    })
}

/// Text bytes of the selection; empty for any failure, non-text or unavailable clipboard.
pub fn get(kind: Kind) -> Vec<u8> {
    with(|cb| cb.get().clipboard(kind.arboard()).text().ok())
        .flatten()
        .map(|text| prepare(text.as_bytes()).unwrap_or_default().as_bytes().to_vec())
        .unwrap_or_default()
}

/// Replaces the selection's text; invalid UTF-8 or an unavailable clipboard is a silent no-op.
pub fn set(kind: Kind, text: &[u8]) {
    let Some(text) = prepare(text) else { return };
    // Ownership lasts while this process runs; persistence after exit needs a clipboard manager.
    let _ = with(|cb| {
        if text.is_empty() {
            cb.clear_with().clipboard(kind.arboard())
        } else {
            cb.set().clipboard(kind.arboard()).text(text.to_owned())
        }
    });
}

/// Truncates at the first NUL, then accepts the prefix only if it is valid UTF-8.
fn prepare(text: &[u8]) -> Option<&str> {
    std::str::from_utf8(crate::luaargs::nul_cut(text)).ok()
}

#[cfg(test)]
mod tests {
    use super::prepare;

    #[test]
    fn plain() {
        assert_eq!(prepare(b"hello"), Some("hello"));
    }

    #[test]
    fn empty() {
        assert_eq!(prepare(b""), Some(""));
    }

    #[test]
    fn nul_truncation() {
        assert_eq!(prepare(b"ab\0cd"), Some("ab"));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        assert_eq!(prepare(b"ab\xffcd"), None);
    }

    #[test]
    fn invalid_after_nul_is_ignored() {
        assert_eq!(prepare(b"ab\0\xff"), Some("ab"));
    }

    #[test]
    fn multibyte() {
        assert_eq!(prepare("héllo→".as_bytes()), Some("héllo→"));
    }
}
