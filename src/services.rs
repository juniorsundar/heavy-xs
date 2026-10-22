//! Process-lived services: the shared Tokio runtime and SIGINT/SIGTERM quit requests.
//!
//! Startup inspects the previous dispositions before SDL or Lua. Only default-disposition
//! signals are captured; ignored and pre-existing custom handlers are left alone, as is
//! SIGPIPE. Signal tasks only publish owned Rust state through `platform`; they never enter
//! Lua or terminate the process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use tokio::runtime::Runtime;
use tokio::signal::unix::{signal, Signal, SignalKind};

static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
static SIGNALS: OnceLock<Result<(), String>> = OnceLock::new();
static CLOSING: AtomicBool = AtomicBool::new(false);

/// Inspects SIGINT/SIGTERM and registers listeners for the default-disposition ones.
///
/// Must run before SDL event initialization and before Lua. If any listener is needed this
/// also initializes the shared runtime, so a later [`runtime`] call reuses it. A setup failure
/// is returned to the caller, which reports it and exits nonzero before starting Lua.
pub fn init_signals() -> Result<(), String> {
    SIGNALS.get_or_init(setup_signals).clone()
}

fn setup_signals() -> Result<(), String> {
    let mut selected = Vec::new();
    for (number, name, kind) in [
        (libc::SIGINT, "SIGINT", SignalKind::interrupt()),
        (libc::SIGTERM, "SIGTERM", SignalKind::terminate()),
    ] {
        if is_default(previous_handler(number, name)?) {
            selected.push((name, kind));
        }
    }
    if selected.is_empty() {
        return Ok(());
    }

    let runtime = runtime()?;
    // `signal()` needs the driver that `signal()` will register with to be current. Create
    // both streams before spawning so a partial setup fails without leaving a live watcher.
    let _guard = runtime.enter();
    let mut streams = Vec::with_capacity(selected.len());
    for (name, kind) in selected {
        let stream =
            signal(kind).map_err(|err| format!("cannot register {name} handler: {err}"))?;
        streams.push((name, stream));
    }
    for (name, stream) in streams {
        runtime.spawn(watch(name, stream));
    }
    Ok(())
}

/// The process-lifetime runtime, created lazily when no signal listener required it early.
///
/// The cached error is retained; a failed runtime is never silently replaced.
pub fn runtime() -> Result<&'static Runtime, String> {
    match RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|err| format!("cannot create async runtime: {err}"))
    }) {
        Ok(runtime) => Ok(runtime),
        Err(err) => Err(err.clone()),
    }
}

/// Stops signal tasks from publishing quit requests or errors after final exit begins.
pub fn close() {
    CLOSING.store(true, Ordering::SeqCst);
}

async fn watch(name: &'static str, mut stream: Signal) {
    loop {
        match stream.recv().await {
            Some(()) => {
                if CLOSING.load(Ordering::SeqCst) {
                    return;
                }
                crate::platform::request_quit();
            }
            None => {
                if !CLOSING.load(Ordering::SeqCst) {
                    crate::platform::report_error(format!(
                        "{name} signal stream ended unexpectedly"
                    ));
                }
                return;
            }
        }
    }
}

fn previous_handler(signal: libc::c_int, name: &str) -> Result<libc::sighandler_t, String> {
    let mut previous: libc::sigaction = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigaction(signal, std::ptr::null(), &mut previous) } != 0 {
        return Err(format!(
            "cannot inspect {name} disposition: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(previous.sa_sigaction)
}

fn is_default(handler: libc::sighandler_t) -> bool {
    handler == libc::SIG_DFL
}

#[cfg(test)]
mod tests {
    use super::*;

    static CUSTOM_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    extern "C" fn custom_handler(_: libc::c_int) {
        CUSTOM_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    // Each disposition is established inside a fresh child, not assumed to survive exec.
    #[test]
    fn real_signals_respect_dispositions_in_children() {
        for signal in ["INT", "TERM"] {
            for disposition in ["default", "ignore", "custom"] {
                let case = format!("{signal}:{disposition}");
                let output = std::process::Command::new("timeout")
                    .args(["-k", "1", "10"])
                    .arg(std::env::current_exe().unwrap())
                    .args(["--exact", "services::tests::signal_child", "--ignored", "--nocapture"])
                    .env("LXL_SIGNAL_TEST_CASE", &case)
                    .output().unwrap();
                assert!(output.status.success(), "{case}: {}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            }
        }
    }

    #[test]
    #[ignore = "isolated subprocess subject; invoked by real_signals_respect_dispositions_in_children"]
    fn signal_child() {
        let case = std::env::var("LXL_SIGNAL_TEST_CASE").expect("child test role");
        let (name, disposition) = case.split_once(':').unwrap();
        let signal = if name == "INT" { libc::SIGINT } else { libc::SIGTERM };
        let handler = match disposition {
            "default" => libc::SIG_DFL,
            "ignore" => libc::SIG_IGN,
            "custom" => custom_handler as *const () as libc::sighandler_t,
            _ => panic!("unknown disposition"),
        };
        unsafe { libc::signal(signal, handler); }
        init_signals().unwrap();
        if disposition != "default" { assert_eq!(previous_handler(signal, name).unwrap(), handler); }
        assert_eq!(unsafe { libc::raise(signal) }, 0);
        if disposition == "default" {
            assert!(crate::platform::wait(Some(std::time::Duration::from_secs(2))).unwrap());
            assert!(crate::platform::ready());
            crate::platform::retire_generation();
            assert!(crate::platform::ready()); // readiness/restart never consumes a request
            assert_eq!(crate::platform::pop(), Some(crate::platform::Input::Quit));
            assert!(!crate::platform::wait(Some(std::time::Duration::from_millis(20))).unwrap());
        } else {
            assert!(!crate::platform::wait(Some(std::time::Duration::from_millis(20))).unwrap());
            assert_eq!(CUSTOM_CALLS.load(Ordering::SeqCst), usize::from(disposition == "custom"));
        }
        close();
        crate::platform::close();
    }

    #[test]
    fn only_default_disposition_is_eligible() {
        assert!(is_default(libc::SIG_DFL));
        assert!(!is_default(libc::SIG_IGN));
        assert!(!is_default(libc::SIG_DFL + 2));
    }
}
