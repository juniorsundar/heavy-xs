//! Process-lived services: the shared Tokio runtime and SIGINT/SIGTERM quit requests.
//!
//! Startup inspects the previous dispositions before Lua or any native display. Only default-disposition
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
#[cfg(test)]
pub(crate) static QUITS_DELIVERED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Inspects SIGINT/SIGTERM and registers listeners for the default-disposition ones.
///
/// Must run before native display initialization and before Lua. If any listener is needed this
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
        #[cfg(test)]
        let registration = if name == "SIGTERM" && tests::FAIL_REGISTRATION.load(Ordering::SeqCst) {
            Err(std::io::Error::other("scripted registration failure"))
        } else { signal(kind) };
        #[cfg(not(test))]
        let registration = signal(kind);
        let stream = registration.map_err(|err| format!("cannot register {name} handler: {err}"))?;
        streams.push((name, stream));
    }
    for (name, stream) in streams {
        runtime.spawn(watch(name, stream));
        #[cfg(test)]
        tests::WATCHERS_SPAWNED.fetch_add(1, Ordering::SeqCst);
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
        let received = stream.recv().await;
        #[cfg(test)]
        let received = if name == "SIGINT" && tests::END_STREAM.load(Ordering::SeqCst) { None } else { received };
        match received {
            Some(()) => {
                if CLOSING.load(Ordering::SeqCst) {
                    return;
                }
                crate::platform::request_quit();
                #[cfg(test)]
                QUITS_DELIVERED.fetch_add(1, Ordering::SeqCst);
            }
            None => {
                if !CLOSING.load(Ordering::SeqCst) {
                    crate::platform::report_error(format!(
                        "{name} signal stream ended unexpectedly"
                    ));
                }
                #[cfg(test)]
                tests::STREAMS_ENDED.fetch_add(1, Ordering::SeqCst);
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
    pub(super) static FAIL_REGISTRATION: AtomicBool = AtomicBool::new(false);
    pub(super) static END_STREAM: AtomicBool = AtomicBool::new(false);
    pub(super) static WATCHERS_SPAWNED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    pub(super) static STREAMS_ENDED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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
                assert!(String::from_utf8_lossy(&output.stdout).contains("test result: ok. 1 passed;"), "{case}: signal subject did not run");
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
    fn fatal_runtime_failure_keeps_diagnostics() {
        let output = std::process::Command::new("timeout")
            .args(["-k", "1", "10"])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "services::tests::fatal_runtime_child", "--ignored", "--nocapture"])
            .env("LXL_FATAL_RUNTIME_TEST", "1")
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent-lite-xl-test-bus")
            .env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY").env_remove("WAYLAND_SOCKET")
            .env_remove("XDG_RUNTIME_DIR").env_remove("XAUTHORITY")
            .output().unwrap();
        assert!(output.status.success(), "{:?}: {}", output.status, String::from_utf8_lossy(&output.stdout));
        assert_eq!(output.stderr, b"Fatal runtime: original diagnostic\n");
    }

    #[test]
    #[ignore = "isolated cached-runtime failure; invoked by fatal_runtime_failure_keeps_diagnostics"]
    fn fatal_runtime_child() {
        assert_eq!(std::env::var("LXL_FATAL_RUNTIME_TEST").as_deref(), Ok("1"));
        assert!(RUNTIME.set(Err("scripted runtime initialization failure".into())).is_ok());
        crate::fatal::report(b"Fatal runtime".to_vec(), b"original diagnostic".to_vec());
        assert_eq!(runtime().unwrap_err(), "scripted runtime initialization failure");
    }

    #[test]
    fn invalid_disposition_query_is_an_error() {
        assert!(previous_handler(-1, "invalid").unwrap_err().starts_with("cannot inspect invalid disposition: "));
    }

    #[test]
    fn service_faults_stay_in_their_process_and_error_path() {
        for case in ["runtime", "registration", "listener", "closed-listener", "first-error"] {
            let work = std::env::temp_dir().join(format!("lite-xl-service-fault-{}-{case}", std::process::id()));
            std::fs::create_dir(&work).unwrap();
            let output = std::process::Command::new("timeout").args(["-k", "1", "10"])
                .arg(std::env::current_exe().unwrap())
                .args(["--exact", "services::tests::service_fault_child", "--ignored", "--nocapture"])
                .current_dir(&work).env("LXL_SERVICE_FAULT_TEST", case)
                .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent-lite-xl-test-bus")
                .env("HOME", &work).env("LITE_USERDIR", work.join("user"))
                .env("LITE_PREFIX", work.join("missing-data")).env_remove("LITE_XL_RUNTIME")
                .env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY").env_remove("WAYLAND_SOCKET")
                .env_remove("XDG_RUNTIME_DIR").env_remove("XAUTHORITY")
                .output().unwrap();
            std::fs::remove_dir_all(work).unwrap();
            assert!(output.status.success(), "{case}: {:?} {}{}", output.status, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("service fault {case} passed")));
            let expected = match case {
                "runtime" => "Error initializing process services: scripted runtime failure\n",
                "registration" => "Error initializing process services: cannot register SIGTERM handler: scripted registration failure\n",
                _ => "",
            };
            assert_eq!(output.stderr, expected.as_bytes(), "{case}");
        }
    }

    #[test]
    #[ignore = "isolated service fault subject; invoked by service_faults_stay_in_their_process_and_error_path"]
    fn service_fault_child() {
        let case = std::env::var("LXL_SERVICE_FAULT_TEST").expect("isolated service fault role");
        unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL); libc::signal(libc::SIGTERM, libc::SIG_DFL); }
        assert!(crate::native::untouched() && crate::platform::accepts(crate::platform::generation()));
        if case == "runtime" || case == "registration" {
            if case == "runtime" { assert!(RUNTIME.set(Err("scripted runtime failure".into())).is_ok()); }
            else { FAIL_REGISTRATION.store(true, Ordering::SeqCst); }
            assert_eq!(crate::app::run(), std::process::ExitCode::FAILURE);
            FAIL_REGISTRATION.store(false, Ordering::SeqCst);
            assert!(init_signals().is_err()); // the setup failure stays cached
            assert_eq!(WATCHERS_SPAWNED.load(Ordering::SeqCst), 0);
            assert!(crate::native::untouched());
            assert_eq!(crate::custom_events::register_custom_event(b"not-started", None).unwrap_err(), "Custom events are not initialised");
            if case == "registration" {
                assert_ne!(previous_handler(libc::SIGINT, "SIGINT").unwrap(), libc::SIG_DFL);
                assert_eq!(previous_handler(libc::SIGTERM, "SIGTERM").unwrap(), libc::SIG_DFL);
            }
        } else {
            assert!(matches!(case.as_str(), "listener" | "closed-listener" | "first-error"));
            END_STREAM.store(true, Ordering::SeqCst);
            init_signals().unwrap();
            assert_eq!(WATCHERS_SPAWNED.load(Ordering::SeqCst), 2);
            if case == "closed-listener" { close(); }
            if case == "first-error" { crate::platform::report_error("earlier driver failure".into()); }
            assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);
            if case == "listener" { assert!(crate::platform::wait(Some(std::time::Duration::from_secs(2))).unwrap()); }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while STREAMS_ENDED.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_eq!(STREAMS_ENDED.load(Ordering::SeqCst), 1);
            if case != "closed-listener" {
                // Protected call observes the real Lua error boundary, not a forged quit.
                unsafe {
                    let l = mlua::ffi::luaL_newstate();
                    assert!(!l.is_null());
                    mlua::ffi::lua_pushcfunction(l, crate::sysevents::poll_event);
                    assert_eq!(mlua::ffi::lua_pcall(l, 0, 1, 0), mlua::ffi::LUA_ERRRUN);
                    let error = std::ffi::CStr::from_ptr(mlua::ffi::lua_tolstring(l, -1, std::ptr::null_mut()));
                    assert_eq!(error.to_bytes(), if case == "first-error" { &b"earlier driver failure"[..] } else { &b"SIGINT signal stream ended unexpectedly"[..] });
                    mlua::ffi::lua_close(l);
                }
            }
            assert!(crate::platform::accepts(crate::platform::generation()));
            assert!(!crate::platform::ready());
            assert!(crate::platform::take_error().is_none() && crate::platform::pop().is_none());
        }
        close();
        crate::platform::close();
        println!("service fault {case} passed");
    }

    #[test]
    fn only_default_disposition_is_eligible() {
        assert!(is_default(libc::SIG_DFL));
        assert!(!is_default(libc::SIG_IGN));
        assert!(!is_default(libc::SIG_DFL + 2));
    }
}
