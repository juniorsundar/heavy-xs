//! Portal requests own no Lua data. Borrow native handles only on their request thread.
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use ashpd::WindowIdentifier;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::custom_events::{push_custom_event_at, CustomEvent};
use crate::platform::Payload;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Open,
    Save,
    Directory,
}

pub struct Options {
    pub location: Option<Vec<u8>>,
    pub title: Option<Vec<u8>>,
    pub accept: Option<Vec<u8>>,
    pub many: bool,
    pub filters: Vec<(Vec<u8>, Vec<u8>)>,
}

type Outcome = Result<Option<Vec<Vec<u8>>>, String>;

fn finish(id: usize, generation: u64, result: Outcome) {
    let (code, payload) = match result {
        Ok(Some(paths)) => (0, Some(Payload::Paths(paths))),
        Ok(None) => (1, None),
        Err(error) => (2, Some(Payload::Message(error.into_bytes()))),
    };
    let _ = push_custom_event_at(
        b"dialogfinished",
        CustomEvent {
            code,
            data1: id,
            payload,
        },
        Some(generation),
    );
}

pub fn start(parent: u32, id: usize, kind: Kind, options: Options) {
    let generation = crate::platform::generation();
    if !crate::platform::accepts(generation) {
        return;
    }
    let runtime = match crate::services::runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            finish(id, generation, Err(error));
            return;
        }
    };
    let parent = match crate::native::retain_parent(parent) {
        Ok(parent) => parent,
        Err(error) => {
            finish(id, generation, Err(error));
            return;
        }
    };
    if let Err(error) = std::thread::Builder::new()
        .name("file-dialog".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(choose(&parent, kind, options))
            }))
            .unwrap_or_else(|_| Err("File dialog worker panicked".into()));
            finish(id, generation, result);
            // Parent::drop returns its last lease to the main thread, even for stale results.
            drop(parent);
        })
    {
        finish(
            id,
            generation,
            Err(format!("Cannot start file dialog thread: {error}")),
        );
    }
}

fn text<'a>(bytes: &'a [u8], field: &str) -> Result<&'a str, String> {
    std::str::from_utf8(bytes).map_err(|_| format!("File dialog {field} is not valid UTF-8"))
}

fn filters(specs: &[(Vec<u8>, Vec<u8>)]) -> Result<Vec<FileFilter>, String> {
    specs
        .iter()
        .map(|(name, pattern)| {
            let mut filter = FileFilter::new(text(name, "filter name")?);
            let pattern = text(pattern, "filter pattern")?;
            if pattern == "*" {
                return Ok(filter.glob("*"));
            }
            for extension in pattern.split(';') {
                if extension.is_empty() {
                    return Err("Empty pattern not allowed".into());
                }
                if !extension
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                {
                    return Err(
                        "Invalid character in pattern (Only [a-zA-Z0-9_.-] allowed, or a single *)"
                            .into(),
                    );
                }
                filter = filter.glob(&format!("*.{extension}"));
            }
            Ok(filter)
        })
        .collect()
}

struct SaveLocation {
    folder: Option<PathBuf>,
    file: Option<PathBuf>,
    name: Option<String>,
}

fn save_location(bytes: &[u8]) -> SaveLocation {
    let path = Path::new(std::ffi::OsStr::from_bytes(bytes));
    let metadata = std::fs::metadata(path);
    let existing = metadata.as_ref().is_ok_and(|m| m.is_file());
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let new_named = metadata
        .as_ref()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        && folder.is_dir();
    if existing || new_named {
        let name = path.file_name().unwrap_or(std::ffi::OsStr::new("."));
        let name = name.to_str().map(str::to_owned);
        // current_name is UTF-8, but current_file carries the complete native filename bytes.
        let byte_name = name.is_none();
        return SaveLocation {
            folder: if new_named {
                Some(folder.to_owned())
            } else {
                None
            },
            file: if existing || byte_name {
                Some(path.to_owned())
            } else {
                None
            },
            name,
        };
    }
    SaveLocation {
        folder: Some(path.to_owned()),
        file: None,
        name: None,
    }
}

async fn choose(parent: &crate::native::Parent, kind: Kind, options: Options) -> Outcome {
    let title = options
        .title
        .as_deref()
        .map(|b| text(b, "title"))
        .transpose()?
        .unwrap_or(match kind {
            Kind::Open => "Open File",
            Kind::Save => "Save File",
            Kind::Directory => "Open Folder",
        });
    let accept = options
        .accept
        .as_deref()
        .map(|b| text(b, "accept label"))
        .transpose()?;
    let filters = filters(&options.filters)?;
    let handle = parent
        .window()
        .window_handle()
        .map_err(|e| e.to_string())?
        .as_raw();
    let display = parent
        .window()
        .display_handle()
        .map_err(|e| e.to_string())?
        .as_raw();
    let identifier = WindowIdentifier::from_raw_handle(&handle, Some(&display))
        .await
        .ok_or("Cannot export file dialog parent")?;
    let request = if kind == Kind::Save {
        let mut request = SelectedFiles::save_file()
            .identifier(identifier)
            .title(title)
            .modal(true)
            .accept_label(accept)
            .filters(filters);
        if let Some(location) = &options.location {
            let location = save_location(location);
            request = request
                .current_folder::<PathBuf>(location.folder)
                .map_err(|e| e.to_string())?
                .current_file::<PathBuf>(location.file)
                .map_err(|e| e.to_string())?
                .current_name(location.name.as_deref());
        }
        request.send().await
    } else {
        let mut request = SelectedFiles::open_file()
            .identifier(identifier)
            .title(title)
            .modal(true)
            .accept_label(accept)
            .multiple(options.many)
            .directory(kind == Kind::Directory)
            .filters(filters);
        if let Some(location) = &options.location {
            request = request
                .current_folder::<&Path>(Some(Path::new(std::ffi::OsStr::from_bytes(location))))
                .map_err(|e| e.to_string())?;
        }
        request.send().await
    };
    // Keep the export and parent alive until ashpd has received the terminal response.
    selected(request.and_then(|r| r.response()))
}

fn selected(result: Result<SelectedFiles, ashpd::Error>) -> Outcome {
    match result {
        Err(ashpd::Error::Response(_)) => Ok(None), // SDL maps portal responses 1 and 2 to cancel.
        Err(error) => Err(format!("Portal file chooser: {error}")),
        Ok(files) if files.uris().is_empty() => Ok(None),
        Ok(files) => paths(files.uris().iter().map(|uri| uri.as_str())).map(Some),
    }
}

fn paths<'a>(uris: impl IntoIterator<Item = &'a str>) -> Result<Vec<Vec<u8>>, String> {
    uris.into_iter().map(local_path).collect()
}

// Match SDL's valid local-file/hostname/percent-decoding behavior without its malformed-input UB.
fn local_path(uri: &str) -> Result<Vec<u8>, String> {
    let error = || "Portal returned an invalid or non-local file URI".to_owned();
    let mut path = uri
        .strip_prefix("file:")
        .filter(|p| p.starts_with('/'))
        .ok_or_else(error)?;
    if let Some(authority) = path.strip_prefix("//") {
        let (host, suffix) = authority.split_once('/').ok_or_else(error)?;
        if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
            let mut hostname = [0u8; 257];
            if unsafe { libc::gethostname(hostname.as_mut_ptr().cast(), 256) } != 0 {
                return Err(error());
            }
            let end = hostname.iter().position(|&b| b == 0).unwrap_or(256);
            if !host.as_bytes().eq_ignore_ascii_case(&hostname[..end]) {
                return Err(error());
            }
        }
        // Preserve SDL's extra slash removal after an explicit local host.
        path = if !host.is_empty() && suffix.starts_with('/') {
            &suffix[1..]
        } else {
            &path[path.len() - suffix.len() - 1..]
        };
    }
    let mut bytes = path.bytes();
    let mut decoded = Vec::with_capacity(path.len());
    while let Some(b) = bytes.next() {
        let b = if b == b'%' {
            let hi = (bytes.next().ok_or_else(error)? as char)
                .to_digit(16)
                .ok_or_else(error)?;
            let lo = (bytes.next().ok_or_else(error)? as char)
                .to_digit(16)
                .ok_or_else(error)?;
            (hi * 16 + lo) as u8
        } else {
            b
        };
        if b == 0 {
            return Err(error());
        }
        decoded.push(b);
    }
    if decoded.is_empty() {
        return Err(error());
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_uris_preserve_path_bytes_and_reject_bad_payloads() {
        for (uri, expected) in [
            ("file:///tmp/a%20b", b"/tmp/a b".as_slice()),
            ("file:/tmp/a", b"/tmp/a"),
            ("file://LOCALHOST/tmp/%FF%23%3f", b"/tmp/\xff#?"),
            ("file:////double", b"//double"),
            ("file://localhost//double", b"double"),
            ("file://localhost/", b"/"),
            ("file:///a/../b+z", b"/a/../b+z"),
            ("file:///", b"/"),
            ("file:///tmp/日本語", "/tmp/日本語".as_bytes()),
        ] {
            assert_eq!(local_path(uri).unwrap(), expected, "{uri}");
        }
        for uri in [
            "file://remote.invalid/a",
            "http://localhost/a",
            "file:relative",
            "file://localhost",
            "file://localhost//",
            "file:///a%",
            "file:///a%zz",
            "file:///a%00b",
        ] {
            assert!(local_path(uri).is_err(), "{uri}");
        }
        let mut name = [0u8; 257];
        assert_eq!(
            unsafe { libc::gethostname(name.as_mut_ptr().cast(), 256) },
            0
        );
        let host = std::ffi::CStr::from_bytes_until_nul(&name)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            local_path(&format!("file://{host}/local")).unwrap(),
            b"/local"
        );
    }

    #[test]
    fn responses_are_atomic_and_cancel_codes_match_sdl() {
        for error in [
            ashpd::desktop::ResponseError::Cancelled,
            ashpd::desktop::ResponseError::Other,
        ] {
            assert_eq!(selected(Err(ashpd::Error::Response(error))).unwrap(), None);
        }
        assert_eq!(selected(Ok(SelectedFiles::default())).unwrap(), None);
        assert!(paths(["file:///ok", "https://remote/bad"]).is_err());
    }

    #[test]
    fn filters_and_save_location_follow_the_portal_baseline() {
        let f = filters(&[
            (b"Images".to_vec(), b"png;jpg".to_vec()),
            (b"All".to_vec(), b"*".to_vec()),
        ])
        .unwrap();
        assert_eq!(f[0].pattern_filters(), ["*.png", "*.jpg"]);
        assert_eq!(f[1].pattern_filters(), ["*"]);
        for pattern in ["", "a;", ";a", "a;;b", "*.png", "a/*"] {
            assert!(filters(&[(vec![], pattern.as_bytes().to_vec())]).is_err());
        }
        let dir = std::env::temp_dir().join(format!("lxl-dialog-location-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("new.txt");
        let new = save_location(path.as_os_str().as_bytes());
        assert!(new.folder.is_some() && new.file.is_none());
        assert_eq!(new.name.as_deref(), Some("new.txt"));
        std::fs::write(&path, "").unwrap();
        let existing = save_location(path.as_os_str().as_bytes());
        assert!(existing.file.is_some() && existing.folder.is_none());
        assert_eq!(existing.name.as_deref(), Some("new.txt"));
        let directory = save_location(dir.as_os_str().as_bytes());
        assert!(directory.folder.is_some() && directory.file.is_none() && directory.name.is_none());
        let raw_name = dir.join(std::ffi::OsStr::from_bytes(b"\xff"));
        let new = save_location(raw_name.as_os_str().as_bytes());
        assert_eq!(new.file.as_ref(), Some(&raw_name));
        assert!(new.name.is_none() && new.folder.is_some());
        std::fs::write(&raw_name, "").unwrap();
        let existing = save_location(raw_name.as_os_str().as_bytes());
        assert_eq!(existing.file.as_ref(), Some(&raw_name));
        assert!(existing.name.is_none() && existing.folder.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
