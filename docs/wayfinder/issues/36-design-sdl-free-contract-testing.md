# Design SDL-free platform contract testing

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-contract-testing
Blocked by: 26, 27, 33, 34, 35

## Question

How will the contract suites inject deterministic events, wake blocked waits, create draw targets and read pixels after SDL's event queue and dummy/offscreen drivers are removed, without pretending that synthetic input proves the real OS path?

Choose the minimum test seam consistent with [Choose SDL-free pixel storage and presentation conversion](34-choose-sdl-free-pixel-storage.md#resolution) and [Define the pump-loop and dialog-runtime boundary](35-define-pump-and-dialog-runtime-boundary.md#resolution). Decide whether existing test-only hooks suffice or a headless implementation is necessary, and specify which parts are shared with real event translation, queuing, drawing and teardown. Keep test-only facilities out of normal user behavior and avoid a second, divergent platform implementation.

Preserve the existing contract suites and hit-test/startup checks. Test hooks may be adapted; public Lua behavior and deterministic image expectations may not silently change. Whole-window opacity tests must retain the API's argument handling/true return while treating the effect as an explicitly approved no-op, not a pending feature. Internal drawing alpha remains covered.

Carry the [pixel-storage resolution's required checks](34-choose-sdl-free-pixel-storage.md#resolution) into the chosen test architecture. Its raw-storage/presentation assertions are required alongside the existing RGB-only hooks; unchanged RGB hashes alone do not prove valid presentation alpha. This constrains coverage, not the still-open harness/adapter design.

Also carry the [runtime resolution's queue, restart, dialog and clipboard checks](35-define-pump-and-dialog-runtime-boundary.md#resolution). The explicitly accepted UTF-8-only clipboard limit and hidden-parent/generation lifetime policy must be reflected in test expectations rather than treated as unapproved regressions or silently omitted. Keep raw clipboard bytes distinct from native filename bytes, and injected completions distinct from OS/portal evidence. This still does not select a headless adapter or test implementation.

Apply [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md#resolution), the [selected stack](27-select-platform-stack.md#resolution), and [Complete the outstanding window and input desktop checks](33-complete-platform-desktop-checks.md#resolution). Keep automated and live evidence separate; retain deferred, conditional and accepted-unverified dispositions rather than turning them into passes or new planning blockers. The existing process timing failure still needs classification before any all-passing claim.

Consult codebase-design when choosing a test seam. Resolve with the test architecture, suite/hook adaptation outline, required live checks and stop conditions for the implementation handoff. Do not implement or run a new desktop test as part of this decision without separate approval.

## Comments

### Live decision — automated display environment

The user chose **“Private Xvfb.”** Run the automated suites against real winit windows on a private Xvfb display instead of creating a separate display-free native-window implementation. Share production window/userdata, drawing and event processing, retaining narrow deterministic test hooks. The runner must not inherit the developer's display connection. This is automated X11 evidence only, not a Wayland, fractional-scale or native-session desktop pass. The previous window prototype already has bounded private-Xvfb smoke evidence; Xvfb is on this host's PATH, but the migrated editor has not been tested.

This selection does not approve baseline regeneration or determine the input/external-result injection seam. The subsequent live decisions and final shared confirmation below settle these points.

### Live decisions — injection, external results and baseline policy

The user chose **“Shared pre-Lua path,” “Narrow isolated seams,”** and **“Preserve and review.”** Synthetic records enter the production owned-input translation and queue before Lua conversion, not as finished Lua tuples. Check native-to-owned capture separately and retain actual OS-path evidence; synthetic records alone do not cover capture.

Use real clipboard/primary round trips only on the private display, and a real unavailable-portal check in an isolated process. Script only portal transport/results and narrowly targeted failure points, retaining production request validation, conversion, generation filtering, worker/proxy delivery and polling. Ordinary filesystem/process/plugin/directory-monitor checks stay real. No host clipboard, chooser or bus access is approved.

Keep historical goldens and use a separately pinned working SDL/Rust reference for matched comparisons; retire the obsolete C-recording path. Deterministic contract outputs and RGB hashes stay strict. Explain/review environment-dependent observations and native error wording before changing expectations; no blanket normalization or automatic re-recording is approved.

### Source findings — baseline provenance and process signals

The current [system module](../../../src/system.rs) creates a fresh Lua table; `LXL_TEST_C_EVENTS` now returns before registering window/dialog/event functions rather than retaining a surviving C implementation. The [runner's RECORD mode](../../../scripts/rust-port/verify.sh) still sets that obsolete flag. Preserve the historical goldens, but do not claim that today's binary can regenerate a C baseline; replacement provenance/recording policy is part of this decision. This is static inspection, not a run or repair.

The real SIGINT check in [startup-check.sh](../../../scripts/rust-port/startup-check.sh) exposes a separate production integration choice: SDL currently supplies signal-to-quit handling that winit does not automatically replace. [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md) now owns that precise decision and blocks the handoff. This ticket retains real subprocess coverage; it does not select a signal mechanism.

The same startup check requires the deliberate Lua error/traceback and `error.txt` even without a display. [Preserve fatal-error reporting without SDL](38-choose-sdl-free-fatal-error-reporting.md#resolution) owns the subsequent reporting and display-initialization failure decision. The test architecture preserves that actual startup path rather than masking it with Xvfb or a fake fatal-report function.

### Resolution

The user chose **“Confirm test design”** after selecting private Xvfb, the shared pre-Lua path, narrow isolated seams and preserved/reviewed goldens. This fixes the test architecture, not an implementation or acceptance result. No test, private display or desktop interaction was run to resolve it.

#### Private display, not a second platform implementation

Run the existing contract runtime in the real editor process with real winit windows on a runner-owned **Xvfb** display. Retain the production window userdata/ownership, draw targets, coverage atlases, queue, translation and lifecycle code. Do not build a display-free window adapter, SDL-surface shim or generic mock platform.

The runner must reserve its own display and authentication/resources, disable unintended external access, and set child environment before starting library threads. Do not inherit the host DISPLAY, Wayland connection/socket, session-bus route or clipboard transport: use a private runtime directory and unreachable/private bus address, force the intended X11 window path, and verify clipboard traffic uses the private display rather than assuming it from the window backend. Xvfb and any required runner/authentication tooling are test dependencies, not editor runtime dependencies. The absence of `xvfb-run` on this host is not a blocker: Xvfb itself is available. Use bounded readiness/timeouts and cleanup only runner-owned resources; fail rather than silently use the developer's display.

Use explicit scale-1 display geometry/font inputs for legacy comparisons and record actual physical extents/density. Synthetic scale cases are arithmetic/translation checks, not genuine fractional-display evidence. Xvfb proves neither Wayland nor native-session X11/WM behavior. Keep the standard build's missing-display failure path real; there is no automatic headless fallback.

#### Input, wake and lifecycle seam

Adapt `lxl_test.push_event` and its fixtures to feed owned input records **before production Lua conversion**, through the same translation/queue functions native callbacks use. Remove SDL event-number dependence where useful rather than retaining an SDL event emulator. Do not enqueue final Lua tuples or copy key-name, coordinate, focus or coalescing logic into hooks.

Keep semantic transformation in shared production functions. Test native-to-owned capture with constructible native cases where possible, plus actual OS-path checks for library-private/nonconstructible events and backend behavior. Record the uncovered capture cases; passing injected records does not prove callback delivery. Hooks may change their private interface without changing the public Lua results.

Exercise non-consuming wait readiness (including suppressed focus markers), queue ordering and key flushing, coalescing, mouse/drop/touch units, all existing timeout conversions and a single deadline across internal wake-ups. Check timeout arithmetic with supplied instants/inputs where practical, and separately run bounded real waits woken by actual worker/EventLoopProxy delivery on the private display. A direct insertion into the pending queue is not wake-up evidence. Add restart/caller-id-reuse races, concurrent/duplicate-id requests, closed-but-retained parents, old-result disposal and unanswered-request exit checks from the [runtime resolution](35-define-pump-and-dialog-runtime-boundary.md#resolution). No Lua reentrancy, periodic worker polling or idle-spin fix is selected.

#### External transport and fault cases

Keep filesystem, process, native-plugin, directory-monitor and ordinary system checks on their real implementations. Run clipboard/primary set/get/clear against arboard on the private display, independently per selection. Cover the accepted UTF-8/NUL/failure behavior and persistence of the owner across Lua restart; do not use this to claim cross-application Wayland interoperability or post-exit selection survival.

Use a **test-only scripted portal transport** for accept/cancel, URI payloads, failures and controlled delayed replies. Substitute the unavoidable external reply, not the entire Lua dialog function or the production outcome converter. Keep argument/options validation, request admission/tokens, parent retention, URI/status mapping, generation filtering, actual request-worker/proxy delivery and poll-time Lua packing in the production path. Supply malformed/unsupported replies safely; verify native filename bytes independently of clipboard's UTF-8 limit. Observe returned outcomes and lifetime effects, not private call sequences.

Use focused fault points only where real failures cannot safely/deterministically be induced (runtime init, spawn, export, transport/clipboard failures); make their scopes explicit and avoid order-dependent cached state between cases. Also run the **real ashpd unavailable-portal path** in a separate fresh process with an unreachable bus. Fake replies are not proof of actual portal permissions, modality, export on both backends or sandbox operation. Normal automated runs must never open a host chooser/message box or read/replace the host selections.

#### Drawing and presentation checks

Adapt existing `lxl_test.pixel`, `surface_hash` and `dirty_rects` to the actual retained draw target/rencache output. Preserve their RGB behavior and deterministic scenes. The existing RGB-only hashes do not cover alpha or presentation storage.

Add direct Rust assertions through the actual drawing/storage/copy functions for the [storage resolution's required cases](34-choose-sdl-free-pixel-storage.md#resolution): opaque initialization and post-drawing alpha, channel packing, observed SDL rectangle rounding, odd-width/padded rows and sentinels, rotating destinations with unrelated prior contents, changed-region full copies, unchanged-frame skipping versus forced refresh, and resize/invalidation/minimize/restore/owner teardown. Test destination rows with explicit buffers where useful; do not implement a second drawing or copy algorithm as the oracle. Pin expected bytes/rounding from independent baseline evidence rather than regenerating them from candidate output. Backend presentation on Xvfb remains separate from these assertions and from visible Wayland/editor parity.

#### Suites, startup and baseline provenance

Retain **all 16 suites**, hit-test comparisons and startup checks. Update test-only fixtures and reference plumbing, not public Lua behavior. Keep real subprocess SIGINT/clean exit, persistent-window Lua restart, deliberate Lua-load failure without DISPLAY/Wayland/bus, traceback/error.txt/exit status, and no update/draw after projects are removed. The signal and fatal-reporting mechanisms are deliberately left to their own tickets below; these checks must not be weakened to conceal either gap.

Preserve existing historical goldens as evidence. Use a separately identified **SDL/Rust reference**, not the obsolete `LXL_TEST_C_EVENTS` path, for matched candidate comparisons. The source reference selected here is current unchanged implementation commit `c80459bc0f464df68b8d8cdff553cd8288519a67`, with Cargo.lock SHA-256 `1a2b1485592910d3bc588c04d8a7f12f0d285b9997a6cab91eac66a718f72a35`. This pins source/lock only; no new reference binary has been built or certified. Record features, binary and fixture identities, actual linked SDL/runtime versions and display/transport settings when building/running it. The historical C outputs remain historical C evidence; today's Rust binary must not be described as capable of selecting a surviving C implementation.

Compare like environments: retain legacy offscreen evidence and collect matched SDL/Xvfb reference observations when judging the private-display candidate. Keep deterministic Lua outputs, text geometry, dirty rectangles and RGB hashes strict. Explain every difference. WM/display observations and native driver/portal error wording may need targeted profile-specific assertions or expectations, but require explicit reviewed diffs; do not blanket-normalize errors, discard event order or re-record after a failure. Preserve the approved opacity no-op, portal Cancel label and UTF-8 clipboard exceptions without inferring new ones. Rerun/classify the pre-existing process timing failure before any all-passing claim. A crash, timeout, incomplete suite or nonzero exit cannot become a golden.

#### Shipping exclusion, live evidence and handoff

Keep injection/scripted transport/fault controls behind the non-default `test_hooks` feature or Rust-only test configuration. A normal build must not expose `lxl_test`, accept scripted/fault environment overrides or silently switch transport/platform behavior when a display is unavailable. Build/check the ordinary configuration separately from the hook-enabled contract executable; hooks alone cannot establish shipping behavior. Execute future adaptations in small behavior-checked increments, not by generating a parallel test implementation.

Carry the existing [desktop-validation dispositions](33-complete-platform-desktop-checks.md#resolution): real full-editor Wayland/XWayland OS-event, IME/shortcut, pointer/scale, dialog/clipboard/drop and presentation checks remain distinct; native X11 stays accepted-unverified/reactive, hardware touch stays unverified until available, sandbox checks remain conditional, and no-manager selection validation stays isolated rather than disturbing Klipper. Do not infer these passes from Xvfb, scripts or injected records, or reinstate deferred items as planning blockers. Apply the [calibrated startup/idle/frame/memory gates](31-calibrate-platform-cost-gates.md#resolution), including copy/presentation accounting, in matched real-editor runs, not as Xvfb performance certification.

Adaptation reaches [the runner](../../../scripts/rust-port/verify.sh), [startup checks](../../../scripts/rust-port/startup-check.sh), [hit-test checks](../../../scripts/rust-port/hit-test-check.sh), [contract runtime](../../../tests/contract/contract.lua), its case fixtures and preserved baseline artifacts; [test hooks](../../../src/test_hooks.rs), [system registration](../../../src/system.rs), application initialization, input/custom events, window/userdata, rendering/readback and dialog/clipboard transport seams; plus non-default feature/test-tool declarations. The [migration handoff](28-plan-sdl3-migration.md) owns concrete step/build ordering and must carry these checks. No source/script/baseline/dependency edit is implemented here.

The investigation exposed two precise, separately owned decisions: [Preserve fatal-error reporting without SDL](38-choose-sdl-free-fatal-error-reporting.md#resolution), then [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md#resolution). The latter depends on the former's initialization policy; their mechanisms are not selected by this test design. Carry the subsequent signal resolution's real SIGINT/SIGTERM, previous-disposition, pre-loop/proxy/restart race, listener-failure and early-runtime cost checks; do not substitute synthetic quit or signal the host session. Carry the subsequent reporting resolution's private-bus/fake-daemon payload, deadline, encoding and failure checks alongside the real no-display startup path and process-isolated native-init cases. No host notification is permitted during automated contract runs.

Stop implementation for an unexplained contract/image difference, unsafe transport leak, missed wake/stale-generation delivery, inaccessible normal-build error path or failed acceptance gate. Report incomplete/unverified cases explicitly; no automatic golden refresh, fake-only proof or hidden SDL fallback is approved.
