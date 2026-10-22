# SDL3 Step 4 paired cost results

Unattended paired runs of the native cutover against SDL, 2026-10-22 18:02–22:10, under the [calibrated gates](../issues/31-calibrate-platform-cost-gates.md#resolution). A is SDL, B is the candidate; A/B/B/A, both configs (bundled, user). Verdicts use the 3-round medians, replaced by 5-round medians where a result was within one noise floor of its limit.

## Identities and deviations

- **SDL (A):** `prototype/sdl3-cost-baseline` at `4b131a63` (the baseline branch's later commit, which only adds an unused allocation-count hook), binary SHA-256 `5f7dfb65f8a0…`, system SDL 3.4.14.
- **Candidate (B):** `d25d4768` plus the same presentation-time hook, local throwaway branch `prototype/sdl3-cost-candidate`. Default (X11) build `17a878e7f519…` for XWayland; `--features wayland` build `b20769cf1913…` for native Wayland.
- **Display: not the calibrated one.** The laptop panel (eDP-1, 1.25×, 165 Hz) was disabled. Runs used DP-8, 3440×1440 at 60 Hz, scale 1, with DP-7 also connected. The editor drew full screen at 3440×1440 for frames, idle and memory on both builds.
- **Probe changes, applied to both builds:**
  - The driver clears `LD_LIBRARY_PATH`: the dev shell's newer alsa-lib needs glibc 2.43, and the SDL build runs on 2.42.
  - The memory workload frees its second window at once. Under winit, KWin focuses a second window that is still open, and the unfocused editor then blocks in `wait_event()` with no input to wake it.
  - The first full-screen request waits 0.5 s. KWin (with Krohnkite) drops a full-screen request made while a restored maximized state is still being applied, and SDL drops one made before the first frame too.
  - The memory timeout is doubled for the 200-cycle run.
- **Sleep and lock** were inhibited for the run. Nothing else was running on the desktop.
- Raw TSVs were left in the session scratchpad under `/tmp`, so they won't survive a reboot. The numbers below are copied from them.

## Gate verdicts

Values are candidate − SDL (or absolute where the gate is absolute). Bundled / user config.

| Gate (limit) | Native Wayland | XWayland (default build) |
| --- | --- | --- |
| Warm startup median (≤ max(10 ms, 5%) slower) | **pass**: −148 / −138 ms (69 / 100 ms vs 217 / 238 ms) | **pass**: −141 / −139 ms |
| Frame active p95 (≤ max(1 ms, 10%)) | 3 rounds: pass (−1.25…−0.21 / −4.03…−0.43 ms). 5-round user rerun: **over**, scroll +1.43, page +2.00, type −0.07 ms | 5-round rerun: **over**, scroll +1.03 / +1.63, page +1.35 / +2.10; type pass (+0.51 / +0.64) |
| Frame presentation p95 (≤ max(1 ms, 20%)) | scroll and page pass (faster). Typing **over**: +1.79 (3 rounds) / +2.07 (5-round user) | scroll and page pass. Typing **over**: +3.11 / +2.46 (5 rounds) |
| Focused idle drawn frames (equal) | **pass**: 2.5 /s | **pass**: 2.5 /s |
| Focused idle CPU (≤ +0.5 pp) | **over**: +0.88 / +0.80 (5-round user) | **over**: +1.08 / +1.10 |
| Focused idle `wait_event` calls (≤ +20%) | **pass**: +13% / +9.6% (5-round user; 3-round +26.6% was noise) | **pass**: +3% / +6% |
| Focused idle voluntary switches (≤ +10%) | **pass**: −1% | **pass**: +8.3% / +8.2% |
| Unfocused idle (≤ 0.5% CPU, ≤ 10 switches/s) | **pass**: 0.20 / 0.43%, 1.0 / 1.2 /s | **pass**: 0.23% / 0.17% (5-round user), ≤ 3.2 /s |
| Memory accumulation (≤ 1 MiB over last 30 of 50) | **pass**: 0 / 6 KiB | bundled 3-round median 2.2 MiB → 200-cycle run: 0 KiB and **4.3 MiB** over the last 50 (one step at cycle 162, flat before and after). User **pass** (0) |
| Memory size (≤ max(5 MiB, 5%) higher) | **pass**: first frame −71 MiB, peak −83 MiB | **pass**: first frame −66 MiB, peak −97 MiB |

SDL still fails accumulation: about 30 MiB over the last 30 cycles, and about 35 MiB over the last 50 of 200.

## Explanations for the over-limit results

1. **Typing presentation.** On every changed frame the candidate copies the whole visible frame (3440×1440). SDL updated only the dirty rectangles. A one-character edit therefore costs a full copy: the typing median rises from about 1.3 to 2.6–2.8 ms. The same design makes scroll presentation faster than SDL. This is the full-copy, no-buffer-age design selected in [Choose SDL-free pixel storage and presentation conversion](../issues/34-choose-sdl-free-pixel-storage.md#resolution). Recovering the cost needs damage-region copies with buffer-age tracking, which is a design change and has not been made.
2. **Scroll active work (+1–2 ms at p95).** A per-frame timer around the native event pump (diagnostic build, `prototype/sdl3-cost-pumptime`) shows the pump takes about 0.005 ms median and 0.01–0.02 ms p95 inside a frame on both backends. The extra time is therefore not winit's event loop. It is in the editor's own frame work (Rust drawing into the pixel store, or work that differs around it). The cause is not yet identified; `perf` is not installed. The 3-round native Wayland set showed the opposite sign, so this metric also varies a lot between sessions.
3. **Focused idle CPU (+0.8–1.1 pp).** The editor's own polling loop (out of scope, reproduced unchanged) makes 700–1,000 non-blocking `wait_event` calls a second, each one event-loop pass. The candidate uses more CPU at about the same call rate, so each winit pass costs a few microseconds more than SDL's. The handoff forbids changing timeout behaviour or the loop rate.
4. **XWayland memory step.** RSS rises in single steps (cycles 1, 2, 55, and in one of two 200-cycle runs 162) and is otherwise flat, consistent with the allocator reserving a larger block, not a per-cycle leak. On the literal rule, one of the two adjudication runs fails.

Each needs the user's approval or rejection under the gate rules. None has been waived.

## Behaviour findings during the run (no code kept)

- **Second windows take focus under winit.** On KWin, a newly shown second window (`renwindow.create`) gets keyboard focus; under SDL it did not. If such a window stays open and no input arrives, the unfocused main window blocks in `wait_event()` by design. Closing the window returns focus. Observed on both backends.
- **Restored maximized state on native Wayland arrives after the first frame.** SDL draws its first frame already maximized (2752×1152 here). The candidate draws at the normal size (2048×1152) and is maximized shortly after, so startup extents differ on native Wayland only. A fix that re-applied early mode requests after the first frame made no measurable difference and was dropped.
- **Early full-screen requests are lost by KWin for both builds** when made before the first frame. This is compositor behaviour, not a difference between the builds.
