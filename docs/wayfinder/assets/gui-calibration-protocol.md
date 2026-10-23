# Proposed GUI reference calibration protocol

For [Calibrate GUI comparison noise and desktop timing boundaries](../issues/46-calibrate-gui-comparison-noise.md). **Draft: concrete collection is not yet approved or started.** The ticket records the user's approved preparation limits, tentative workload/boundary agreement, and explicit two-minute total initial desktop ceiling. This proposal sets experiment procedures, not numerical acceptance limits or candidate qualification.

## Purpose and interruption limits

Measure variation in repeated, matched reference-against-reference (A/A) observations. Both labels mean the same executable, files, configuration and instrumentation; A/B labels identify ordering, not different designs. Do not interpret default-versus-customised differences as noise.

Routine work stays on an owned private Xvfb: no visible desktop window, focus stealing, screenshots or clipboard reads. The user need not leave the desktop idle or a window running AFK. Record host activity/load and any resulting uncertainty rather than demand inactivity or increase repetitions without agreement.

The initial real-desktop check is exceptional and human-present: at most two launches, normal and maximised, at most 60 seconds each and two minutes total. Schedule it separately, after reviewing the actual setup and obtaining an explicit readiness/go-ahead. No automatic host-desktop input driver, launch loop, or third-party plugins. Future desktop checks are reserved for agreed final-candidate checkpoints or targeted suspected problems, not every change.

## Fixed reference, inputs and safety

Reuse the [compatibility reference](gui-compatibility-reference.md): source `e556d388892280d8343a82a625941137e3a6ac5f`; clean default-X11 release executable SHA256 `bde5ae899aef0302c41201ad54569f045bdef0db6a46874bd0a6f835642cb186`; byte-checked runtime data/fonts and pinned five working plugins from its manifest. Neither the failed Bracket Match/Indent Guide candidates nor their failed combinations become passing configurations or approved exclusions.

Private display: Xvfb 1280×800×24, DPI 96; editor 1024×640, scale 1; reference bundled defaults unless explicitly listed below. Fresh scratch HOME/userdir/session/project per launch. Writable project/init copies live inside the sandbox; canonical host inputs remain read-only. Never open/edit a personal project or execute project configuration from an arbitrary checkout. Real-project seeds contain only the listed pinned public Lua sources, not a developer's `.git`, user settings or `.lite_project.lua`.

Record source, executable, runtime/font/plugin hashes, exact fixture/project file hashes, seed configuration, environment/backend/window geometry, helper sources/binaries, toolchain, timer/clock-tick units and host/block metadata. Use the recorded source/build provenance rather than assume HEAD alone identifies a binary.

Adapt the existing disposable timing probe and reviewed sandbox runner, leaving retained baseline/reference evidence unchanged. Before any editor execution, finish source/argument review and self-checks. Before third-party execution, obtain advisory review of the actual calibration setup, verify the byte pins, and pass trusted filesystem/network/PID/environment/resource/no-display/normal-core gates. Revalidate changes rather than assume the old private-display review covers new launch/bootstrap code. Gate/smoke launches count against the collection budget. Stop on unexpected failures; do not keep sampling a broken workload.

## Four separate workloads

| ID | Configuration and files | Category |
| --- | --- | --- |
| W1 | Bundled defaults; existing generated 2,000-line Lua fixture from the cost-baseline generator | Ordinary, reproducible generated file |
| W2 | Bundled defaults; disposable copy of pinned `data/core/**` and `data/plugins/**`, preserving paths; open `data/core/docview.lua` | Ordinary, small real-source project |
| W3 | Same project/file as W2, plus the five working plugins and the existing 80-rectangle `View` example | Ordinary customised use, not stress |
| W4 | Bundled defaults; same generator as W1, 20,000 lines | Separately labelled large-file stress |

W3 uses `autosaveonfocuslost,bigclock,custom_caret,scalestatus,smoothcaret` in the observed loader order. It does not additionally enable the baseline's whitespace/line-guide configuration; do not label it identical to that earlier custom variant. The small view uses the existing class/draw behaviour, not a new widget framework. These are current-reference configurations, not substitutes for future candidate customisation demonstrations.

For each interaction launch: after the startup/focus/input handshake, run 30 untimed warm-up frames, then 60 action/draw samples each for typing, scrolling and split/divider layout changes, using the baseline action definitions. The earlier baseline used 90 samples/action; this shorter new protocol is calibrated against itself, not treated as an identical rerun of its old percentiles. Also assert one bounded open/save/tab-switch/split/search/command-prompt/close sequence against scratch files, with results recorded separately from the repeated action timings. Pin the exact sequence/helper and expected saved content before collection. These are scripted Lua/editor actions; only the specified private-display input handshake crosses the native event path.

W4 is size stress only. Long/complex lines, larger real projects, many-view extremes and heavier plugin integrations remain named gaps, not implied coverage. Do not pool results across workloads or compare differently sized/configured scenes as A/A.

## Repetitions and ordering

Two collection blocks within the same host session. First block visits W1→W4; second W4→W1, with resource/lifecycle observations between them. This observes between-block drift, not different-day or different-desktop-session variability.

| Observation | Per workload, across both blocks | Total |
| --- | --- | --- |
| Priming, not scored | Two fresh-process launches per block | 16 launches |
| Startup/input A/A | 30 matched pairs: 15 per block | 60 launches/workload; 240 total |
| Repeated interaction A/A | 10 matched pairs: five per block | 20 launches/workload; 80 total |
| Settled idle | Two launches: one per block | Eight launches |
| No-forced-GC lifecycle | Two launches: one per block | Eight launches |
| Lua save/restart | W1 and W3 only; four matched pairs each, two per block | Eight launches per selected workload; 16 total |

The overhead controls below are additional: six W1 startup launches and two W1 idle launches. Together with the table this schedules 376 private-display launches, before the small trusted gate/smoke set, all budgeted. Launch count is not human effort: none of these windows appears on or takes focus from the user's desktop.

Within each pair alternate AB and BA; reverse the odd extra order in the second block so each workload's aggregate ordering balances. Preserve pair IDs and exact order. Fresh processes, retained OS/Cargo caches and priming are recorded conditions, not cold-cache startup evidence. Do not drop caches, pin CPUs, change governors or alter desktop settings.

Report startup/frame/input measurements per workload and block, with per-run raw values and pair differences. Report process-level results before any within-workload aggregate. Individual frame samples are nested within their launch; do not treat them as thousands of independent processes. Use the existing nearest-rank quantile convention, state sample counts and report typical/p95 behaviour separately. With only 20 interaction launches/workload and fewer restart/resource observations, slower-tail and between-block conclusions may remain weak/inconclusive; no p99, precision guarantee or regression budget follows from these counts.

## Measurement boundaries and controls

- **Launch to first completed internal frame:** parent spawn timestamp to observed flushed first-frame marker. Include/document namespace/bootstrap/pipe overhead; this is not directly interchangeable with the older unsandboxed launch number or proof of monitor visibility.
- **Demonstrated input readiness:** private-display focus is asserted, a known test key is delivered through the native path, and the Lua handler acknowledges it. Record the parent/helper boundary and Lua event timestamp separately; it is not the earliest possible input-ready instant or physical keyboard latency.
- **Acknowledgement to response frame:** a deterministic scratch-document change requests redraw; record its subsequent completed frame in the same editor clock domain. Do not subtract unrelated Node/Lua clock origins.
- **Scripted action to completed step:** retain dispatch/update/Lua-draw/end-frame boundaries. End-frame includes dirty-cache/raster/presentation submission work, not pure presentation or physical scanout.
- **Controls:** retain timer-call overhead checks; add three W1 startup pairs comparing minimal marker-only against detailed instrumentation, separately labelled as overhead controls, not identical A/A treatments. Add one extra W1 idle launch per block with periodic sampling disabled but interval endpoints retained, adjacent to its sampled run. Record observer CPU/time cost and uncertainty rather than assume the observer is free. Neither control is a perfectly uninstrumented measurement.
- Capture block/pair timing and host-load context. Preserve anomalies and stop reasons. Missing markers, wrong process/focus, core errors, wrong file contents or quota/deadline failures invalidate the affected observation and stop the affected collection; investigate rather than silently retry until it passes.

## Idle, memory and lifecycle

Each idle launch has 5 s focused settling + 20 s focused observation, then an asserted native focus loss, 5 s unfocused settling + 20 s unfocused observation. No forced redraw loop or periodic in-editor logging during the observation intervals. The approximately 50 s observation fits the 60 s launch ceiling; startup/driver/teardown must fit too, otherwise record the limitation rather than extending it.

Sample the editor's CPU ticks, resident memory and lifetime resident high-water mark externally at 1 Hz plus interval endpoints; record elapsed time and clock-tick resolution. At 100 ticks/s, one tick in a 20 s window is 0.05 percentage points of one core; zero observed ticks is not proof of exactly zero CPU. Main-thread context-switch counters must be labelled as such. `VmHWM` is process-lifetime peak RSS, not whole-system peak, PSS or an infinite-session bound.

The sandbox wrapper PID is not the editor PID. Resolve only the owned child subtree, verify executable/comm and `/proc` start-time identity, and stop if identity is uncertain. Do not sample unrelated applications or clipboard/window contents.

Lifecycle runs use the existing 60 document/view add-remove cycles with 200-line transient documents, but remove the harness's explicit GC call. Warm up first; retain per-cycle observations and a 10 s post-cycle settling interval. Do not invoke clock-font reconfiguration or other deliberate plugin GC during this resource scenario. Normal automatic collection remains enabled. Verify documents/views become unreferenced through the intended lifecycle; finite RSS drift is not proof of a leak or its absence. Capture typical/peak RSS during interaction runs as well, labelled by scenario and sampling cadence.

Lua save/restart repeats the existing caret-width edit/save/automatic-restart path using a writable scratch init, not the read-only canonical probe. Record save-to-restarted-first-frame separately from process relaunch and preserve normal collector behaviour.

## Native development cost, bounded separately

One fresh, owned normal-`.git` source copy at the pinned revision, with fresh debug/release artifact directories; reuse cached Cargo sources offline/locked with four jobs. Do not reuse the linked-worktree no-op trap, clear caches or modify retained baseline targets. Record setup cost separately.

For each profile: one empty-artifact build, three no-ops, one runtime-irrelevant source edit/rebuild, then exact-byte restore/rebuild. Verify restoration and record produced executable hashes; do not silently replace the byte-pinned runtime reference if an experimental build differs. This adds a second observation session to the existing build evidence, not enough samples to claim a native-build p95. Report every value and leave weak repeatability bounds explicit.

The entire native-cost block has at most five minutes, not five minutes for every command cumulatively. Each individual command also has the approved five-minute maximum, shortened to the remaining block/global allowance. If it does not fit, stop, restore the owned source and record omitted/aborted observations; do not extend the budget.

## Time, disk and stop rules

- Sum of active collection windows: **at most 30 minutes**, including gates/smoke/control runs, native builds and any timed desktop launches. Waiting for human readiness is not a running collection: stop all editor processes first.
- Private runtime target: about 20 minutes, hard ceiling **22 minutes**, stopping earlier if the global allowance demands it. This is an estimate, not an assertion that all counts will complete under arbitrary load. Existing recorded unsandboxed startup launches took 0.088–0.181 s and 300-frame interaction launches took 5.080–5.191 s; new bootstrap/project/feature-sequence costs have not been measured. The planning allowances below deliberately include room beyond proportional frame time.

  | Private-display item | Planning allowance, not an observed new result |
  | --- | --- |
  | 240 startup + 16 priming + six startup-control launches | 262 × 0.4 s ≈ 105 s |
  | 80 interaction launches, now 210 warm-up/action frames plus the feature sequence | 80 × 4.5 s = 360 s |
  | Eight idle + two idle-control launches | 10 × 53 s = 530 s |
  | Eight lifecycle launches including the 10 s settle | 8 × 13 s = 104 s |
  | 16 Lua save/restart launches | 16 × 1 s = 16 s |
  | Gates, four workload smoke checks, metadata and scheduling overhead | 60 s |
  | Total private-display planning allowance | ≈1,175 s (19.6 min) |

  After the trusted gates and four unscored workload smoke checks, use observed end-to-end costs to check feasibility before bulk collection. If the planned counts will not fit the remaining allowance, stop and request a revised plan; do not assume these estimates are results or silently reduce the sample plan. Smoke/gate costs still count. A later unexpected overrun preserves partial data and an explicit stop reason, never a claim that planned counts were completed.
- Reserve at most five minutes for native cost, **two minutes total for the human-present desktop check**, and one minute for teardown/report bookkeeping. Unused allowance is not permission to exceed the desktop cap or add samples.
- Every editor launch: terminate by 55 s if needed and force-kill by 60 s, including grace time. A build's deadline is the minimum of its individual, block and remaining global allowance. Do not start a launch without its full timeout/cleanup reservation remaining. Budget exhaustion yields partial/inconclusive results, not improvised smaller repetitions called equivalent.
- At most **5 GiB of additional owned artifacts**, excluding existing retained baseline/reference evidence. One fresh debug/release pair previously cost about 1.8 GiB; record actual usage rather than assume that bound. Stop scheduling at 4 GiB, retaining 1 GiB headroom; use resource/file/log limits and supervised owned-disk checks during builds/runs, not only at the end. Never delete developer artifacts to make room.
- Keep all existing sandbox protections and reviewed per-process/tmpfs/output limits, adjusting wall-time only to the agreed ceiling. No dependencies installed, new source downloads, OS cache clearing, host input-service changes or production/test/golden edits.

## Desktop spot-check and remaining coverage

Only after separate setup review and a human readiness/go-ahead: the same trusted executable and bundled defaults, scratch files, current primary `DP-8` (configured 3440×1440, 59.973 Hz, scale 1) on KDE/Wayland via Xwayland. Re-read mode facts if they have changed; do not change settings to reproduce them. Use normal 1024×640 and maximised window cases, recording actual dimensions, one launch each. The human controls focus and keyboard/pointer; never reuse the private-display driver against the shared desktop.

Prioritise a brief readable-default/typing/selection/scroll and focus/prompt/menu check, with normal/maximised geometry observations. Prepare the short checklist before launch. No AFK waiting or benchmark loop. Do not impose an IME-service setup or rush an unfamiliar task to fit the clock. Stop at two minutes even if the checklist is incomplete; record what was and was not observed.

This does not establish physical input-to-pixel timing, native-Wayland operation, fractional-scale behaviour, exhaustive IME correctness or every lifecycle/plugin interaction. The check's short duration is not permission to mark missing checks passed. Produce a coverage/blocker table for the acceptance discussion; if required evidence remains unavailable, leave this task unresolved/blocked rather than silently waive it. Required coverage and numerical acceptance decisions belong to [Define GUI performance and customisation acceptance criteria](../issues/43-define-gui-acceptance-criteria.md).

## Before execution

Obtain agreement on this concrete protocol, then implement only the disposable adaptations it needs, self-check them and obtain advisory/default-model review of the actual setup. Any material change to workloads, access, quotas or observation claims requires renewed agreement. Approval of private-display preparation/collection does not schedule or authorize the desktop launches; ask when the user is ready.
