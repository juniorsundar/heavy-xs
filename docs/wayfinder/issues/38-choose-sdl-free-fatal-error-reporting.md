# Preserve fatal-error reporting without SDL

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-fatal-reporting
Blocked by: 26, 27, 35

## Question

What replaces `system.show_fatal_error(title, message)`, and when may native display initialization fail without preventing the existing startup-error diagnostics?

The selected stack explicitly chooses ashpd for open/save/directory choosers but does not settle SDL's standalone fatal-error message box. [The current function](../../../src/syswindow.rs) validates two string arguments, calls `SDL_ShowSimpleMessageBox` with no parent, discards its outcome and returns no Lua values. [The bootstrap error handler](../../../src/app.rs) writes the diagnostic/traceback and `error.txt`, attempts the box, then exits 1. [The existing startup check](../../../scripts/rust-port/startup-check.sh) deliberately fails a Lua runtime with no display or reachable session bus and requires that original error, traceback, file and exit code—not an earlier display-connection failure.

Settle the fatal-reporting mechanism and its dependencies, unavailable-display/helper behavior, call/return and argument handling, and display/event-loop initialization ordering. Preserve diagnostics before any best-effort UI; do not require a successfully created winit window merely to report that creation/startup failed. No display-free window adapter or hidden fallback is selected by the test architecture. Distinguish a normal missing-display failure path from test-only facilities.

Use the [selected stack](27-select-platform-stack.md#resolution) and [process ownership](35-define-pump-and-dialog-runtime-boundary.md#resolution). Investigate existing/platform/library facilities before proposing another dependency or a handwritten message-box widget. Any loss of the existing visible error-box behavior, change to the Lua interface, helper requirement or consequential initialization trade-off needs the user's explicit decision; there is no approved fatal-box exception. Do not silently repurpose the rejected file-dialog fallback as a chosen fatal-box route.

Carry real missing-display startup/error-file checks, string-argument/zero-return tests and appropriately approved visible-path review into the migration handoff. Apply existing desktop dispositions and startup/cost gates. Resolve with the mechanism, initialization/failure policy and stop conditions, not production code or a live modal launch.

## Comments

### Live decisions — visible reporting and native initialization

The user chose **“One packaged helper”** and **“Lazy native init.”** Keep the visible fatal-error attempt using one explicitly packaged helper without adding a GUI crate; this initial helper preference was subsequently superseded by the notification decision below. Keep process-owned Rust state available from startup, but initialize winit's native display/event loop on the main thread only when an operation actually needs it. Bootstrap diagnostics and a deliberate Lua-load failure must not be masked by an earlier display failure. This is initialization state, not a display-free window adapter or hidden fallback. The subsequent signal decision must account for the pre-loop phase.

### Live clarification — desktop notification alternative

The user did **not** approve the proposed Zenity route, asking “Why not just use inotify?” and clarifying that they meant **desktop notifications**, not filesystem watching. Investigate notify-send/libnotify and the selected portal module as alternatives before choosing the reporting mechanism. The lazy-native-init decision remains; at that stage no Zenity helper, command-argument privacy trade-off or loss of modality had been approved.

### Live decision — direct desktop notification

After reviewing the alternatives, the user chose **“Direct D-Bus notification.”** This explicitly replaces the blocking error box with a bounded best-effort critical, nonmodal desktop notification and supersedes the packaged-helper preference. Use the selected future ashpd crate's public zbus re-export and the planned shared process-lifetime Tokio runtime, not Zenity, notify-send, libnotify FFI or a portal-notification fallback. Keep full diagnostics/stderr, preserve Lua argument handling/NUL truncation/zero returns, and cover connection and send work with a two-second deadline. The user accepted that daemon acceptance is not presentation evidence, post-exit retention depends on server policy/capabilities, and supported sandbox packaging needs permission review. The user subsequently chose **“Confirm revised design”** for the complete design below. No crate or reporter is adopted/implemented here.

### Resolution

The user confirmed **lazy native initialization** and **direct desktop notification**, explicitly superseding the initial packaged-helper preference after clarifying “inotify” meant desktop notifications. The blocking error box is replaced by a best-effort critical **nonmodal notification**. This is an additional reviewed compatibility difference, not preservation of the old modality or guaranteed visual delivery. The Lua function remains; no general notification interface is added.

#### Reporter and observable behavior

Keep `system.show_fatal_error(title, message)` with its existing two `luaL_checkstring` argument checks/coercions, first-NUL truncation and **zero Lua returns**. Argument errors still raise before reporting side effects. For valid arguments, write the full supplied report to stderr before any notification attempt. Preserve the bootstrap's existing stdout/traceback, error-file/core.on_error ordering and exit-1 path; the reporter must not overwrite error.txt or invent a file-recovery subsystem.

After argument handling, recoverable diagnostic/runtime/connection/encoding/service failures must not raise a new Lua error, panic on a failed stderr write, mask the original error or prevent the caller's exit. D-Bus strings require UTF-8: if truncated title/message bytes cannot be represented, retain their original stderr bytes and skip the notification rather than applying lossy replacement or narrowing Lua argument acceptance. This does not promise binary text can be displayed by a notification daemon.

Send directly to **`org.freedesktop.Notifications`**, object `/org/freedesktop/Notifications`, using the selected future ashpd 0.13.13 crate's **public zbus/zvariant re-export**. No added GUI crate, native libnotify FFI, helper executable, shell, portal-notification fallback or helper-selection configuration is chosen. This does not implement a Wayland/X11 client. The selected dependencies are planning references, not present/adopted reporter code; retain the selected Tokio executor/features rather than introducing a second runtime.

Use one notification-local session connection, independent of the file-dialog portal connection/registration. Use the existing application name/desktop identity where appropriate; native notification delivery does not require creating a window, exporting a parent or registering with the portal. Query `GetCapabilities` and pass literal diagnostic body text: escape markup characters when `body-markup` is supported, otherwise send plain text. Do not turn crash text into links or markup. Request critical urgency (byte hint 2), a fresh notification (`replaces_id = 0`), no action buttons and no automatic expiry (`expire_timeout = 0`). No notification listeners, activation/relaunch behavior or app-managed retention are needed.

#### Bounded call and delivery limits

Drive the async work on the [selected process-lifetime Tokio runtime](35-define-pump-and-dialog-runtime-boundary.md#resolution), following the later [process-signal initialization policy](37-choose-process-signal-quit-integration.md#resolution). Reuse the already-required runtime, or initialize it once for reporting if it was not needed earlier. Main-thread blocking until the send completes is selected here; it does not pump native callbacks or re-enter Lua and carries no Lua/window-handle borrow into the future. Use owned report data. Do not create a per-report runtime or fire-and-forget a task that `os.exit` may kill before submission.

Apply **one two-second Tokio deadline** around connection setup, capabilities and Notify submission/acknowledgement. Do not reset it per method, confuse it with the requested display lifetime, alter dialog connection timeouts or add retries. Runtime creation and existing diagnostic IO are separate from this async deadline; it is not a hard realtime guarantee for the entire error path. Report recoverable initialization failure through the already-written diagnostics, not new machinery. Return on acknowledgement, failure or timeout; never wait for user dismissal.

A returned notification id means daemon acceptance, **not presentation**. Critical/no-expiry requests, post-exit retention and body rendering remain server/capability/user-policy dependent. A timeout does not promise withdrawal of an already-submitted request. The full report remains on stderr and the bootstrap's existing diagnostic path regardless; no alternate GUI route is attempted. Direct IPC avoids a child argv payload but is not secret: permitted bus monitors/daemon policy may expose it. Do not add command/payload debug logging.

#### Native initialization and failure ordering

Construct the main-thread process Rust state independently of a native loop/proxy. Initialize winit's native event loop **only when a native operation actually requires it**, on the main thread and at the required native lifecycle points. Module loading, Rust-owned state and fatal reporting must not initialize a display merely to produce diagnostics. A deliberate Lua-load failure without a display must remain that original error, not an earlier display failure. Real native-window requests still fail honestly if no backend is usable; there is no fake window, SDL fallback or automatic display-free adapter.

Retain the successfully created loop across Lua restarts. The pinned winit builder marks creation attempted even when backend construction fails: retain an honest failed state/error and do not retry/recreate the loop in process, claim success or abort bootstrap before the normal Lua error path can report it. Later native operations fail safely; pure diagnostics remain available. This does not choose a pre-loop wait/signal dispatcher. [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md#resolution) supplies the subsequent absent/failed-loop wake policy while preserving these native-init and queue/lifetime requirements.

#### Alternatives and scope

Zenity and its build-time executable-path/packaging proposal were **not selected**. Neither was a packaged notify-send/libnotify route: it is lightweight but adds a helper/argv transport and subprocess deadline work. The portal notification route was considered but not chosen; the direct daemon route avoids depending on the file-dialog portal/backend/app-attribution path to report an error. Do not add helpers to Nix, change EXEFILE through a wrapper, or enable a portal-notification fallback on this decision's authority. The needed async stack remains the already-selected stack, not a new fatal-report dependency family.

Native daemon access may be denied inside confinement. Keep the [existing conditional sandbox disposition](33-complete-platform-desktop-checks.md#resolution): any actual supported confined package must validate permissions and reporting, and an incompatibility returns for review rather than a silent portal/helper fallback or a claimed pass. No new sandbox package/test is required merely for this planning decision. Native X11 remains accepted-unverified/reactive, not a reinstated proactive session gate. A desktop/session lacking notification support still gets diagnostics; visible notification is best effort.

#### Tests, touch scope and handoff

Carry into [test design](36-design-sdl-free-contract-testing.md#resolution) and [the migration handoff](28-plan-sdl3-migration.md):

- Keep malformed-argument checks and add valid/numeric/NUL/invalid-UTF-8 cases, zero-value returns, stderr-before-IPC and original-byte diagnostics. Do not turn runtime failures into Lua errors.
- Exercise the real client/conversion against a **private session bus and test-only fake notification daemon**, never the host bus. Check body capability handling, literal markup, urgency/expiry/types, empty actions, successful acknowledgement, unavailable service/bus, errors and delayed/no replies under the single deadline. Use fixed nonprivate fixtures; do not derive expected payloads from a second copy of the implementation.
- Retain the real no-display deliberate-Lua-error test: traceback/error.txt/exit 1 must survive reporter failure. Check native-loop initialization is not required to load/report, failed creation is not retried, and a successful loop persists across restart. Separate process-isolated native-init cases because of winit's one-creation rule; no fake platform is implied.
- Check the ordinary build's actual reporter as well as test facilities. Keep fake daemon/fault controls test-only or outside the binary, with no new normal-user fallback/override interface. Preserve the [calibrated costs](31-calibrate-platform-cost-gates.md#resolution), including first-use runtime/IPC time; a shared runtime is not a measured startup/latency pass.
- Later live review, with separate permission, uses a nonprivate marker to check actual notification behavior/attribution after the editor exits on the supported desktop path. Daemon acknowledgement and scripted tests do not certify presentation. Preserve all deferred/conditional dispositions and baseline-review rules.

Production touch scope is the reporter in [syswindow](../../../src/syswindow.rs), process/bootstrap/native-initialization state in [app](../../../src/app.rs) and the planned platform module, shared runtime ownership, and associated contract/startup/private-bus checks. Keep configuration/API documentation and test expectations explicit about the new nonmodal best-effort effect. No renderer, chooser result interface, Lua restart redesign or signal mechanism is selected here.

[Primary-source reference facts](../assets/fatal-error-reporting.md) distinguish cached pinned library evidence from live/compiled results. Implementation stops for loss of original diagnostics, unsafe/recreated native state, changed Lua returns, unbounded IPC, unintended host/test transport, or unreviewed compatibility/permission/cost failures. This resolution changes planning records only: no dependency, source, script, runtime notification, native loop, build, commit or push was performed.
