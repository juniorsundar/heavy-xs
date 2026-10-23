# Calibrate platform performance and memory gates

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-sdl3-cost-calibration
Blocked by: 26

## Question

Given measured SDL costs under controlled conditions, which numerical tolerances define an acceptable platform replacement for cold startup, scrolling/typing frame work, idle CPU and wake-ups, and memory during window open/resize/close cycles?

Collect the baseline facts before asking the user to choose tolerances. Record the source/build identity, actual backend (Wayland or X11), surface density, window dimensions, display refresh, workload, repeats and measurement method. Separate active work from presentation and identify environmental noise and unavailable measurements. Agree the workloads, comparison method and numerical tolerances with the user before judging candidate results. Do not reuse font or regex budgets, silently excuse accumulating memory or idle busy-spin, or infer editor performance from a standalone window prototype.

Planning only: baseline collection supports a decision; it authorizes no production change. Keep a concise Markdown record, not a collection of raw evidence files in the main tree.

Context: [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md#resolution). Its qualitative no-regression requirements remain in force; this ticket supplies numerical calibration for the [implementation handoff](28-plan-sdl3-migration.md).

## Comments

### Agreed measurement method (round 1)

The user answered “yes to all” to the following. These fix *how* SDL is measured; no tolerance is chosen yet.

1. **Presentation split.** Re-add a presentation-time counter (as in the font prototype) in a separate measurement build only; the main line stays unchanged. Active work and presentation are reported separately.
2. **Workloads**, each under both the bundled config and the user's own config (fonts, theme, plugins), on a plain-text document so no language server attaches:
   - **Startup:** launch to first presented frame, then exit.
   - **Scrolling and typing:** the font work's frame probe, editor window maximised.
   - **Idle:** 60 s focused (caret blinking) and 60 s unfocused; CPU time, loop wake-ups and context switches.
   - **Memory:** about 50 cycles of opening, drawing, resizing and closing a second window, plus main-window resizes; RSS and peak RSS.
3. **Display and backend.** Laptop panel only (2560×1600, scale 1.25, 165 Hz), external monitors disabled during runs. SDL's native Wayland backend is the primary baseline; XWayland is a second set.
4. **Startup means warm-cache.** Discard the first run, take the median of about 10. True cold-cache startup needs root to drop the page cache, so it is recorded as unavailable, not waived.
5. **Run structure.** A/B/B/A over at least 3 rounds; report the median of each round's p95 (frames) or median (startup, idle). Disturbed runs are excluded with the reason recorded. The first pair is **SDL against SDL**, to measure run-to-run noise before tolerances are set.
6. **Memory method.** RSS and peak RSS from `/proc` plus Lua's GC count, no allocator shim. Accumulation is judged by RSS staying flat over the last 30 of 50 cycles; absolute size is reported separately.

Next: collect the SDL baseline with the user's desktop idle, then put tolerances to the user.

### Method changes agreed or forced since round 1

- **Idle is 20 s, not 60 s,** with focused and unfocused as separate runs (user agreed, to keep the baseline near 30 minutes per backend).
- **Fullscreen instead of maximised.** The desktop runs KWin with the Krohnkite tiling script, which turns *maximise* into a tile (1261×1474 px, about half the panel), so maximised sizes depend on the desktop layout. Frames and idle runs use fullscreen and wait for the size to settle; the measured surface is 2560×1545 px (the compositor reserves a strip). Startup keeps the editor's own default window (2048×1280 px; 13% of launches opened at 2560×1499).
- **Unfocused idle is sampled from outside the process.** Once minimised and unfocused, the editor's loop blocks in `system.wait_event()` with no timeout, so an in-process probe never resumes. The driver reads CPU time and context switches from `/proc/<pid>` over 20 s, then ends the process. Loop-level counts (wait calls, frames) are therefore unavailable for unfocused idle.
- **Probe fixes found by prechecks on the real desktop:** declare the hook's Lua global for `core.strict`; give every launch a timeout of about twice its expected length, so a hang fails in seconds; write the unfocused-ready marker atomically. None changes what is measured.

### SDL baseline — Wayland (native SDL backend)

Collected unattended (user away, desktop idle, lock and notifications inhibited for the run) on 2026-10-19, 17:32–18:42. Identity: a release build of `f52aa273` with only the presentation-time hook added, binary SHA-256 `8214856800d6…`; system SDL 3.4.14; laptop panel eDP-1 only, 2560×1600 at scale 1.25, 165 Hz. Three rounds of A/B/B/A with A and B the **same build**, both configs, 360 runs, **no failures**, no focused-idle run lost focus. The B−A column is run-to-run noise.

Medians across the three rounds (A / B):

| workload | metric | bundled config | user config |
|---|---|---|---|
| startup | launch to first presented frame | 193 / 197 ms | 220 / 217 ms |
| startup | RSS at first frame | 106 / 106 MiB | 108 / 108 MiB |
| scroll (1 line/frame) | p95 active / present | 9.6 / 4.6 ms | 10.7 / 4.7 ms |
| scroll | median active / present | 4.1 / 3.5 ms | 4.4 / 3.4 ms |
| scroll_page (40 lines/frame) | p95 active / present | 12.4 / 5.6 ms | 13.2 / 5.3 ms |
| type | p95 active / present | 7.2 / 3.0 ms | 7.9 / 2.8 ms |
| type | median active / present | 3.2 / 1.1 ms | 4.0 / 1.2 ms |
| idle, focused | CPU | 4.6 % | 5.1 % |
| idle, focused | drawn frames | 2.5 /s (caret blink) | 2.5 /s |
| idle, focused | `wait_event` calls (sub-ms polls) | 1,500–1,600 /s (about 1,400 /s) | 1,150–1,250 /s (about 1,050 /s) |
| idle, focused | voluntary context switches | 121 /s | 121 /s |
| idle, unfocused | CPU | 0.03–0.13 % | 0.13–0.17 % |
| idle, unfocused | voluntary context switches | 2–3 /s | 2.5 /s |
| memory | RSS after first cycle → at cycle 50 | 148 → 181 MiB | 146–150 → 182 MiB |
| memory | RSS growth per cycle, last 30 cycles | **about 730 KiB** | **700–760 KiB** |

Run-to-run noise between identical builds, as the largest round-median difference: startup about 4 ms (2%); frame p95 active up to 0.8 ms and p95 present up to 0.3 ms; idle CPU up to 0.2 percentage points; wake-up counts up to 10%; peak RSS under 1 MiB. Individual rounds vary more than their medians: scroll p95 active ranged 6.2–10.6 ms within the bundled config.

### Finding: SDL itself grows memory on every fullscreen toggle

The memory workload does **not** stay flat: RSS rises about 0.7 MiB per cycle, linearly, with no sign of levelling off over 200 cycles. The growth is anonymous private memory, not fragmentation (glibc trimming settings do not change it), and the Lua heap stays flat. Isolating it:

- The second window's open/draw/resize/close is flat. Explicit main-window size changes grow about 0.3 MiB per cycle; **toggling fullscreen ↔ normal grows about 0.7–1 MiB per cycle**, even when the editor draws nothing.
- **A 40-line SDL-only C program reproduces it** with no editor code: toggling fullscreen on an SDL window that draws through its window surface grows about 695 KiB per cycle, on both Wayland and X11. The same program without drawing does not grow.
- **It comes from SDL's framebuffer acceleration.** With `SDL_FRAMEBUFFER_ACCELERATION=0`, or with the framebuffer render driver set to `opengl`, the growth disappears (0–1 KiB per cycle). With the editor's own setting, `SDL_RENDER_DRIVER=software`, it remains. So the accumulating memory is in SDL 3.4.14's window-surface path, not in Lite XL.

Consequence for the gate: the agreed requirement “no accumulating memory” **already fails for SDL** in the resize workload. A replacement cannot be judged by comparison with SDL's slope; it needs an absolute requirement. The reproduction is kept locally (`sdl-fullscreen-surface-leak.c`) and is the start of an upstream SDL report; it is not committed to the main tree.

### Finding: the editor's idle loop spins on its own

Focused idle costs about 5% CPU and calls `system.wait_event` 1,100–1,600 times per second, most with a timeout under 1 ms. Those are truncated to 0 ms, a non-blocking poll. The cause is in Lua, not SDL: a plugin thread that is nearly due shortens the wait to microseconds. The treeview (10 ms) and autoreload (50 ms) polling threads dominate; disabling both cut wake-ups by 80–90% in an offscreen check. Unfocused, the loop blocks without a timeout and costs almost nothing. A replacement must reproduce this loop behaviour, not fix it; fixing it is a Lua-side behaviour change outside this migration.

### Not collected yet

- **XWayland set** (second backend): not run in this session.
- **Cold-cache startup:** unavailable without root, as agreed.
- **Unfocused loop counts** (wake-ups, frames): unavailable by design, see above.

### Shape of the gate (round 2)

The user accepted all five points:

1. **Paired and relative.** Each candidate is measured A/B/B/A against SDL in the same session, on the same machine and build base. A tolerance limits the candidate − SDL difference and is the larger of an absolute floor and a percentage. The SDL-vs-SDL noise measured above is the floor below which no limit is set; today's absolute numbers are context only.
2. **Absolute limits where SDL is no reference.** Unfocused idle (SDL is near zero) and memory accumulation (SDL itself grows) get absolute limits that the candidate must meet on its own. SDL failing the accumulation limit is recorded as a finding, not a reason to relax it. Absolute memory size stays relative to SDL.
3. **Exceeding a limit.** Within tolerance counts as not material and needs no explanation. Beyond tolerance is not an automatic fail: the cause is explained and the user approves or rejects it, as for any difference under [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md#resolution). A result within one noise floor of a limit is rerun with 5 rounds before judging; the verdict uses the median across rounds.
4. **No separate XWayland baseline.** Tolerances come from the Wayland noise. The first paired XWayland run is the check: if its SDL-vs-SDL noise exceeds the Wayland floor, the limits are revisited before any candidate is judged. Native X11 stays with the [desktop checks](33-complete-platform-desktop-checks.md).
5. **Both findings stay outside this migration.** The idle-loop spin is a Lua behaviour change, ruled out of scope on the map; reporting the SDL leak upstream is the user's call. A replacement that avoids the SDL leak is a pass, not a behaviour change.

A full baseline set took about 70 minutes per backend, not 30, because startup needs 44 launches per config per round.

Next round: the numbers. Original plan was to put tolerances to the user, using the noise floor above, plus the two findings (an absolute memory requirement instead of a relative one, and idle compared against SDL's spin rate rather than zero).

### Resolution

The user accepted the gate shape (round 2 above) and every proposed number. A candidate is measured A/B/B/A against SDL in the same session, at least 3 rounds; each limit applies separately to every config (bundled, user), every phase and both backends (Wayland, XWayland), and is one-sided, so any improvement passes. Limits are about 2–3× the SDL-vs-SDL noise measured in the Wayland baseline above.

| gate | statistic | limit on candidate − SDL (or absolute) |
|---|---|---|
| Warm startup, launch to first presented frame | median | slower by at most **max(10 ms, 5%)** |
| Frame active work (scroll, scroll_page, type) | p95 | slower by at most **max(1 ms, 10%)** |
| Frame presentation (same phases) | p95 | slower by at most **max(1 ms, 20%)** |
| Focused idle, drawn frames | median | **equal** to SDL (2.5/s caret blink); a difference is a behaviour change, not a cost |
| Focused idle, CPU | median | at most **+0.5 percentage points** |
| Focused idle, `wait_event` calls/s | median | at most **+20%**; fewer passes if the Lua-visible `wait_event` results are unchanged |
| Focused idle, voluntary context switches/s | median | at most **+10%** |
| Unfocused (minimised) idle | median | absolute: **≤ 0.5% CPU** and **≤ 10 voluntary context switches/s** |
| Memory accumulation | RSS, last 30 of 50 cycles | absolute: growth **≤ 1 MiB**; if exceeded, a 200-cycle run decides, passing only with **≤ 1 MiB** growth over its last 50 cycles |
| Memory size | RSS at first frame; peak RSS over the first 20 cycles | higher by at most **max(5 MiB, 5%)** |

Only p95 frame times and medians for startup and idle are gated; frame medians and totals are reported, not gated. Beyond a limit is not an automatic fail: the cause is explained and the user approves or rejects it. A result within one noise floor of a limit is rerun with 5 rounds, judged on the median across rounds. If the first paired XWayland run's SDL-vs-SDL noise exceeds the Wayland floor, the limits are revisited before any candidate is judged.

**SDL itself fails the accumulation gate** (about 21 MiB over the last 30 cycles, from its framebuffer-acceleration path on fullscreen toggles); this stays a finding, not a relaxation. The idle-loop spin is ruled out of scope on the map. Cold-cache startup is unavailable without root and stays pending, not waived.

**Source:** a measurement build of the main line with only a presentation-time hook, driven by a probe script with A/B/B/A scheduling and a summary script; the SDL-only leak was reproduced with a small standalone C program. No editor code changed. These tolerances feed [Define the SDL3 replacement handoff](28-plan-sdl3-migration.md).
