# Choose process-signal quit and wake integration

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-signal-quit
Blocked by: 26, 35, 38

## Question

How should SIGINT/SIGTERM retain SDL's graceful-quit behavior after SDL's signal handling disappears, without unsafe signal-handler work, an unwakeable Lua wait or an unreviewed change to the selected runtime's initialization and cost?

Inspection during [Design SDL-free platform contract testing](36-design-sdl-free-contract-testing.md) found that [the existing startup check](../../../scripts/rust-port/startup-check.sh) sends a real SIGINT and requires a clean exit with no update/draw after projects are removed. [Application initialization](../../../src/app.rs) only explicitly handles SIGPIPE. In the inspected [SDL 3.4.18 quit source](https://github.com/libsdl-org/SDL/blob/release-3.4.18/src/events/SDL_quit.c), SDL installs SIGINT/SIGTERM handling when their previous disposition is default, sets a pending flag in the handler, and emits quit during pumping. The cached winit Linux source has no corresponding signal handler. Removing SDL does not automatically preserve this path; static source evidence is not a reproduced signal test, and the SDL bindings' version is not proof of the linked runtime's version.

The requirement to retain graceful quit and the real subprocess check is already settled; do not silently replace it with synthetic quit injection, change the expected exit status, or drop SIGTERM. Resolve the ownership and wake mechanism, registration/previous-disposition policy, process-versus-Lua-generation lifetime and restart-race handling. Account for interruption during native waits and open dialogs, asynchronous-signal-safety, and delivery without Lua reentrancy. Keep final exit independent of unanswered choosers.

Use the [selected process/runtime ownership](35-define-pump-and-dialog-runtime-boundary.md#resolution) and the native-initialization/failure policy from [Preserve fatal-error reporting without SDL](38-choose-sdl-free-fatal-error-reporting.md#resolution). Native initialization is lazy and failure is retained without recreation; do not assume a loop/proxy exists during all startup/error paths. Account for this pre-loop/failed-loop phase without silently changing the diagnostic or queue contract. Compare reuse of selected runtime facilities with any justified minimal alternative before choosing: do not silently add a signal dispatcher/thread/dependency or initialize the lazy runtime earlier without reviewing the trade-off. Preserve the [calibrated startup/idle/memory gates](31-calibrate-platform-cost-gates.md#resolution), and define safe failure handling and real subprocess evidence for the handoff. Consult grilling, domain-modeling and codebase-design. Planning only; no signal delivery, process termination, production edits or desktop tests are authorized by this ticket.

## Comments

### Live decision — preserve graceful-quit policy

The user chose **“Preserve quit policy.”** Handle SIGINT/SIGTERM only when their previous disposition is default; preserve ignored and pre-existing custom dispositions. Coalesce pending requests into the existing normal-polling `quit` path, including across Lua restarts. A quit request retains existing unsaved-document handling: it is not a guaranteed immediate exit, forced termination, automatic save/discard, or second-Ctrl-C force-exit policy. At that stage the mechanism and initialization/failure policy remained undecided.

### Live decisions — reuse Tokio and fail required setup honestly

The user chose **“Reuse Tokio early”** and **“Fail startup.”** If either signal needs handling, initialize the selected shared process-lifetime one-worker Tokio runtime before Lua and register signal listeners there, preserving ignored/custom dispositions. This explicitly revises the earlier lazy-runtime policy; native display and D-Bus connections stay lazy. Enable signal support and account for its transitive dependencies and early startup/RSS costs, rather than adding a dedicated signal-reader thread or custom pipe handler. Required runtime/registration failures report on stderr and fail startup nonzero before Lua; deliberately skipped ignored/custom signals are not failures. The user subsequently chose **“Confirm signal design”** for the complete proposal below.

### Resolution

The user confirmed preserving SDL's graceful-quit policy, reusing the selected Tokio runtime **earlier**, failing required setup honestly, and the complete routing/lifetime/cutover design. This is a planning decision; it adopts no dependency or implementation.

#### Ownership, registration and initialization

Own SIGINT/SIGTERM handling at process scope, outside the Lua-restart loop. Before Lua and before any competing application signal initialization, inspect each previous disposition with the existing libc facilities. Register a Tokio Unix signal stream **only for SIG_DFL**; leave SIG_IGN and pre-existing custom handlers unchanged. Serialize the application's checks/registration, create the actual listeners before proceeding into Lua, and do not defer registration to a task that might not run before startup completes. Do not overwrite or monitor later plugin handler changes. Preserve the existing SIGPIPE policy; no additional signal family is selected.

If either signal is eligible, initialize the [selected shared Tokio 1.53.3 runtime](35-define-pump-and-dialog-runtime-boundary.md#resolution) during startup: one process-lifetime runtime, one async worker, IO/time enabled, with signal support and its required transitive dependencies. **This supersedes the earlier always-lazy runtime initialization preference.** If neither is eligible, keep the runtime lazy until an ordinary operation needs it. Dialogs, reporting and the signal tasks share that same successfully initialized runtime; no per-listener/per-Lua/per-dialog runtime, dedicated signal-reader thread or new dispatcher is selected. One async worker does not promise that libraries create no helper threads.

Native display/event-loop creation remains lazy under [Preserve fatal-error reporting without SDL](38-choose-sdl-free-fatal-error-reporting.md#resolution). Starting Tokio must not connect to the desktop bus, initialize clipboard, create a native loop/window or require a display. Signal capture uses Tokio's handler machinery, not an application-authored handler doing allocations, locking, Lua, logging or EventLoopProxy calls. Ordinary runtime tasks perform the later Rust-state/wake work. Retain listeners for the process lifetime: Tokio does not restore default behavior merely because a stream is dropped, so do not drop/reinstall streams on Lua restart or promise per-generation restoration.

#### Quit requests, pre-loop waits and races

A signal becomes a **request for the existing normal quit path**, not direct OS termination. Preserve ignored/custom dispositions, ordinary unsaved-document confirmation and the existing `quit` event shape. No second-Ctrl-C force-exit, signal counter, automatic save/discard, new Lua event or public signal interface is selected.

Normal Tokio tasks publish process-owned pending/coalescing state **before** waking a waiter. Keep at most one outstanding process quit request; signal counts/order are not a public delivery guarantee. The main thread admits it to the shared production owned-Rust queue at normal polling/pump points, preserving already-queued event order, and creates Lua values only when `poll_event` returns it. Do not invoke Lua from the handler, runtime task or native callback.

Pending and queued-but-unconsumed process quits are **not Lua-generation-tagged** and must survive generation retirement/queue filtering. Clear the request when normal polling consumes it, not merely on wake or `wait_event` readiness. Once consumed, respect Lua's normal handling/cancellation/restart; do not automatically reissue it forever. A signal arriving during Lua teardown/startup remains pending for the next generation. This does not permit old window inputs or old dialog completions to cross the [existing generation fence](35-define-pump-and-dialog-runtime-boundary.md#resolution).

Before a native loop/proxy exists, store requests without creating a display. Any blocking pre-loop Rust wait uses a Rust notifier on the same runtime, with registration/predicate rechecking and stored notification semantics appropriate for the single main-thread consumer. Do not rely on an unregistered `notify_waiters` broadcast. Rust state and owned data, not a Lua pointer/borrow, cross that wait. Queue/readiness predicates remain authoritative; stale/internal notifications alone do not become public readiness or events.

Once a live EventLoopProxy is published, use it from normal thread/task context to interrupt the existing winit pump. Publish/register the wake target and recheck pending state before sleeping; a signal racing proxy publication cannot fall between a missing proxy and a missed pending check. Keep pre-loop notifications and proxy availability process-owned across Lua restarts. Release mutable platform borrows/locks before pumping callbacks or entering Lua.

Retain the [poll/wait contract](35-define-pump-and-dialog-runtime-boundary.md#resolution): non-consuming wait readiness, ordinary queue consumption, the existing millisecond cast/clamping/zero-timeout quirks and false-on-timeout/error behavior. One monotonic deadline spans spurious/internal wakes; do not reset it or introduce periodic polling, short-timeout inflation or an idle-spin fix. Retain winit's native timed/indefinite pump rather than replacing it with a descriptor-only wait that has not preserved backend timer semantics.

#### Failure and final exit

Required runtime construction, disposition inspection or listener registration failure reports clearly to stderr and fails startup nonzero **before Lua**. Do not continue with partial/unreliable handling, silently revert to default termination, initialize a display for the diagnostic or introduce a pipe/helper fallback. Intentionally skipped ignored/custom dispositions are not failures. This is a pre-Lua setup error, not a promise that the bootstrap has already written error.txt.

Unexpected listener/driver failure during a live session must wake/report through the normal main-thread Rust/application error path rather than silently strand an indefinite wait. Never terminate from the worker, bypass unsaved-document/error handling or invent a public failure event. Expected closure during final exit is distinct from a live failure. A pending signal must not mask an already-fatal Lua/bootstrap error or rewrite its established error-file/traceback/exit-1 path into successful quit.

Close delivery/admission before Lua destruction on final exit; dispose of late wakes/results without Lua access. A closed proxy is not permission to recreate a failed/retired event loop. Do not wait for signal listeners, an unanswered chooser or user interaction to release the process. Keep borrowed process resources valid for in-flight work until completion/process termination, preserving the existing parent/runtime lifetime policy. This does not add a shutdown timeout, force-exit escalation or remote-dialog cancellation promise.

#### Migration cutover and alternatives

Move signal ownership in the **same separately shippable step as the native event-pump/queue cutover**. Do not ship a listener wired only to a nonexistent winit loop in the current SDL-only build. At that cutover, install the selected listeners before any remaining SDL event initialization so SDL's default-only installation skips captured signals, or remove that SDL initialization with the replaced pump. Ensure one signal-to-quit producer, not Tokio chaining an already-installed SDL handler and producing duplicate quits. No separate SDL wake bridge or generic dual-platform framework is selected. If safe coupling cannot be maintained, return for review rather than silently changing the mechanism.

The custom async-safe-handler/nonblocking-pipe/reader-thread alternative was considered but not chosen: it keeps Tokio lazy at the cost of another application reader thread and unsafe descriptor/handler lifetime work. Winit does expose AsFd; its existence alone does not establish that a bare external descriptor wait handles internal Wayland repeat deadlines. No such wait replacement, signal masking/signalfd design or new platform dependency is selected.

#### Required evidence, touch scope and stop conditions

Carry into [test design](36-design-sdl-free-contract-testing.md#resolution) and [the migration handoff](28-plan-sdl3-migration.md):

- Real child-process **SIGINT and SIGTERM**, separately, with default dispositions: clean exit/no error.txt and no update/draw after projects are removed, retaining the existing bounded startup check. Synthetic quit injection does not prove OS signal capture.
- Deliberately ignored and pre-existing custom dispositions, with the custom handler established inside the child before initialization, not incorrectly assumed to survive exec. Exercise partial/setup failure safely in isolated test subjects; no test signal may target the host/editor/session.
- Timed/indefinite native waits, pre-loop/failed-loop Rust state, signals racing notifier/proxy publication, pending/coalesced requests and non-consuming readiness, restart before delivery, and consumed/cancelled quit behavior. Use narrow test-only barriers/fault seams for deterministic races, then separate real signal evidence; no production scripting override.
- Outstanding chooser/final-exit and listener-failure paths without Lua reentrancy, lost wake, generation leakage or unbounded waits. Keep actual missing-display deliberate-Lua-error/traceback/error.txt/exit-1 checks; the signal runtime must not create a display or bus first. Maintain ordinary-build checks as well as hook-enabled runs.
- Measure early runtime/feature/thread/descriptor costs under the [existing startup/idle/frame/memory gates](31-calibrate-platform-cost-gates.md#resolution), including first-frame RSS and startup latency. This is an accepted design trade-off, not a cost waiver or measured pass. Apply [existing desktop dispositions](33-complete-platform-desktop-checks.md#resolution): private-Xvfb evidence is automated X11 only, native X11 remains accepted-unverified/reactive, and supported sandbox requirements remain conditional. Live desktop signal/wake checks require separate approval.

Touch scope includes [app startup/restart/teardown](../../../src/app.rs), the planned main-thread platform owner and Rust event/wake queue, [event conversion/waits](../../../src/sysevents.rs), [custom-event producers](../../../src/custom_events.rs), runtime/feature/build declarations, and [startup/contract runners](../../../scripts/rust-port/startup-check.sh). Keep the [Lua consumer](../../../data/core/init.lua) and public event interface unchanged. Document the earlier runtime start for implementation/cost accounting; do not change clipboard/chooser generation or parent behavior.

[Primary-source reference facts](../assets/process-signal-quit.md) cover the exact SDL 3.4.14 baseline, Tokio permanence/coalescing, descriptor/timer limits and EINTR retry caveats. Stop implementation for unsafe handler work, stolen/duplicated signal ownership, missed wake, lost process quit across restart, changed Lua behavior, broken diagnostics, unbounded shutdown or an unreviewed failed gate. Only planning/tracker files changed here: no dependency was adopted, handler installed, runtime built, signal sent, process killed, GUI/bus touched, test run, commit or push performed.
