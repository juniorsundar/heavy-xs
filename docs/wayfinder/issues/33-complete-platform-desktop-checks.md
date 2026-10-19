# Complete the outstanding window and input desktop checks

Parent: ../sdl3-map.md
Labels: wayfinder:task
Type: task
Mode: HITL
Status: resolved
Assignee: juniorsundar / pi-desktop-checks
Blocked by: 26, 29, 30

## Question

Complete the remaining real-desktop checks from [Prototype window, scale and IME on Wayland and X11](29-prototype-window-scale-ime.md), so stack selection has either reviewed evidence or an explicit user-approved disposition for each gap. Apply [the agreed acceptance criteria](26-define-platform-acceptance.md#resolution); closing the first prototype waived none of them.

Reuse the existing standalone probe on local throwaway branch `prototype/sdl3-window-probe`, commit `f8dacae0`, or a subsequently reviewed version. Locate its worktree with `git worktree list`. Do not rebuild a second editor or silently expand its minimal field into a complete text widget. Record backend, real session type, scale and exact probe version for each result.

Already reviewed: corrected native Wayland grid/appearance, borderless drag/resize, ordinary typing and IME composition/popup placement at fractional scale 1.25; XWayland composition and refocus/editing behaviour. Xvfb startup/presentation is automated smoke evidence only. Do not require the user to repeat passed checks without a relevant change.

Original checklist (see the approved dispositions in [Resolution](#resolution)):

- Review the required window/input scenarios in a **native X11 session**, not only XWayland: appearance, resize, maximise/minimise/restore/fullscreen, borderless drag and all resize edges/corners, shortcuts, ordinary typing, IME composition and candidate placement, and focus transitions. Carry any unreported scenarios on Wayland/XWayland too; successful editing alone is not evidence for their window controls.
- Check real 1× display behaviour and fractional scaling, including pointer/caret alignment; move between differently scaled outputs where available. Xvfb at scale 1 is not human 1× display review.
- Exercise non-US layout/dead keys, cursor changes, window icon and a real nonprivate file drop. Verify arrow-key delivery separately if needed: the probe currently has no left/right caret-navigation implementation, so its stationary caret cannot judge event delivery or editor navigation.
- Exercise touch on available hardware. If hardware or a desktop session is unavailable, leave that check pending until the user explicitly decides whether and how to narrow the gate; no inferred waiver.
- Complete the native Wayland runtime dependency trace/audit on the exact tested feature set. The initial no-SDL runtime-load evidence covered Xvfb, not Wayland; the development shell still contains SDL.

Keep the known Wayland opaque-padding workaround visible in every result; passing with it does not qualify production presentation. From [Prototype the editor loop, clipboard and dialogs](30-prototype-loop-and-dialogs.md#resolution), which was resolved with these moved here rather than waived: in a **native X11 session**, clipboard/primary in both directions and rfd/ashpd dialogs; **sandboxed** dialog behaviour; whether clipboard and primary survive exit **without a clipboard manager** such as Klipper; and live runtime dependency traces for the service crates. Use the loop probe on throwaway branch `prototype/sdl3-loop-probe` (commit `4b275ae7` or later). Wayland and XWayland service results are already recorded there and need no repeat. Full-editor contract/drawing/performance validation remains in the migration handoff.

Record a pass, fail or pending result for every required item, with the human's actual feedback. Resolve only after the remaining checks are completed or explicitly revised by the user, recording the exact revision and its effect on selection/handoff requirements. Never silently count XWayland as a native X11 session or unavailable checks as passed.

## Comments

### Native Wayland 1× borderless review — partial evidence

- Probe commit: `f8dacae00279e014595f9aacb61755ee44cab793`; detached worktree `../heavy-xs-window-probe`.
- Cargo.lock SHA-256: `25d736f989c7f615073a59355bf0f05f968291077217ebf6bac6db48722d2c85`; rebuilt debug executable SHA-256: `7c077761d42cc0927fc002263980f5b9940d3eb62ed844f6d784f11e5baae584`.
- Launched with `--backend wayland --borderless` in the native KDE Wayland session. Probe reported scale `1.000` and successful presentation; both connected outputs were independently reported at scale 1 by `kscreen-doctor`.
- **Opaque-padding workaround remains enabled.** These results do not qualify production presentation; that remains with [Qualify Wayland presentation](32-qualify-wayland-presentation.md).
- Prompted human checklist: grid/text appearance and redraw after resizing; green-strip dragging and all four edges/four corners; F7 cursor shapes; taskbar/window-switcher icon; nonprivate file drop.
- Human feedback: “Nothin looks wrong according to your tests”. **Pass, user-reported**, for that prompted checklist at native Wayland 1× in borderless mode. No per-action event-log audit or pixel-parity claim is implied.
- Follow-up human window-control review on the same native Wayland 1× borderless probe: **pass**, explicitly answered “All passed” for maximise/restore, minimise/restore, entering/leaving fullscreen, and switching focus away/back with correct redraw and responsiveness. This does not establish arbitrary application-shortcut behaviour.

### Native Wayland 1× decorated and keyboard review — partial evidence

- Same window-probe commit, lock and executable hashes as above; launched without `--borderless`, with `--backend wayland`, in the same native KDE Wayland session. Probe reported scale `1.000` and successful presentation. **Opaque-padding workaround remains enabled; production presentation remains unqualified by these results.**
- Decorated appearance/redraw, titlebar dragging/resizing, minimise/maximise/restore, fullscreen and focus transitions: **pass, user-reported**, explicitly answered “All passed”. Human observation: “I see a title bar now. I then see a blue border maybe about 5 pixels thick. Then there is a green top bar, then the grid and other stuff.”
- Non-US layout/dead-key input question (intended characters without omissions/duplicates): **pass, user-reported**, answered “Tested and passed”. Exact layout and keystroke sequence were not supplied; no particular layout or separate coverage of both input mechanisms is inferred.
- Separate arrow-key delivery: **pass** on the same native Wayland 1× decorated probe/version with the opaque-padding workaround. After the user confirmed testing, the fresh local run log contained `ArrowLeft`, `ArrowRight`, `ArrowUp` and `ArrowDown` events. Only named arrow events and backend/scale metadata were inspected; raw typed text was not copied. This establishes delivery, not event-count correctness or editor caret navigation (which the probe does not implement).

### Native Wayland window-probe runtime dependency audit — pass within scope

- Exact reviewed window-probe commit `f8dacae00279e014595f9aacb61755ee44cab793`; probe lock `docs/wayfinder/assets/sdl3-window-probe/Cargo.lock` SHA-256 `25d736f989c7f615073a59355bf0f05f968291077217ebf6bac6db48722d2c85`; executable SHA-256 `7c077761d42cc0927fc002263980f5b9940d3eb62ed844f6d784f11e5baae584`. Lock and executable hashes unchanged before/after; no rebuild and probe worktree clean.
- Native KDE Wayland, decorated, scale `1.000`, `--backend wayland --smoke-seconds 3`. Exit 0 with a successful 800×520 presented frame. **Opaque-padding workaround remains enabled; this dependency audit does not qualify production presentation.**
- Mirrored the launcher's Nix shell, library paths and font setup, then traced the executable itself, not Cargo/Nix: `strace -f -e trace=openat,open,mmap,socket,connect`. Inspected successful library opens separately from failed lookup attempts. Assertions against identity, frame/backend and syscall evidence passed.
- **No SDL library open attempts or loads observed.** The locked Linux normal/build dependency tree contains no SDL package (the probe's own name is not an SDL dependency); `ldd`/`readelf` show only libc, libm, libgcc_s and the loader as direct dependencies. Runtime successfully loads `libwayland-client.so.0` and `libxkbcommon.so.0`.
- Selected feature audit: winit `0.30.13` with default/Wayland/X11 support; softbuffer `0.4.8` with Wayland/X11 and dlopen support; wayland-backend `0.3.17` with `client_system`, `default`, `dlopen`, without `log` (its `cc` build dependency does not compile the optional logging shim in this feature set).
- X11 libraries also load eagerly through softbuffer's compiled-in `tiny-xlib` constructor, confirmed with constructor-symbol and loader diagnostics: `libX11.so.6`, `libX11-xcb.so.1`, `libxcb.so.1`, `libXau.so.6`, `libXdmcp.so.6`. This is **not an X11 connection or backend fallback**: the follow-up syscall trace records exactly one socket and one successful connection, to the native `wayland-0` Unix socket, no failed connections, and no X11 Unix sockets or IPv4/IPv6 activity, despite `DISPLAY` being set.
- Local-only evidence: [socket/library trace](/tmp/wf-wayland-audit.87Xxtf/connection-trace.log), [probe log](/tmp/wf-wayland-audit.87Xxtf/connection-probe.log), [identity](/tmp/wf-wayland-audit.87Xxtf/identity.txt), [assert-based self-check](/tmp/wf-wayland-audit.87Xxtf/selfcheck.sh), and dependency/loader diagnostics in the same private directory (mode 700). These temporary raw artifacts may expire; this ticket records the findings and exact tested identities.
- Limits: three-second startup/presentation path only, not every delayed runtime path. The development shell still contains SDL3; this does not establish an SDL-free final editor/build environment. Clipboard/dialog service crates were not exercised or audited, and no new visual/input/fractional-scale pass is inferred.

### Native Wayland mixed-scale pointer/caret and output movement — pass

- User confirmed the other presentation session was paused or finished before the scale change, then manually set upper DP-2 (2560×1440) to 125%, leaving lower DP-1 (5120×1440) at 100%. `kscreen-doctor` independently confirmed scales `1.25` and `1`; DP-2's resulting logical size is 2048×1152. The agent did not alter display settings.
- Same reviewed window-probe commit `f8dacae00279e014595f9aacb61755ee44cab793`, probe lock and executable hashes as the runtime audit above; executable hash verified unchanged before launch. Native KDE Wayland, `--backend wayland --borderless`. **Opaque-padding workaround remains enabled; passing does not qualify production presentation.**
- Human checklist: pointer targeting of the green strip and resize edges/corners on the 125% monitor; cyan caret aligned with the end of throwaway text; movement to the 100% monitor and back, checking for stale areas, misplaced text/caret and offset pointer targets. User explicitly answered **“All passed”** for this checklist. The probe's click-to-end behaviour is intentional; arbitrary character-position hit-testing was not tested.
- Independent [local run-log evidence](/tmp/heavy-xs-window-probe-mixedscale.WpJYhl.log): real `scale factor changed` events at `1.000` and `1.250`, with successful presented frames at both scales and transitions in both directions. Only backend/scale/frame metadata was inspected; raw typed text was not copied. No whole-editor drawing or performance result is inferred.
- Both outputs remained at their test scales in the post-check query; restoring upper DP-2 to its previous 100% remains a user action if desired.

### Native Wayland clipboard/dialog runtime dependency audit — pass within exercised scope

- Reviewed loop-probe commit `4b275ae70903f9fd7b828c4f968f277c5ef67f1b`; probe lock `docs/wayfinder/assets/sdl3-loop-probe/Cargo.lock` SHA-256 `a945885930dbfd9ad8b1c06acc97d21756b9b93bb7de0ed913ee9c8cbb380085`; freshly built executable SHA-256 `773e91ba9234caeb2bbc81578b4420322f19193fc4512e97927d576cf697683e`. Locked build and launcher syntax checks passed; normal tests: **13 passed, 0 failed, 2 ignored**. Ignored service tests were not run. Lock and executable hashes remained unchanged during the live audit; probe worktree stayed clean.
- Native KDE Wayland, decorated, forced `--backend wayland` with `DISPLAY` unset in the child. Runtime reported scale `1.000`, successful frames and clean exit for the main process and traced threads. **Opaque-padding workaround remains enabled; this service audit does not qualify production presentation.**
- User explicitly approved replacing clipboard/primary with fixed public markers and opening/cancelling one dialog per API, then confirmed **“Completed all steps”** for F2, F4, rfd Open/Cancel, F6, ashpd Open/Cancel, Escape. Service logs confirm successful clipboard/primary setters, both dialog requests and completed no-selection results. ashpd explicitly reports cancellation. rfd reports `no-selection(cancel-or-error)` with `fallback-zenity=false`; its log alone cannot distinguish cancellation from an error, so the cancellation observation is attributed to the human. No clipboard read action was exercised.
- Traced the exact binary with `strace -f -ttt -s 256 -e trace=open,openat,mmap,socket,connect` in the pinned probe shell (including its probe-only dbus dependency). No clipboard/D-Bus payload syscalls or screenshots were captured. Successfully opened/mapped libraries are distinguished from failed loader-search candidates; no `libSDL` opens/loads were observed on these paths.
- Runtime evidence: `libwayland-client.so.0` and `libxkbcommon.so.0`; rfd loads `libdbus-1.so.3` and its `libsystemd.so.0` dependency. ashpd's zbus/Tokio implementation is compiled into the binary, not a dynamically loaded `libzbus`/`libtokio`. Captured locked feature trees confirm arboard `wayland-data-control`, rfd `xdg-portal`/`wayland`, and ashpd `file_chooser`/`raw_handle`/`wayland` with Tokio/zbus support.
- All six socket connections succeeded: four to Wayland and two to the session bus (rfd and ashpd phases). Xlib libraries loaded eagerly before the first Wayland connection, but there were **no X11 or TCP connections**. The payload-free trace establishes library/transport use, not exact D-Bus methods or marker contents on the wire.
- Local-only evidence in the private mode-700 directory: [sanitized audit summary](/tmp/heavy-xs-loop-probe.CW5IeE/runtime-audit-summary.txt), [syscall trace](/tmp/heavy-xs-loop-probe.CW5IeE/20-trace-wayland.log), [service log](/tmp/heavy-xs-loop-probe.CW5IeE/22-live-probe.log), [runtime self-check](/tmp/heavy-xs-loop-probe.CW5IeE/runtime-selfcheck.sh), and captured identity/build/test/dependency/feature artifacts alongside them. The coordinator reran the artifact assertions successfully after the analysis helper timed out; no live rerun was required. Temporary artifacts may expire; this ticket preserves the findings and identities.
- Scope limits: exercised Wayland setters and Open/Cancel only, not clipboard reads, Save/folder dialogs, native X11, sandboxing, or persistence without a clipboard manager. No final-editor/build-environment guarantee or calibrated performance claim is made. Klipper and display settings were not changed during this audit; previous clipboard/primary contents were not restored by the agent.

### Native X11 verification deferral

- Explicit user decision: “Skip the native x11 verification. We can address it as it arrives because xwayland works fine”. This revises the native X11 desktop-verification gate in [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md#resolution), using this ticket's allowance for an explicit user-approved disposition.
- **Deferred, not passed:** all native X11 verification in this ticket, including window/input/IME, clipboard/primary in both directions, rfd/ashpd dialogs and service runtime traces. Existing XWayland results remain evidence of X11-client behaviour under the Wayland compositor, not a native X11-session pass. No logout or proactive native X11 test session is required by this effort; native X11 problems will be addressed if they arise.
- **X11 support is retained.** This is a validation deferral, not approval to drop the backend, change the Lua-visible contract or add an SDL fallback. Native X11-specific integration differences may remain undetected; the user accepts proceeding on the existing XWayland evidence.
- **Selection/handoff effect:** lack of native X11 verification no longer blocks this desktop-checks ticket, stack selection or the migration handoff. The migration plan must carry native X11 as an explicitly accepted unverified path with reactive follow-up, not silently report it as tested or reinstate proactive verification as a blocking requirement. Other blockers and acceptance criteria are unchanged.
- Sandboxed dialogs, clipboard/primary survival without a manager, touch and remaining shortcut coverage are not native-X11-only checks and are **not** waived by this decision. The separate supported-Wayland-presentation blocker is unchanged.

### Resolution

The user approved resolving this ticket at **planning/feasibility scope**, using the evidence already collected rather than extending the manual probe checklist. After explicitly approving the four handoff dispositions, the user requested a reassessment of test necessity and then approved the resulting cleanup: “Ok lets do that”. This is not certification of a completed SDL replacement.

**Observed evidence accepted for planning:** the bounded human reviews and runtime audits above cover native Wayland 1× borderless/decorated behaviour, window controls/focus, reported keyboard-layout input, arrow-event delivery, 100%/125% pointer/caret alignment and movement between outputs, plus the exercised window/clipboard/dialog runtime paths. The earlier [window/input](29-prototype-window-scale-ime.md#resolution) and [loop/services](30-prototype-loop-and-dialogs.md#resolution) resolutions supply the already-reviewed IME, XWayland and service evidence. Their exact versions, metadata and limitations remain authoritative; no unobserved path is counted as passed.

**User-approved dispositions for checks not performed here:**

| Check | Disposition and handoff requirement |
| --- | --- |
| Editor shortcuts | Validate modifier/chord translation and editor-command correctness in the migrated editor's contract/live checks, not by adding editor commands to the standalone probe. This does not block planning. |
| Clipboard/primary survival without a manager | Carry as required isolated-session validation in the handoff, without disturbing Klipper on the current desktop. Compare with the existing SDL behaviour before promising additional persistence; no survival guarantee or pass is inferred from the manager-assisted runs. This does not block planning. |
| Touch | Retain touch support and carry the unverified hardware path into the handoff. Perform real hardware validation when suitable hardware is available; do not infer success or silently drop support. This does not block planning. |
| Sandboxed dialogs | Conditional on an actual supported sandboxed distribution/package and its permissions. No new sandbox package or sandbox test is required merely to choose the platform stack. If such a package is supported, validate dialogs in that genuine confinement; host portal success is not sandbox evidence. |
| Native X11 | The [explicit native X11 deferral](#native-x11-verification-deferral) remains in force: retain support, carry the accepted-unverified path, and address native-session issues if they arise rather than reinstate a proactive verification gate. |

**Effect on the map:** this desktop-checks ticket no longer blocks stack selection. The migration handoff must preserve these distinctions between observed passes, deferred validation and conditional requirements. Full-editor contract/drawing validation, actual OS event-path checks and calibrated cost gates remain implementation acceptance requirements; no all-passing release claim or production adoption is approved here.

**Separate blocker unchanged:** the probes still use the opaque-padding workaround. [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md) remains responsible for a supported presentation route or an explicit alternative disposition. Its claim and dependencies are unchanged; resolving this ticket selects no stack and approves no workaround.

No new live tests, service/display changes, sandbox packaging or production edits were performed to resolve the remaining planning gates.
