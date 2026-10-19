# Fatal-reporting mechanism reference

Source inspection and primary-document research supporting [Preserve fatal-error reporting without SDL](../issues/38-choose-sdl-free-fatal-error-reporting.md). The decision lives in that ticket; these are reference facts, not implementation or live evidence.

## Existing interface and bootstrap

At source reference `c80459bc0f464df68b8d8cdff553cd8288519a67`, [show_fatal_error](../../../src/syswindow.rs) checks two arguments with `luaL_checkstring`, calls the unparented SDL simple message box and discards its result, returning zero Lua values. Numeric coercion is retained; C-string use truncates at the first NUL. [The bootstrap](../../../src/app.rs) writes error/trace diagnostics and follows its existing error-file/core.on_error path before calling the reporter, then exits 1. [The startup check](../../../scripts/rust-port/startup-check.sh) requires the deliberate Lua error, traceback, error.txt and exit 1 with display variables unset and an unreachable session bus.

Cached winit 0.30.13 [EventLoopBuilder::build](https://docs.rs/winit/0.30.13/src/winit/event_loop.rs.html) sets its process creation flag before backend construction; on a backend error it does not reset it. A later build returns RecreationAttempt. A failed creation must not be treated as a successfully initialized loop or retried as an ordinary window operation. This is source evidence, not a reproduced failure.

## Native desktop notifications

The [Desktop Notifications Specification](https://specifications.freedesktop.org/notification/latest-single/) defines `org.freedesktop.Notifications`, object `/org/freedesktop/Notifications`, `GetCapabilities` and `Notify`. Native Notify requires no portal parent/window identifier or portal application registration. It returns a notification id, not proof of presentation. A daemon may be absent; hints and presentation/retention policy vary. The `persistence` capability describes retention until acknowledgement/removal. Critical urgency is the byte-valued hint 2; expiry 0 requests no automatic expiry, not an absolute delivery guarantee.

Summary/body are UTF-8 protocol strings. Body markup is capability-dependent; literal diagnostic text needs escaping when `body-markup` is advertised and plain text otherwise. Empty actions need no callbacks. Same-user or privileged bus monitors permitted by bus policy may observe method payloads; direct IPC is not confidential transport. It does avoid placing the report in a helper's command line. See [D-Bus monitoring policy](https://dbus.freedesktop.org/doc/dbus-daemon.1.html).

The selected planning reference ashpd 0.13.13 [publicly re-exports zbus and zvariant](https://docs.rs/ashpd/0.13.13/src/ashpd/lib.rs.html), independently of its optional notification portal module. Its [default executor feature is Tokio](https://docs.rs/crate/ashpd/0.13.13/features); use the selected executor, not a second async runtime. [zbus's first-party client documentation](https://github.com/z-galaxy/zbus/blob/main/book/src/client.md) includes a native Notifications client. These dependencies have not been adopted in the editor here, and no example was compiled. Context7 identified the first-party zbus docs; its Zenity lookup returned no relevant project.

## Portal and helper alternatives

[Portal Notification](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Notification.html) supports notifications expected to outlast the app, but provides no presentation feedback. It adds portal/backend/application-attribution considerations, not a native message box. The frontend [notification implementation](https://github.com/flatpak/xdg-desktop-portal/blob/main/desktop-portal/notification.c) allows host apps, including inferred/empty identities from [host app info](https://github.com/flatpak/xdg-desktop-portal/blob/main/shared/xdp-app-info-host.c); that does not prove every backend will display an unattributed notification. Explicit host registration has ordering requirements; do not infer that every unregistered host app is universally rejected. The repository's desktop entry and existing SDL metadata both use `com.lite_xl.LiteXL`.

[libnotify's build inputs](https://github.com/GNOME/libnotify/blob/master/meson.build) include GLib/GIO and gdk-pixbuf; GTK is for tests, not a required error-dialog implementation. [notify-send](https://gitlab.gnome.org/GNOME/libnotify/-/blob/master/tools/notify-send.c) is a lightweight notification client, but summary/body are command arguments. Its display-expiry option is not a bus-call deadline.

[Zenity's option table](https://gitlab.gnome.org/GNOME/zenity/-/blob/master/src/option.c) provides error/title/text/no-markup modes. Its error text is an argument; the stdin text-info viewer is a different interface, not an argv-free equivalent error box. Cached rfd 0.17.2's default Linux message-dialog implementation invokes Zenity rather than the file-dialog portal; adding that crate alone would not eliminate the helper requirement.

## Evidence limits

Only source/document inspection and executable-path lookup were performed. No helper, notification, daemon, bus call, native loop, sandbox or GUI test was launched; nothing was installed or built. The current tool shell had no notify-send/Zenity candidate on PATH, which says nothing conclusive about a Nix shell or target package. Native notification permission inside a supported sandbox, real presentation after process exit, exact closure size and timing remain unverified. Existing desktop-validation dispositions remain authoritative; no unavailable check becomes a pass or a new planning gate.
