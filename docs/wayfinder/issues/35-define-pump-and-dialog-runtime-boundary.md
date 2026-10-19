# Define the pump-loop and dialog-runtime boundary

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-pump-runtime
Blocked by: 24, 26, 27, 30

## Question

Who owns winit's main-thread event loop and windows, the translated Lua event queue, and ashpd's process-lifetime async runtime, and how do they cooperate without changing the existing poll/wait contract?

The [selected stack](27-select-platform-stack.md#resolution) retains Lua poll/wait through winit's pump API. Do not reopen a callback-driven redesign by default. The [loop prototype](30-prototype-loop-and-dialogs.md#resolution) demonstrated worker/dialog wake-ups through EventLoopProxy, and showed that ashpd's cached D-Bus connection cannot use a per-dialog runtime.

Settle runtime choice and ownership, initialization/shutdown, parent-window lifetime, dialog request/result routing by id, modality, and bounded wake-up behavior while Lua waits. Preserve immediate dialog entry-point returns followed by the existing dialogfinished event shape, including distinct accept/cancel/error outcomes. The portal-controlled Cancel label is the accepted difference; do not introduce an rfd/Zenity or SDL fallback.

Explain how the existing focus/queued-key quirks, coordinate conversions, event ordering and poll/wait timeouts survive. Account for the clipboard/primary service's dispatch and lifetime needs. Keep user data and non-Send window handles on appropriate threads, and define failure/teardown handling rather than silently dropping requests.

Consult codebase-design for the minimum useful interface and ownership boundary. Carry the existing desktop-validation dispositions and [calibrated startup/idle/frame/memory gates](31-calibrate-platform-cost-gates.md#resolution); no idle-spin fix or unmeasured performance improvement is assumed. Resolve with an integration decision and stop conditions for the handoff, not production code.

## Comments

### Live decisions — runtime and clipboard ownership

The user chose **“Reuse tested arrangement”** and **“Main-thread instance.”** Use one lazily initialized, process-lifetime Tokio runtime with one background worker (tested planning reference **Tokio 1.53.3**), and short-lived per-dialog request threads awaiting on it. Do not create/drop a runtime per dialog or copy the probe's one-outstanding-dialog restriction. The later [process-signal decision](37-choose-process-signal-quit-integration.md#resolution) supersedes this initial lazy-start preference when signal handling is required; process ownership and dialog execution remain unchanged.

Keep Lua execution, window creation and platform-queue consumption on the main thread. Dialog request threads send owned results through EventLoopProxy rather than invoking Lua. Use one lazy, persistent arboard instance directly on the main thread for the existing synchronous clipboard/primary calls; no worker request/reply channel or new clipboard-result events is selected.

Source checks distinguish the live prototype from unproved alternatives: winit's Linux Window is documented Send + Sync, but borrowed raw handles must remain on the acquiring request thread; ashpd creates the parent identifier there and the window must remain valid. Do not cast raw handles to Send or silently omit the parent. The clipboard bridge must preserve the actual SDL-facing behavior: a returned empty string is not zero Lua values merely because arboard reports ContentNotAvailable. Setters continue hiding backend failure.

### Live decisions — parent lifetime and Lua restart

The user chose **“Retain until completion”** and **“Use lifetime split.”** A pending dialog retains the native parent even if its Lua window is logically closed and hidden; the chooser may remain open, and its completion still reaches the original caller id while that Lua generation lives. Do not introduce cancel-on-parent-close or silently unparent a dialog.

Keep the event loop, runtime and clipboard owner across in-process Lua restarts. Tag each request/completion with the Lua generation and an internal request token independent of the caller's id. Retire the old generation before Lua teardown; discard its late results rather than deliver them to a new Lua state. Preserve concurrent calls and duplicate caller ids; no new public id validation. Neither restart nor final exit may wait indefinitely for user interaction. Final exit stops completion delivery before Lua destruction.

### Live decision — clipboard encoding

The user chose **“UTF-8 text only.”** This explicitly accepts a further clipboard/primary-text compatibility limit: the old Lua/SDL bridge does not validate arbitrary bytes, whereas arboard's text interface requires valid UTF-8. Backend-specific invalid-byte round trips have not been demonstrated; do not claim byte fidelity. Preserve first-NUL truncation, then validate without lossy replacement. Invalid-UTF-8 sets leave the selection unchanged and return no values; getter backend/non-text/conversion failures return an empty string. Empty valid input clears the selected clipboard/primary target; setters continue hiding failures.

The user subsequently chose **“Confirm runtime design”** for the complete proposal below.

### Resolution

The user confirmed the runtime, clipboard, parent-lifetime and Lua-generation policies after explicitly accepting the UTF-8 clipboard limit. This selects the integration design for planning; it does not adopt dependencies, implement the module or claim combined-editor acceptance.

#### Process owner and presentation seam

Keep one main-thread-owned platform module outside the `app::run` Lua restart loop. It owns winit's event loop, live-window registry, the pending owned-Rust-event queue and the process-lived service owners. Lua execution, window creation, event consumption and logical window close/hide stay on the main thread. The [selected pixel storage](34-choose-sdl-free-pixel-storage.md#resolution) stays window-owned and obeys its existing presentation seam; this decision adds no renderer redesign.

Native callbacks queue owned Rust data, never Lua values/pointers or Lua calls. They may also perform operations that require a live native callback, particularly the selected borderless drag/resize operations using Rust-held region/control state. Do not defer those operations until the live mouse callback has expired. Operations requiring an ActiveEventLoop, including creation at the appropriate lifecycle callback, use Rust-only handling; a synchronous Lua window call may stage/pump such work without re-entering Lua. Do not create a new generic command/adapter framework for ordinary window methods.

#### Dialog execution and result delivery

- Reuse the tested **Tokio 1.53.3** arrangement: keep one application-owned process-lifetime runtime with **one async worker**, IO/time enabled, and await each dialog on a short-lived request thread using that runtime. Initialize according to [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md#resolution), which revises the original lazy-start timing and adds signal support without changing runtime ownership. No per-dialog runtime, LocalSet dispatcher or global single-pending restriction is selected. Tokio's worker count is not a claim that libraries create no helper/export/blocking threads; account for them in measurements.
- Obtain borrowed raw window/display handles on the request thread from a retained, documented-thread-safe Linux winit Window. Construct/await the parent identifier on that same thread; do not move raw borrows across threads, mark pointers Send, or put such a future into a Send-only spawn. Retain the window/display/export lifetimes through the portal operation. The probe and pinned source support this arrangement; they do not establish all production teardown cases.
- Dialog entry points preserve their existing argument handling and immediate zero-value return. Each invocation gets an internal token and Lua generation, separate from its unchanged caller-supplied id. Preserve concurrent requests and duplicate caller ids; do not serialize all dialogs behind one open chooser or add new public id validation. Honor the selected filters, location, custom Accept text and parent modality; the accepted Cancel-label difference stands.
- Send one owned terminal result for each admitted request in the live generation through EventLoopProxy. The main thread queues the existing `dialogfinished` event and constructs Lua values only through normal polling. Do not call Lua from a worker/native callback, deduplicate by caller id, or invent a public worker/clipboard-result event. Publish the payload as part of the wake operation, or enqueue before waking and check before sleeping, so a completion cannot be stranded by a lost wake.
- Preserve the existing event shapes: accept has the path list, cancel has no fourth value, error has the existing message payload where available, and the reserved unknown status remains representable. Thread/runtime/parent-export/transport/conversion failures must take the recoverable error path rather than hang, masquerade as cancellation, or launch an unparented/rfd/Zenity/SDL replacement. Do not cache a transient failed runtime initialization as a false successful service; subsequent requests must fail honestly or retry safely without replacing a successfully cached runtime.
- Match the SDL-facing portal outcome quirks: response codes 1 and 2 map to cancellation, and an otherwise successful empty selection also cancels; ordinary transport/protocol/conversion failures error. Distinguish ashpd's response-code errors from those failures rather than mapping every library error identically. Convert selected local-file URIs to owned native paths without lossy filename conversion, silent entry dropping or partial acceptance. The clipboard UTF-8 limit does **not** narrow native filename bytes. Validate malformed/unrepresentable payloads safely; verify valid local-host/percent-decoding/path cases against the baseline rather than assuming a URL helper is byte-identical.

#### Parent closure, Lua restart and final exit

A dialog retains its native parent through completion. If the Lua window closes, hide/logically retire it from live input routing while retaining the native handles needed by the request. **The chooser may outlive its visible parent.** Deliver the original id/outcome while that Lua generation lives; parent closure alone is not grounds to discard it or synthesize cancellation. No new cancel-on-close mechanism is selected.

On Lua restart, retire the generation and clear its pending delivery before `lua_close`; native events from retired windows and old completions cannot enter the next Lua state. Keep the winit event loop, cached runtime and clipboard owner across the reset. Do not call winit's final exit or recreate/drop the cached D-Bus runtime merely to restart Lua. Keep any outstanding request's handle leases valid until it finishes, even though its eventual result will be discarded.

On final exit, close delivery/admission before Lua destruction. Late sends are disposed of without touching Lua. Do not block exit or restart waiting for a user to finish a chooser, and do not promise that dropping a future closes the remote portal dialog. Process-lifetime resources remain valid for in-flight work until that work ends or the process terminates; do not tear down a display/runtime still borrowed by a detached request. No helper daemon or new shutdown timeout/configuration is selected.

#### Poll/wait and event semantics

Use one owned Rust queue retaining the information needed for the [observable platform contract](24-inventory-platform-contract.md), including non-emitting markers. A queue containing only preformatted Lua events is insufficient: focus gained can make wait ready but produce no Lua event while discarding queued key presses.

- `poll_event` uses a zero-timeout pump at safe Rust points and consumes queued events until it can return one existing Lua event or no values. Release mutable state borrows before pumping callbacks. Keep current queue filtering/coalescing, event order, mouse/drop/touch units and rounding; do not normalize the recorded quirks.
- `wait_event` is **non-consuming readiness**, not a second event-dispatch interface. Existing queued SDL-equivalent events, including ones subsequently suppressed by polling, can satisfy it. Leave them for `poll_event`; internal scheduling/proxy notifications alone must not invent Lua events or false public readiness.
- Preserve negative-timeout clamping and the current `(seconds * 1000.0) as i32` conversion, including sub-millisecond zero and cast edge cases. No argument means the existing indefinite wait. Use a single monotonic deadline across internal wake-ups rather than restarting the timeout. Recheck queue/closure state before blocking; preserve the false-on-timeout/error result.
- Preserve focus-gained suppression and discarding of queued key presses without dropping unrelated releases, text, dialog results or other events. Existing motion coalescing and coordinate conversions belong in the same tested queue/translation path. Keep callbacks active while a real dialog is open.
- Do not fix the documented focused idle spin, raise short timeouts, add periodic worker polling, or impose a callback-driven Lua loop. Native/proxy wake-ups interrupt the existing waits; startup/idle/frame/memory behavior still has to pass the agreed gates.

#### Clipboard and primary selection

Use one lazy, process-lived arboard instance on the main thread, independent of Lua generations and the dialog runtime. Its synchronous calls retain existing Lua argument coercions and return shapes; no clipboard worker or asynchronous public event is selected. Use arboard's transport selection/library dispatch without mutating global backend environment variables after threads have started, and report the actual transport in backend-specific validation rather than inferring it from the window alone. Do not claim selection survival without a manager.

**Explicitly accepted compatibility limit:** clipboard/primary text is valid UTF-8 only. For setters, first apply existing NUL truncation and argument coercion, then validate without replacement. Invalid UTF-8 leaves the selection unchanged and returns no values; backend/init failures remain hidden as before. Empty valid input clears only the requested clipboard/primary target using the appropriate library operation. For getters, return valid text with existing first-NUL truncation; ordinary missing/non-text/backend/conversion failures yield `""`, not an invented nil/event. Preserve any specifically demonstrated exceptional null-return contract rather than conflating it with normal arboard ContentNotAvailable. This exception does not authorize lossy filename or filesystem conversions.

#### Required checks and stop conditions

Carry this design into [Design SDL-free platform contract testing](36-design-sdl-free-contract-testing.md) and [Define the SDL3 replacement handoff](28-plan-sdl3-migration.md). Required cases include:

- poll versus non-consuming wait, queued suppressed events, focus-key flushing, motion/unit quirks, zero/negative/sub-ms/finite/indefinite waits, spurious wake-ups and completion races without timeout extension;
- synchronous window creation/native-handler operations without Lua reentrancy; real OS and dialog wake paths remain distinct from injected queue tests;
- successive and concurrent dialogs, duplicate caller ids, at-most-once delivery per internal token, init/spawn/export/portal failures, cancel/empty selection/error mapping, faithful local path bytes and parent modality;
- a hidden/closed parent with a live Lua generation, restart during an outstanding request followed by caller-id reuse, discarded old native events/results, and final exit while the chooser is unanswered;
- clipboard and primary separately: coercion, empty/failed reads, clear, embedded NUL, invalid UTF-8 before/after truncation, unchanged selection after an invalid set, hidden failure and persistence across Lua restart;
- library/thread/lease/queue teardown and repeated-use memory, lazy-service startup and first-use latency, focused/unfocused idle, active/copy/presentation timing and wake latency under the [calibrated gates](31-calibrate-platform-cost-gates.md#resolution).

A source-supported design is not a compiled or live acceptance pass. Keep [all desktop-validation dispositions](33-complete-platform-desktop-checks.md#resolution), including conditional sandbox checks and native X11's accepted-unverified status; do not turn them into passes or new planning blockers. A deadlock, stale-generation delivery, incorrect event/path outcome, unsafe handle lifetime or failed gate stops implementation for evidence/review, not a silent redesign, lossy conversion or waiver. The test architecture remains undecided until its own HITL ticket.

#### Evidence and touch scope

Local contract/source inspection: [application Lua restart/teardown](../../../src/app.rs), [event polling, waits and coordinate/coalescing quirks](../../../src/sysevents.rs), [dialog entry points and callback values](../../../src/sysplugin.rs), [synchronous clipboard bridge](../../../src/system.rs), [window ownership](../../../src/renwindow.rs), [core event/dialog consumers](../../../data/core/init.lua), and [system contract cases](../../../tests/contract/cases/system.lua). Include startup, custom-event plumbing and test-only injection users when implementing the seam; replacing only the dialog function is incomplete. The local-only [loop prototype](30-prototype-loop-and-dialogs.md#resolution), commit `4b275ae7` on `prototype/sdl3-loop-probe`, supplies the tested runtime/request arrangement, not the full generation/clipboard-main integration.

Pinned library sources inspected: [winit Window thread/handle contract](https://github.com/rust-windowing/winit/blob/v0.30.13/src/window.rs), [ashpd parent identifiers](https://docs.rs/ashpd/0.13.13/src/ashpd/window_identifier/mod.rs.html), [ashpd response/request preparation](https://docs.rs/ashpd/0.13.13/src/ashpd/proxy.rs.html), and [arboard text/clear operations](https://docs.rs/arboard/3.7.0/src/arboard/lib.rs.html). SDL source comparisons used [3.4.14 clipboard implementation](https://github.com/libsdl-org/SDL/blob/release-3.4.14/src/video/SDL_clipboard.c), [3.4.18 portal responses](https://github.com/libsdl-org/SDL/blob/release-3.4.18/src/dialog/unix/SDL_portaldialog.c) and [its URI helper](https://github.com/libsdl-org/SDL/blob/release-3.4.18/src/SDL_utils.c). The bindings' tag does not establish the linked runtime's version. In the inspected 3.4.18 source the URI helper returns -1 on unsupported inputs while the portal caller tests truthiness; do not copy unsafe malformed-URI handling or cite that call site as proof of safe validation. Valid-path parity requires tests; undefined behavior is not a compatibility feature.

Only planning/tracker records change here. No production code, dependency installation, desktop/clipboard action, upstream report, commit or push is authorized by this resolution.
