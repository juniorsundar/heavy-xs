# Calibrate GUI comparison noise and desktop timing boundaries

Parent: ../gui-map.md
Labels: wayfinder:task
Type: task
Mode: HITL
Status: claimed
Assignee: pi-gui-calibration
Blocked by: 42, 45

## Question

What measurement variation and desktop timing boundaries does the pinned GUI compatibility reference exhibit under the agreed workloads, so that the human can set defensible numerical acceptance limits?

This is a bounded reference-only measurement prerequisite for [Define GUI performance and customisation acceptance criteria](43-define-gui-acceptance-criteria.md), under its approved policy comments. It is not candidate implementation, architecture selection or permission to decide acceptance budgets automatically. Use [Establish GUI runtime and customisation iteration baselines](42-baseline-gui-and-development-costs.md#resolution) and the pinned configurations from [Assemble the GUI and plugin compatibility reference evidence](45-assemble-gui-compatibility-reference.md). Any repeated policy is a measurement instruction; the acceptance decisions remain in the acceptance ticket.

### Preparation and human involvement

- Review the existing probes and evidence before adding instrumentation. Pin source, executable/build inputs, runtime data/fonts, plugin revisions/configuration and fixture hashes. Preserve existing tests and expectations.
- Before collection, record a bounded protocol: workload actions, repetition/order/pairing, warm-up/settling, cache conditions, observation boundaries, instrumentation/control checks, and explicit wall-time, per-launch/build timeout and owned-disk limits. Agree desktop access and required manual observations with the human. Do not escalate repetitions indefinitely if the environment cannot yield useful precision; record the limitation.
- This task is HITL because controlled desktop access, focus/visual/input observations and any IME/scale/session setup require human coordination. Prepare and automate what can safely be done by the agent; give the human precise steps only for actions that actually require them. Do not change the user's desktop configuration, input services, refresh settings or active projects without permission.
- Retain the third-party execution safety requirements of the compatibility-reference task. Inspect plugin sources and obtain advisory review of the actual execution setup before running them; disposable user directories alone are not a sandbox. If a safe setup for a desktop/plugin combination is unavailable, report the blocker rather than exposing personal files, credentials, projects or session services.

### Observations to collect

1. **Reference against itself.** Repeat matched A/A comparisons using the same reference executable/configuration on both sides. Cover the agreed ordinary/default/customised workloads and separately labelled bounded stress cases. Freeze concrete files, project, actions and configurations before comparisons; use interleaved or balanced order and record host load. Do not compare different configurations as if their difference measured noise.
2. **Startup and interaction.** Separate launch-to-first-completed-frame, demonstrable input-ready, and observable desktop response boundaries. Record compositor/session/backend, display scale and refresh rate. State what the method can actually establish; an internal frame marker or subjective visual observation is not a precisely timed physical input-to-pixel result. Preserve existing supported Linux paths/checks; label unavailable backend/scale/IME coverage explicitly, not passed.
3. **Default and input observations.** Record the approved default-editor and keyboard/pointer/focus/IME/geometry review scenarios, distinguishing automated assertions, human observations and missing checks. Review reference behaviour, not new appearance or features. Reuse compatibility-reference observations where their configuration/provenance genuinely match. A pre-existing failure is evidence, not an approved exclusion or a candidate regression.
4. **Resources.** Use longer settled focused/unfocused idle intervals than the exploratory baseline, account for clock-tick resolution, and record CPU/memory sampling overhead. Observe typical/peak editor memory and repeated document/view lifecycle work without forced GC, allowing normal collection/settling. Preserve interval/cycle data; finite samples cannot prove the absence of leaks.
5. **Cost of trying a change.** Assess repeatability of the existing Lua save/restart workflow and native no-op/core-edit/empty-artifact measurements where needed for defensible limits. Keep debug/release, cache/build conditions, executable size, build-disk use and runtime memory distinct. Use only bounded owned build copies/targets for edits, restore sources, and verify executable provenance; do not repeat the linked-worktree no-op measurement trap or clear/rebuild developer artifact caches. Cached Cargo sources may be reused. Record setup/download costs separately.

Retain per-run results and report typical and slower-tail timings separately, with sample counts, quantile method and observed between-run/session variability. Do not hide a weak ordinary workload behind pooled results or an overall score. Quantify limitations of instrumentation and available controls; missing or underpowered observations remain unverified/inconclusive. A noise estimate is not itself permission for that much slowdown: practical importance and permitted regression limits require the human's subsequent decision.

### Completion and boundaries

Resolve with linked reproducible evidence/manifest under `docs/wayfinder/assets/`, the executed protocol, raw or reviewable per-run results, human-observation notes, uncertainty/precision limitations and an explicit coverage/blocker table. Collect sufficient reference variation/boundary evidence for the acceptance discussion; if a required measurement cannot be made, leave the task blocked and record why rather than declaring its calibration complete. Do not invent a missing value or silently waive a check. The human may subsequently decide how to treat a blocker, with that decision recorded in the acceptance ticket.

The follow-up is a live return to [Define GUI performance and customisation acceptance criteria](43-define-gui-acceptance-criteria.md) to agree noise-aware numerical limits, practical significance, remaining required coverage and permitted trade-offs. This task does not set those limits or qualify any candidate. Do not run unbounded builds, drop host OS caches, alter production code, install dependencies silently or select/migrate GUI libraries. A fresh process or empty Cargo target is not evidence of cold-cache startup.

## Comments

### Agreed preparation boundaries — user approval

The user approved the first preparation round:

- Use two phases: isolated private-display calibration first; then a separately scheduled, human-present desktop session, initially with the pinned executable and bundled plugins only. Third-party plugins on the real desktop require a separate safety review. The private-Xvfb isolation arrangement is not automatically adequate for a shared desktop display. Do not change display settings, input services or active projects.
- Bound the first automated collection to 30 minutes total, including any build-repeatability checks; at most 5 GiB of additional owned artifacts; 60 seconds per editor launch and five minutes per build, within the overall ceiling. These are experiment ceilings, not editor-performance targets. Stop with limitations/inconclusive results rather than silently extending the budget. Do not clear OS/developer caches or install dependencies.
- Read-only queries of the current desktop's output configuration are permitted, to discover monitor scale/refresh and related facts. This permission does not include changing settings, screenshots, clipboard inspection, moving focus or launching the editor.

This approval establishes the route, preparation access and resource ceiling only. The concrete workloads, repetitions, timing/observation boundaries and execution setup still require agreement before collection. No benchmark or desktop editor launch has been authorized by this preparation round; numerical acceptance limits remain in the acceptance ticket.

### Tentative workload/boundary agreement — minimal desktop interruption

The user tentatively accepted the four proposed workloads (bundled generated 2,000-line file; bundled disposable real-source project; the same project with the working five plugins and existing small custom view as ordinary customised use; separately labelled 20,000-line large-file stress), separate software timing boundaries without physical input-to-pixel claims, and later normal/maximised window observations on the primary display.

That assent is conditional: long AFK desktop benchmark sessions are not feasible as a recurring workflow. Physical/visual checks must be used in extreme moderation. Default to unattended private-display automation; keep any real-desktop checks short, human-present, separately scheduled and expressly approved. Do not require a recurring desktop benchmark for every change. Record missing/limited evidence rather than silently treating it as passed or waived; any effect on final qualification requirements remains for the acceptance discussion.

No concrete repetition plan, desktop time allowance, benchmark collection or editor launch is approved by this tentative round. Revise the proposed protocol for minimal interruption and obtain explicit agreement before execution.

### Approved initial desktop time ceiling

The user explicitly confirmed that a two-minute total ceiling is practical for the initial desktop check. Split this into at most two human-present launches of up to 60 seconds each, normal and maximised, with no AFK or repeated-launch benchmark loop. This approves the time ceiling only: the desktop check still requires a reviewed setup and a separate readiness/go-ahead before launching. Later desktop checks should be exceptional, at an agreed final-candidate checkpoint or for a specific suspected problem, not routine per-change benchmarks.

The [proposed concrete calibration protocol](../assets/gui-calibration-protocol.md) specifies private-display repetitions, controls and budget allocation. It is a draft awaiting agreement, not an executed protocol or authorization to launch the desktop check.

### Read-only desktop facts — preparation, not measurement

`kscreen-doctor --json` at 2026-10-23 12:01:27 UTC reported KDE/Wayland with two enabled outputs: `DP-8` (priority 1), 3440×1440 at 59.973 Hz, scale 1; and `DP-7` (priority 2), 2560×1440 at 59.951 Hz, scale 1, rotated. The connected laptop panel `eDP-1` was disabled; its listed 165 Hz mode is not an active-desktop condition. This was a configuration read only: no setting, focus, screen capture, clipboard or editor operation was performed. The pinned X11 reference would run through Xwayland on this desktop; no native-Wayland executable has been qualified by these facts.
