# SDL3 replacement implementation handoff

**Status: planning complete; implementation not started by this handoff.** Start execution only under a separate implementation instruction. Builds, installation, desktop interaction, clipboard changes, signals and commits/pushes are not authorized by this document's creation.

## Authority and starting point

The destination is [Replace SDL3 with Rust-native platform crates](../sdl3-map.md). This handoff orders work already selected by its resolutions; it does not introduce another platform framework, approve new compatibility losses or certify the combined editor. The linked resolutions are normative. If implementation evidence contradicts them, stop with a precise question rather than silently redesigning.

Read these before their affected step:

- [Define platform compatibility and acceptance criteria](../issues/26-define-platform-acceptance.md#resolution) and [Calibrate platform performance and memory gates](../issues/31-calibrate-platform-cost-gates.md#resolution).
- [Select the Rust-native platform stack](../issues/27-select-platform-stack.md#resolution), [Qualify Wayland CPU presentation without unsupported pixel padding](../issues/32-qualify-wayland-presentation.md#resolution), and [Complete the outstanding window and input desktop checks](../issues/33-complete-platform-desktop-checks.md#resolution).
- [Choose SDL-free pixel storage and presentation conversion](../issues/34-choose-sdl-free-pixel-storage.md#resolution), [Define the pump-loop and dialog-runtime boundary](../issues/35-define-pump-and-dialog-runtime-boundary.md#resolution), and [Design SDL-free platform contract testing](../issues/36-design-sdl-free-contract-testing.md#resolution).
- [Preserve fatal-error reporting without SDL](../issues/38-choose-sdl-free-fatal-error-reporting.md#resolution) and the later [Choose process-signal quit and wake integration](../issues/37-choose-process-signal-quit-integration.md#resolution). The latter supersedes always-lazy **Tokio** initialization, not lazy display initialization.

The implementation reference remains **`c80459bc0f464df68b8d8cdff553cd8288519a67`**, with Cargo.lock SHA-256 **`1a2b1485592910d3bc588c04d8a7f12f0d285b9997a6cab91eac66a718f72a35`**. This identifies source/lock, not a newly certified reference binary. Current production still uses SDL. No production C implementation survives behind `LXL_TEST_C_EVENTS`; the C plugin fixtures are not an alternate editor. Preserve the sixteen historical goldens.

## Reviewed differences and invariants

Document these four reviewed differences in configuration/interface documentation and test expectations, following their owning resolutions:

1. Whole-window opacity is an intentional no-op with existing argument handling and `true` return; remove functional-opacity configuration/UI if present. Do not recover it. Internal drawing alpha remains required.
2. Portal dialogs control the Cancel label; custom Accept text, filters, parent modality and accept/cancel/error distinctions remain.
3. Clipboard/primary text is UTF-8-only: truncate at NUL first, never replace lossily, leave the selection unchanged on invalid sets, hide setter failure, and return `""` for ordinary failed/non-text gets. Native filename bytes are **not** narrowed by this exception.
4. Fatal reporting becomes bounded best-effort critical **nonmodal** native desktop notification, not a blocking box. Keep diagnostics and Lua coercion/NUL/zero-return behavior; daemon acceptance is not presentation.

Also carry the explicitly reviewed early-runtime timing and app-side borderless-zone mechanism. Neither permits losing hit-region/coordinate quirks, title controls, normal quit confirmation or other Lua behavior. Freeze the [complete observable inventory](platform-contract-inventory.md): 41 system functions, four window functions and 26 event shapes, not merely the cases currently covered by tests. Linux remains the supported target; restoring other platforms is out of scope.

## Dependency and build plan

Adopt these only in the steps below, with Cargo.lock updated and locked builds checked:

| Dependency | Planning reference and feature constraints |
| --- | --- |
| libloading | `=0.8.9` as the cached implementation reference for the already-selected loader; compile/behavior validation belongs to Step 2, not a claimed prototype pass. |
| winit | `=0.30.13`, Wayland **and** X11, retaining tested defaults/raw-window-handle support. No GPU renderer or forced fallback to another protocol. |
| softbuffer | Git revision **`9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`** from the upstream softbuffer repository; its package version says `0.4.8` but this is **not released 0.4.8**. Retain tested `wayland`, `wayland-dlopen`, `x11`, `x11-dlopen` with default features disabled. |
| arboard | `=3.7.0`, default features disabled, `wayland-data-control`; text-only feature set was exercised by the probe. |
| ashpd | `=0.13.13`, `file_chooser`, `raw_handle`, `wayland`, with the Tokio executor selected consistently with zbus. Native fatal notifications use its public zbus/zvariant re-export; no portal-notification feature/helper is needed. |
| Tokio | `=1.53.3`; retain tested `rt`/`rt-multi-thread`, `net`, `time`, adding `signal` and the required `sync` notifier facilities. IO/time are also enabled on the runtime builder; there is no Cargo feature named `io`. Signal/notifier integration is selected but not prototype-certified. One worker serves signals, dialogs and reporting. |

Do not add rfd, Zenity, notify-send, native libnotify, a second executor, an SDL fallback or a generic adapter framework. Retain existing font/regex/Lua dependencies and libc.

`flake.nix` currently has `cargoLock.lockFile`, `doCheck = false`, `nativeBuildInputs = [ pkg-config ]`, and the large `sdlDeps` list in both package and shell. Add the exact git-source vendor hash through the pinned nixpkgs buildRustPackage mechanism (`cargoLock.outputHashes` when required); calculate it from the adopted lock, never invent a value or use an unpinned git checkout. Keep checks outside the sandbox unless deliberately integrated; `doCheck = false` is not evidence that checks passed.

Separate runtime providers from test tools. Preserve Wayland/libxkbcommon and the X11/XCB providers actually linked/dlopened by the chosen features, including eager Xlib loads on the Wayland path. Dlopen libraries need to work **outside the development shell** through the built executable's native search/runpath setup; merely listing packages in a shell is insufficient. Retain only demonstrated providers, not SDL's entire audio/USB/GL stack by inertia. Test Xvfb/private dbus-daemon and existing compiler/Lua/coreutils tooling belong in the development/check environment, not the shipping runtime closure.

Keep `scripts/install-linux.sh`, desktop/icon/data paths, `EXEFILE`, `LITE_PREFIX` and the `bin/lite-xl` executable identity. No executable wrapper/renaming is selected. Build.rs's existing icon extraction and version/architecture generation are not SDL and need no replacement. Preserve Linux package declarations for both existing architectures without claiming unperformed architecture tests.

## Sequence and dependencies

Execute **in order**. Each step leaves a working default editor and has its own review/evidence checkpoint. Implementation can use internal commits, but must not publish a broken midpoint of the coupled cutover. SDL remains permitted until the final removal step.

### Step 1 — Reference and isolated test preparation

**Touch:** `scripts/rust-port/verify.sh`, `startup-check.sh`, `hit-test-check.sh`, relevant contract fixtures/runtime, the obsolete switch in `src/system.rs`, stale `src/test_hooks.rs` comments, and test-tool declarations in `flake.nix` if needed. No production backend change.

- Build the pinned SDL reference in a separate worktree/target/stage, without overwriting the developer's install or unrelated `stage/`. Record source/lock/binary/features, fixture/font identities and actual SDL runtime version; 3.4.18 bindings do not prove the linked runtime version. Preserve legacy offscreen results separately.
- Prepare a runner-owned private Xvfb profile for matched SDL/reference and later winit/candidate runs. Select the private display itself, with readiness timeout and cleanup of **owned PIDs only**. Do not inherit the desktop's Wayland connection/runtime directory; use a private runtime directory, explicit display selection and unreachable/private session bus. No host clipboard, portal, chooser or notification access. Record the selected backend rather than infer it.
- Each suite remains a fresh bounded process. Retire `LXL_TEST_C_EVENTS`, its special recording path and C-only fixture branches; it currently omits Rust functions rather than selecting C. Do not regenerate goldens. Any reference-recording workflow must identify the reference and require explicit review before replacing expectations; a crash/nonzero result can never become a golden.
- Collect matched SDL/Xvfb observations before judging profile-specific window/WM/error differences. Review narrowly explained expectations or invariants per the test resolution; no blanket normalization or suppression of event order. Preserve deterministic outputs/RGB hashes strictly.
- Reproduce and classify the already-recorded process timing failure against the reference before an all-passing claim. Keep its evidence distinct from candidate regressions; neither an unexplained failure nor a blanket timing waiver is a passing baseline.
- Capture independent rectangle-rounding, pixel, multi-click state/count and classifier expectations before removing their SDL reference. Preserve the hit-test check as an SDL-free candidate check later; the legacy SDK-based comparator can remain external reference evidence, not a shipping dependency.

**Must prove:** all sixteen suites are accounted for (`customevents`, `dirmonitor`, `fuzzy`, `pluginapi`, `process`, `regex`, `rencache`, `renderer`, `smoke`, `sysevents`, `sysplugin`, `system`, `syswindow`, `tokenizer`, `utf8`, `utf8_edge`), alongside hit-test and startup checks. Verify isolation and failure propagation, normal-build exclusion of hooks, and provenance. No new desktop review is needed for this test-only preparation.

**Stop:** missing reference identity, unsafe inherited transport, obsolete C-baseline claims, unexplained expectation changes or an unclassified failure presented as a pass.

**Implementation status (2026-10-21, uncommitted):** implemented; see [Step 1 evidence](sdl3-step1-evidence.md). The process timing failure is reproduced and classified. One review is pending: the matched private-Xvfb differences in five suites need the user's disposition before Step 4 judges an Xvfb candidate.

### Step 2 — Independent non-window ports

**Touch:** `src/system.rs` filesystem/time functions, plugin-loader portion of `src/sysplugin.rs` and handle GC in `src/system.rs`, command storage in `src/renwindow.rs`/its `src/rencache.rs` callers, `Cargo.toml`, Cargo.lock, and corresponding tests. SDL windows/events/surfaces still work unchanged.

- Replace loading with libloading, preserving `package.native_plugins`, handle ownership/GC, duplicate-load behavior, entrypoint-name selection and both native-plugin entry conventions. Keep the library alive as long as its existing owner, drop exactly once, and retain resolver/interface behavior. Do not leak it to sidestep lifetime work or change the native-plugin ABI/header.
- Move timing/sleep to std: one process-lived monotonic anchor, floating seconds, existing negative clamp and `(seconds * 1000.0) as u32` sleep conversion **before** constructing a Duration. Do not reset time on Lua restart or introduce panics for casts previously handled by the Rust bridge.
- Move directory enumeration/removal to std::fs without sorting/normalizing, losing native path bytes or changing file-versus-empty-directory/symlink behavior. `rmdir` also removes files. Preserve error/return shapes and reviewed wording.
- Replace the command-buffer allocator with Rust-owned storage, updating every growth/free/teardown user together. Never pair Rust allocations with SDL_free. Keep command layout/hashing and replay unchanged.

**Deliberately defer:** `setenv`/sandbox classification, font SDL error transport, clipboard, dialogs and fatal UI until the complete cutover. A libc-only setenv while SDL still reads its own environment snapshot can change behavior; do not ship that partial conversion.

**Must prove:** targeted system/pluginapi/sysplugin plus renderer/rencache checks, large command-buffer growth/drop checks, then the full contract/startup/hit-test set. Keep loader error/entry/GC cases and malformed/native-byte path cases. Default SDL window/presentation remains usable. Track allocations; do not assert a cost improvement from deletion alone.

**Stop:** loader lifetime/ABI regression, changed deterministic output/error handling, allocator mismatch or unexplained drawing difference.

### Step 3 — Retire the alternate SDL renderer build

**Touch:** `Cargo.toml`'s `sdl_renderer` feature, its cfg branches in `src/renwindow.rs`, `src/renderer.rs`, `src/renderer_api.rs`, and stale source comments (including `src/font.rs`) and script/build documentation advertising it. Keep `test_hooks` non-default.

Retire, do not port, the texture/SDL_Renderer alternative. Default software-surface operation remains until the next step. Do not recover opacity or remove internal alpha drawing with this cleanup.

**Must prove:** ordinary and hook-enabled builds, renderer/rencache hashes, all suites/startup/hit checks. The retired feature is no longer a supported build option; update its callers/examples rather than leaving a second path.

**Stop:** accidental change to the default renderer, broken supported build settings or rewritten expected images.

### Step 4 — Complete native platform, storage and service cutover

This is **one separately shippable cutover**. Its internal work is coupled; do not deliver a platform skeleton, a second SDL/window adapter or a listener wired to a loop that does not exist. Pixel storage moves here rather than through an unselected transitional SDL-surface compatibility shim.

**Build:** adopt the selected crate/feature set and git lock/vendor hash; add the measured native providers to `flake.nix` alongside SDL until final removal. Register the private platform module from `src/lib.rs`. Start with one deep `src/platform.rs` module at the selected seam; split only where actual responsibilities warrant it, not a generic framework.

| Work | Main files and required behavior |
| --- | --- |
| Process/window owner | `src/app.rs`, `src/renwindow.rs`, `src/renwindow_api.rs`, `src/platform.rs`: main-thread Rust owner outside Lua restart; lazy native creation, retained loop/windows, valid lifecycle callbacks; honest failed-loop state with no forbidden recreation; metadata/PLATFORM/icon/install identity and cursor-kind mapping preserved. |
| Native and custom inputs | `src/sysevents.rs`, `src/syswindow.rs`, `src/hit.rs`, `src/custom_events.rs`, `src/dirmonitor.rs`: shared owned-input queue/conversion, all public tuple shapes/units/coalescing/focus flushing, non-emitting readiness markers, ordered custom registry/directory wake delivery, normal mouse/key/IME/drop/touch behavior, click-through, multi-click counts and outside-window held-button motion/release. Audit the user-visible effects of the existing video/input hints rather than discarding them with their SDL names. Reuse the pure hit classifier/global-region quirks and title controls; call native drag/resize only inside the valid live callback, without Lua reentrancy. |
| CPU storage/presentation | `src/renderer.rs`, `src/renderer_api.rs`, `src/renwindow.rs`, `src/rencache.rs`, `src/font.rs`: compact coverage Vecs and one reusable opaque `0xFFRRGGBB` draw target, unchanged glyph/rectangle arithmetic and clipping; adapt internal RenSurface users, not an SDL shim. Full visible-row typed/stride-aware copies on changed/forced-refresh frames; no per-frame scratch, buffer-age history or borrowed presentation pixels retained. Wayland Premultiplied/ARGB with alpha FF; X11 Opaque with capability checks. Handle zero extents, resize, refresh, cache invalidation and teardown. |
| Shared runtime and signals | `src/app.rs`, `src/platform.rs`, queue/wait code: one Tokio runtime/worker, initialized before Lua if SIGINT/SIGTERM is eligible; previous-disposition checks and actual listener creation before proceeding; process-owned pending quits and notifier/proxy publication handshake. No handler-side Lua/proxy/locks; no duplicate SDL producer, polling workaround or per-generation listener/runtime. Native display and bus stay lazy. Required setup failure is stderr/nonzero before Lua; live failures use the main error path. |
| Clipboard | `src/system.rs`, `src/platform.rs`: one lazy persistent arboard instance **directly on the main thread**, synchronous clipboard/primary calls, accepted encoding/empty/failure rules. No clipboard worker/result event. |
| Dialogs | `src/sysplugin.rs`, window/runtime/queue owners: immediate zero returns; concurrent short-lived request threads on shared runtime; borrowed handles acquired/used on the proper thread; hidden-parent retention; internal tokens plus Lua-generation fences independent of duplicate caller ids; distinct statuses, filters/Accept/parenting, faithful local path bytes and no partial URI acceptance. No rfd/helper fallback. |
| Fatal report | `src/syswindow.rs`, `src/app.rs`, shared runtime: diagnostics before native Notify, existing coercion/NUL/zero returns, literal capability-aware body, critical/no-expiry request and no actions, one two-second connection-through-ack deadline. Reuse early runtime; no new runtime/helper/window, no loss of original byte diagnostics, no promise of display. |
| Remaining SDL utilities | `src/system.rs`, `src/app.rs`, `src/font.rs`, `src/renderer.rs`, `src/renderer_api.rs`: replace setenv and sandbox classification together as specified below; open font files without SDL, carry owned font errors to the existing Lua failure boundary with unchanged messages; remove SDL platform/metadata/error-string carrier calls. Dispose of custom/dialog/drop payloads with Rust ownership, never mixed allocators. |
| Harness/interface docs | `src/test_hooks.rs`, `tests/contract/contract.lua` and affected cases, `scripts/rust-port/verify.sh`, `startup-check.sh`, `hit-test-check.sh`, `docs/api/system.lua`, relevant configuration/install documentation: use the selected private-display/shared-pre-Lua seam; record the reviewed differences, retain public function/event behavior and shipping hook exclusions. |

**Mouse behavior needs explicit derivation and OS-path checks.** Winit does not supply SDL's `clicks` field: derive it in shared owned-input handling using the [linked baseline mouse state](https://github.com/libsdl-org/SDL/blob/release-3.4.14/src/events/SDL_mouse.c): per-source/button state, processing clock, interval (500 ms Linux default), app-configured 4-pixel radius with applicable baseline overrides, reset/release and saturation rules. Pin the exact 3.4.14 expectations rather than assuming a new per-window policy or emitting 1 for every press. Preserve left-button capture behavior: drag selection/scrolling must continue receiving motion and release outside the window. Winit has cursor-grab methods, but confinement/locking is not equivalent to this capture; verify native implicit-grab behavior, do not trap the cursor as a substitute. Add deterministic private-display checks plus approved real-desktop boundary/click tests. Cover focus click-through and the baseline IME/composited-window hint effects with the same source/desktop discipline. If these cannot be preserved through the selected stack, stop for an explicit new compatibility decision—none is pre-approved here.

**Sandbox/environment parity is a mechanical preservation requirement, not a new detection policy.** Keep `get_sandbox` independent of window/portal initialization and preserve its existing argument/return behavior and process-lifetime first-result cache. Port the linked SDL 3.4.14 Linux priority: accessible `/.flatpak-info` → `flatpak`; presence of all three SNAP/SNAP_NAME/SNAP_REVISION inputs → `snap`; accessible `/run/host/container-manager` → `unknown`; otherwise `none`. Do not replace it with boolean portal detection, return none unconditionally or add new container markers. Non-Linux restoration is not selected.

Preserve **environment snapshot/update semantics**, not just that priority: SDL creates its environment snapshot by initialization before Lua; its unsafe setter updates both the OS and the existing snapshot. Capture those three environment variables' presence at the corresponding process-init point, update it on successful public setenv calls until classification is cached, and retain the result across Lua restart. Check filesystem markers at the corresponding first-classification point, not prematurely at environment capture. Native-only environment changes must not be silently treated as SDL snapshot updates. No generic environment-cache framework is needed. Use existing libc for the unsafe setter's validation/NUL/overwrite/result semantics; do not substitute std::env panics or introduce late transport-environment mutation. Pin marker priority, empty/present variables, setter timing, external-vs-public changes and cache/restart cases in isolated tests. Sources: [baseline detector](https://github.com/libsdl-org/SDL/blob/release-3.4.14/src/SDL.c), [baseline environment functions](https://github.com/libsdl-org/SDL/blob/release-3.4.14/src/stdlib/SDL_getenv.c). The config expression `get_sandbox() ~= "none"` enables the **system file picker**, not the command palette.

**Must prove before shipping the cutover:**

- All sixteen suites, hit-test and startup checks on the runner's private Xvfb, ordinary build separately from hooks. Synthetic inputs enter production owned translation/queue before Lua; native capture is checked separately. Preserve historical goldens/reviewed profile expectations, not automatic re-recording.
- Direct drawing/copy assertions beyond RGB hashes: independent SDL rounding, alpha FF, distinct channel values, clipping/blending, padded odd rows/sentinels, rotating destinations with arbitrary old contents, unchanged-frame skip versus forced refresh, resize/minimize/restore/drop and checked extents/allocation.
- Shared queue, suppressed markers, focus-key flush, finite/indefinite/negative/sub-ms waits, monotonic deadlines under spurious wakes, dirmonitor and real proxy delivery. No short-timeout inflation or idle-spin fix.
- Private-display real clipboard/primary round trips; real ashpd unavailable-portal result in a fresh process on an unreachable bus; scripted narrow portal transport/fault seams with production request/result validation, URI conversion, worker/proxy and generation handling. Dialog results are not OS-evidence substitutes.
- Private-bus fake native-notification daemon checks for arguments/NUL/numeric/invalid UTF-8, original stderr bytes and ordering, body capabilities, urgency/expiry/action types, acknowledgement, service/connection/errors/timeouts. Never send contract messages to the host daemon.
- Real child SIGINT **and** SIGTERM plus default/ignored/custom cases, startup/proxy/restart races and timed/indefinite waits. Establish custom handlers in the subject before initialization; do not assume they survive exec. No signal may target the host/session. Verify no update/draw after project removal and clean ordinary quit.
- Persistent-window Lua restart, duplicate dialog ids, old-generation completion filtering, closed/hidden parents, outstanding chooser at restart/final exit, setup/listener/export/transport failures and no Lua reentrancy. Keep deliberate missing-display Lua failure/traceback/error.txt/exit 1 independent of display/bus success; native init failure must not be retried.
- The applicable real-editor desktop and paired cost checks below. Probe feasibility alone cannot pass this checkpoint.

**Stop:** any changed non-excepted interface, unsafe handle/allocator/handler lifetime, wrong URI/clipboard mapping, stale-generation delivery, missed/duplicate quit or wake, incomplete cutover, broken diagnostics, presentation mismatch or unreviewed failed gate. Return with evidence; do not split off a broken shipping midpoint or add a fallback.

### Step 5 — Remove SDL from build/package and prove the final artifact

**Touch:** Cargo.toml/lock, `flake.nix`, stale SDL imports/build/runtime declarations and SDK-dependent test plumbing, and installation/build documentation. Delete `sdl3-sys`, `sdlDeps`, remaining SDL initialization/atexit/error/allocator paths and SDL-specific environment setup in candidate scripts. Retain historical reference instructions/assets separately. Remove pkg-config only if no retained dependency needs it; do not change flake.lock merely to obtain unrelated upgrades. Keep needed source-adaptation license notices.

Build locked ordinary release and hook-enabled configurations, the Nix package and development flow. Test the installed ordinary executable outside the dev shell with no accidental inherited library path. Ensure new source files are actually included in the intentional Nix source; Cargo success on untracked files is not a Nix proof. Never include unrelated `stage/` or private raw captures merely to get them into a source closure.

**Must prove:**

- Static production-source/dependency/feature review has no remaining SDL calls/imports/crates or retired renderer path. References in historical docs/reference fixtures are not production use; a global text match alone is not the verdict.
- Inspect Cargo normal/build feature trees, Nix build inputs/runtime closure, and installed `readelf -d`/`ldd` output. Confirm no application SDL dependency; retain the allowed native system libraries.
- With separate permission, trace the **exact installed binary**, not Cargo/Nix, through exercised startup/window/presentation, clipboard, dialogs and fatal-notification paths. Capture library opens/maps and transport identities, distinguishing failed lookups from successful loads; no SDL load/attempt on those application paths. No typed-text, clipboard or D-Bus payload logging. Dynamic dlopen evidence is required; a clean lockfile/NEEDED list is insufficient.
- Re-run full automated/startup/hit checks against the final package and finish the applicable desktop/cost evidence. Record source, lock, binary, features, linked/dlopened providers and backend/scale identities. No debug/hooks build alone certifies shipping behavior.

**Stop:** remaining SDL requirement/load, missing dlopen provider outside the shell, broken install/EXEFILE identity, unsupported production override/fallback, runtime test failure or unreviewed acceptance difference. SDL is gone only after this step, not because the cutover source stopped mentioning it.

## Real-desktop and cost acceptance

Request permission before live launches, chooser/notification interaction, clipboard replacement, captures or signals. Use nonprivate markers/documents and an isolated profile; do not change monitor scales, stop Klipper, log private input or operate on the current desktop's selections without explicit scoped consent.

| Path/check | Required disposition |
| --- | --- |
| Migrated editor on native Wayland and XWayland | Actual input/shortcut/non-US/dead-key/IME composition and candidate placement, pointer/caret/scale and mixed-output movement, window/focus/icon/cursor/hit controls, click-through, multi-click counts and held-button dragging beyond window bounds, nonprivate drop, clipboard/primary and Open/Save/Directory accept/cancel/error/parent behavior; full-editor drawing parity and supported ARGB presentation. Check 1× and genuine fractional scaling; Xvfb is not fractional/native-desktop evidence. |
| Clipboard/primary without a manager | Required isolated-session comparison with SDL, without disturbing the user's current manager. Do not promise additional persistence or infer a pass from manager-assisted tests. |
| Native X11 | Retain backend support; **accepted-unverified/reactive**, not a proactive session gate. XWayland/private Xvfb do not certify it. |
| Touch hardware | Retain translation/support and carry real hardware validation as unverified until available; no invented pass or silent removal. |
| Supported sandbox package | Conditional: validate actual confined dialogs and native-notification permissions if such a package is supported. No new package or sandbox exercise is needed merely to complete planning. A denied native notification does not erase diagnostics; no silent portal/helper fallback. |
| Cold-cache startup | Unavailable without root under the existing disposition; pending, not a claimed pass or a reason to change the warm-start method. |

Use the **full numerical [calibrated gates](../issues/31-calibrate-platform-cost-gates.md#resolution)** separately per bundled/user configuration, phase and Wayland/XWayland backend. Paired A/B/B/A, at least three rounds, matched source/build/workload/physical extent. Report presentation independently from active work; copy/conversion remains inside comparable presentation totals and is also reported separately. Include early-runtime first-frame latency/RSS, retained frame capacity, backend buffers and resize transients.

Quick reference, subordinate to that resolution:

| Metric | Existing limit |
| --- | --- |
| Warm-start median | candidate slower by at most max(10 ms, 5%) |
| Active frame p95 | at most max(1 ms, 10%) slower |
| Presentation p95 | at most max(1 ms, 20%) slower |
| Focused cadence | equal SDL drawn-frame cadence; not a cost optimization |
| Focused CPU / waits / voluntary switches | at most +0.5 percentage points / +20% / +10%; fewer waits require unchanged public results |
| Unfocused idle | absolute ≤0.5% CPU and ≤10 voluntary switches/s |
| RSS size | first frame/early peak at most max(5 MiB, 5%) higher |
| Accumulation | ≤1 MiB growth over the last 30 of 50 cycles; if exceeded, use the established 200-cycle adjudication, ≤1 MiB over its last 50 |

A borderline result within one measured noise floor uses five rounds; higher first-pair XWayland noise returns for recalibration before judgment. An over-limit result requires explanation and user approval/rejection, **not** an automatic golden, silent waiver or automatic permanent rejection. SDL's known accumulation failure does not relax the candidate gate. Do not fix the focused Lua idle spin as part of this migration.

## Completion and recovery

For each step retain a last-passing revision, explicit evidence/identities, classified failures and pending dispositions. There is no user-data migration: recovery returns only the owned implementation changes to the previous passing revision, with normal process restart as needed. Never reset the user's worktree, replace documents/configuration or run a broad cleanup/kill command. Reverting the coupled step is an atomic code/build/test rollback, not an SDL runtime fallback in the shipped binary.

Completion requires the final ordinary installed artifact, applicable automated/live/cost evidence, no SDL build/runtime use, documented reviewed differences, and an honest list of deferred/conditional/unverified paths. Any newly discovered material difference returns for a fresh decision before implementation continues. Planning completion itself claims none of those implementation passes.
