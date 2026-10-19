# Prototype the editor loop, clipboard and dialogs

Parent: ../sdl3-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: pi-sdl3-loop-prototype
Blocked by: 24, 26

## Question

Apply [the agreed acceptance criteria](26-define-platform-acceptance.md#resolution). Record unavailable checks as pending, distinguish XWayland from a native X11 session, and request explicit approval for any proposed difference. This standalone prototype provides feasibility evidence, not full-editor acceptance.

Can the editor keep its own poll/wait loop on winit, be woken from a worker thread and by an asynchronous file-dialog result, and keep clipboard and primary selection working on KDE Wayland and X11?

A standalone throwaway binary: pump with a timeout and wake through `EventLoopProxy` from a thread; open portal dialogs with rfd and with ashpd (open, save, directory, filters) and check the loop wakes and modality; copy, paste and middle-click primary selection with arboard (with `wayland-data-control`) including after the app exits; check how sandbox detection could work. The user judges dialog and clipboard behaviour.

Context: [Survey Rust-native platform crates](25-survey-rust-platform-crates.md) (experiments E4, E5, X4).

## Comments

### Progress — standalone probe ready; live service review pending

Captured on local-only throwaway branch `prototype/sdl3-loop-probe`, commit `beaa9ae2` (base `f8dacae0` from the window probe; not pushed or merged). Locate its worktree with `git worktree list`. The source, pinned lockfile, launcher, `check-loop.sh`, optional visible-pixel check and README are under `docs/wayfinder/assets/sdl3-loop-probe/` in that worktree. Run `bash docs/wayfinder/assets/sdl3-loop-probe/run.sh --backend wayland`; repeat with `--backend x11` for X11-client evidence, explicitly labelling XWayland when appropriate. No editor code changed.

The probe reuses the earlier drawing/input helper and keeps its experimental Wayland opaque-padding workaround explicitly unapproved. It drives winit 0.30.13 with main-thread zero-timeout poll followed by a bounded wait, handling native callbacks inline. Worker events and service results arrive through typed `EventLoopProxy` messages. This establishes no production-loop choice or full Lua event-translation result.

Controls are displayed below the field: F1 worker wake; F2 overwrite clipboard with public marker / F3 read summary; F4 overwrite primary with public marker / F5 read summary; F6 switch rfd/ashpd; F8 open, F9 save, F10 directory; F11 sandbox heuristic. Clipboard instances are lazy, serialized on one persistent worker and retained for process life. Dialog workers hold the parent window and post redacted outcomes. No clipboard or portal access occurs at startup or during self-check. Explicit writes overwrite existing selections; ordinary typed-key logs still contain supplied text, so use nonprivate samples. Clipboard contents and selected paths/URIs are never logged, and the probe never reads or writes a selected file.

Pinned service crates: arboard 3.7.0 with text-only Wayland data control, rfd 0.17.2 portal/Wayland features, ashpd 0.13.13 file chooser/raw-handle/Wayland with its default Tokio runtime (1.53.3). The launcher forces clipboard transport along with the window backend in the child environment, preventing silent cross-transport fallback.

**Discovered API limitations, not approved differences:** rfd's `Option` result conflates cancellation and some failures; it is labelled `no-selection(cancel-or-error)`. Its Linux portal path dlopens system libdbus and can fall back to Zenity; the probe detects and reports `fallback-zenity=true`, which cannot count as portal evidence. ashpd classifies cancellation separately from errors. `ashpd::is_sandboxed()` is a cached heuristic, not an actual sandbox test. These findings inform eventual selection, not an automatic rejection or acceptance of either API.

**Automated evidence:** build, all 12 unit tests, format/shell checks and independent light review passed. Both decorated/borderless runs on private Xvfb (with an invalid private D-Bus address) passed the real blocked-pump test: 6 pump iterations, two proxy interruptions before the 5-second bound (about 150–175 ms), one worker wake, one explicitly simulated async-result wake and three frames. Dropping wake sends in a temporary fault injection made the check fail with exit 1; restored code passed again. This is diagnostic wake/spin evidence, not measured editor performance or real dialog completion. Static Linux normal/build audit found 189 deduplicated package rows, no SDL package and no dynamic SDL symbols; runtime service traces remain pending. The README records build identities and exact limitations.

**Still pending:** actual Wayland and X11/XWayland cross-application clipboard/primary behaviour, real rfd/ashpd open/save/directory results and cancellation, filters and modality, continued pump processing during real dialogs, selection availability after process exit, native X11-session evidence, actual sandbox-specific behaviour and live service dependency traces. Human results are not inferred from worker simulations or unit tests. The ticket remains claimed, no stack is selected and no acceptance gate is waived.

### User feedback — Wayland worker and selection checks

For the instructed Wayland run, the user reported: “step 1 -5 seems to work.” This records positive live feedback for the F1 worker wake, clipboard-marker paste into another application, clipboard read summary after copying externally, primary-marker middle-click paste into another application, and primary read summary after external selection. Read summaries expose byte count/marker-match, not arbitrary text, so this is bounded behavioural feedback rather than an exact arbitrary-payload integrity test.

Real dialogs, their result wake-ups/filters/modality, selection availability after exit, X11/XWayland and native-X11 service checks, actual sandbox behaviour and live runtime traces remain pending. The ticket stays claimed; no stack or gate revision is approved.

### Failure — rfd open dialog on Wayland; probe environment fixed

The user's first rfd F8 attempt showed no dialog and returned in 35 ms: rfd reported a backend error, fell back to Zenity (`fallback-zenity=true`), and the fallback also failed. This is a **failed dialog attempt, not a cancellation or pass**. The following worker wake shows only that the loop stayed responsive.

A test with no bus connection or GUI isolated the first failure: neither of rfd 0.17.2's dlopen names for libdbus resolved in the launcher environment, because the repository dev shell does not include dbus. The pinned rfd source reports exactly that failure, then returns no result. The probe's `check-loop.sh` now runs this loader test; it failed before the change and passes after it. The launcher enters the pinned repository dev shell plus `dbus` through a probe-only override, leaving the repository flake unchanged. rfd failure logs now name the failure stage without printing details.

Fix snapshot: local throwaway commit `af90d33a` on the same branch, not pushed. Thirteen unit tests, the loader test and both private-Xvfb loop runs pass. Whether rfd's portal call now opens a dialog is unverified until the user retries. **Packaging fact for selection:** rfd's portal path needs the desktop's libdbus at runtime; the editor's packaging would have to provide it. This records no decision.

### Retry — rfd accept path works on Wayland

After the fix, the user's Wayland log (scale 1.20) shows rfd Open, Save and Directory each returning `selected count=1` with `fallback-zenity=false`, so the result came from the portal, not Zenity. Results arrived after about 5.2, 3.3 and 3.3 seconds. While each dialog was open, the 150 ms worker wake was processed with `dialog_pending=true` and redrew the window, and each result interrupted a blocked pump wait. The probe regained focus and accepted clicks afterwards. No backend error lines appeared.

This log does not show cancellation, filter contents or whether the parent window was blocked while the dialog was open, so those remain pending for rfd, along with all ashpd checks.

### Failure — ashpd hung after its first dialog; probe fixed

The user reported that ashpd F8 worked once, after which no dialog opened. The log shows the first ashpd Open returning `selected count=1` after about 8 s; the following Save logged `Ashpd Save pending` and never returned. Later F10/F8 presses were silently ignored because the probe allows one outstanding dialog. So this was one stuck request, not three separate failures.

Cause, confirmed in the pinned sources and by a regression test: ashpd 0.13.13 caches its zbus connection for the whole process, and zbus runs that connection's background tasks on the Tokio runtime that created it. The probe created a fresh runtime per dialog and dropped it afterwards, so every request after the first waited forever. A test running three successive ashpd requests on a private bus with no portal hung on the second request before the fix; afterwards all three fail promptly. The probe now keeps one process-lifetime dialog runtime and logs ignored dialog keys. This was the probe's integration error, not evidence against ashpd. **Integration fact for selection:** ashpd needs a long-lived async runtime owned by the application.

Fix snapshot: local throwaway commit `4b275ae7`, same branch, not pushed. Thirteen unit tests, the libdbus loader test, the ashpd repeat test and both private-Xvfb loop runs pass. The ashpd Wayland checks, plus rfd cancellation, filters and blocking, still need the user's retry.

### Retry after the runtime fix

The user reported “Ok is working now.” The pasted Wayland log (scale 1.20) shows only **rfd** requests on the fixed build: Save, Open and Directory each returned `selected count=1` with `fallback-zenity=false` after about 1.8, 6.6 and 1.9 s. During each, the worker wake was processed with `dialog_pending=true`, and the result woke the blocked pump. This confirms rfd still works on the shared dialog runtime. The log contains no ashpd requests and no cancellations, so it is not evidence for either; those wait for the user's explicit answers.

A second pasted Wayland log on the fixed build shows:

- **rfd cancel:** an Open returned `no-selection(cancel-or-error) fallback-zenity=false` after about 1.0 s, with the probe refocusing afterwards. This is consistent with a portal cancel, but rfd cannot distinguish it from an error.
- **ashpd, repeated use:** Open, Save and Directory, one after another, each returned `selected count=1` after about 6.2, 1.2 and 1.9 s. This is the sequence that previously hung on the second request, so the runtime fix holds on the real desktop. During each dialog the worker wake was processed with `dialog_pending=true`, and each result woke the blocked pump.

Not shown in either log: an ashpd cancellation (`no-selection(cancelled)`), the offered filters, or whether the parent window was blocked while a dialog was open. Those, plus selection availability after exit, the X11/XWayland service checks, native X11 and actual sandbox behaviour, remain pending.

### User feedback — after exit and XWayland

The user reported that the clipboard and primary markers **survive after the probe exits**, and that **all checks worked with the X11 backend**. The session is KDE Wayland, so the X11 run is XWayland: X11-client evidence under a Wayland compositor, not a native X11 session.

KDE's clipboard manager (Klipper, inside plasmashell) was running on the session bus. The probe runs no helper after exit, so the surviving markers were most likely preserved by Klipper, not by the probe. This shows selections survive on this desktop configuration; it does not show that they survive without a clipboard manager, which is how SDL-based Lite XL behaves today as well. Untested.

The user then reported trying the remaining three dialog checks and that they worked well: ashpd cancellation, the Rust/Text/All files filters, and the parent window being blocked while a dialog was open.

### Resolution

The user chose to **close this prototype as a success and fold its remaining checks into the desktop-checks task**. This answers the question for KDE Wayland and XWayland; it selects no stack and waives no gate.

- **Own loop on winit: yes.** A main-thread zero-timeout poll followed by a bounded wait through winit 0.30.13's pump API kept the window responsive. Worker threads and real asynchronous dialog results woke a blocked wait through typed `EventLoopProxy` events, including while dialogs were open. This is feasibility evidence; choosing between the pump model and a callback-driven loop remains open fog on the map.
- **Clipboard and primary selection: working.** arboard 3.7.0 copied and read both, in both directions with other applications, on Wayland (data-control) and under XWayland. The markers survived the probe's exit, most likely through KDE's Klipper; survival without a clipboard manager is untested.
- **Dialogs: both APIs work.** rfd 0.17.2 and ashpd 0.13.13 Open, Save and Directory dialogs accepted, cancelled, showed filters, blocked the parent window and woke the loop on completion, on Wayland and under XWayland. rfd cannot tell cancellation from failure and falls back to Zenity silently; ashpd reports cancellation separately.
- **Integration facts found by failures, for stack selection:** rfd's portal path dlopens the desktop's libdbus at runtime, which the editor's packaging must provide. ashpd caches one D-Bus connection per process and needs an async runtime that lives as long as the process; a per-dialog runtime hangs every request after the first. Both were probe errors, fixed and covered by tests.
- **Sandbox detection:** `ashpd::is_sandboxed()` exists as a cached heuristic; no real sandbox was tested.
- **Primary source:** local-only throwaway branch `prototype/sdl3-loop-probe`, commits `beaa9ae2` to `4b275ae7`, with source, launcher, `check-loop.sh` and README under `docs/wayfinder/assets/sdl3-loop-probe/` in its worktree. Not pushed or merged; no editor code changed. The inherited Wayland pixel-padding workaround is still owned by [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md).

**Moved, not waived:** native-X11-session clipboard, primary and dialog checks; sandboxed dialog behaviour; selection survival without a clipboard manager; and live runtime dependency traces for the service crates were moved to [Complete the outstanding window and input desktop checks](33-complete-platform-desktop-checks.md#resolution), whose resolution records their subsequent user-approved validation dispositions.
