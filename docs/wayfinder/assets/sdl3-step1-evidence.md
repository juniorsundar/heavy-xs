# SDL3 replacement, Step 1 evidence

Evidence for [Step 1 — Reference and isolated test preparation](sdl3-migration-handoff.md#step-1--reference-and-isolated-test-preparation), collected on 2026-10-21 on Linux (NixOS), with no live desktop interaction. This step changes only tests and test tooling. The production editor still uses SDL, and nothing here certifies a replacement.

**Status: implemented, uncommitted. One review is pending:** the matched private-Xvfb differences below need the user's disposition before Xvfb-profile expectations exist. Nothing has been normalized or re-recorded.

## What changed

| Area | Change |
| --- | --- |
| [Runner](../../../scripts/rust-port/verify.sh) | Builds the hook-enabled contract executable into `target/contract` (`--locked`), so the developer's `target/debug`, install and `stage/` are never touched. Runs each suite as a fresh 60 s-bounded process under `PROFILE=offscreen` (the historical goldens' driver) or `PROFILE=xvfb`. Records the video backend SDL actually chose, using SDL's own `system=debug` log line, then removes exactly those SDL lines before the diff. A backend other than the profile's fails the suite. Starts each suite with SIGINT at its default disposition. `BIN`/`STAGE_DIR` run a prebuilt editor (the reference). `EVIDENCE_DIR` keeps outputs, backends and provenance. `RECORD=1` writes passing outputs to a review area only. The `LXL_TEST_C_EVENTS` recording path is gone. |
| [Private environments](../../../scripts/rust-port/private-display.sh) | The runner owns every profile. The child loses `DISPLAY`, `XAUTHORITY`, the Wayland variables, the IME variables, `LD_LIBRARY_PATH` and `LD_PRELOAD`. It gets a private 0700 runtime dir and a session-bus path that does not exist. For `xvfb` the runner picks a free display, writes its own cookie, starts Xvfb with `-nolisten tcp`, waits up to 10 s for `-displayfd` readiness, and stops only the PID it started. Without Xvfb it fails rather than using another display. |
| [Startup checks](../../../scripts/rust-port/startup-check.sh) | All three cases run through the private environment. Restart and quit run under `STARTUP_PROFILE=dummy` (default) or `xvfb`. The failure case always runs without a display. Takes `BIN`/`STAGE_DIR`. |
| [Hit-test check](../../../scripts/rust-port/hit-test-check.sh) | By default it compares the Rust grid with the recorded identity of the C grid and needs no SDL. `HIT_TEST_REFERENCE=c` also rebuilds the SDL-header C comparator as external reference evidence. |
| [Normal-build check](../../../scripts/rust-port/normal-build-check.sh) | New. Builds the ordinary configuration into `target/normal` and checks that it ships no `lxl_test`, no hook code and no `LXL_TEST_C_EVENTS` effect, and that it exports all 41 `system` functions. |
| [Reference builder](../../../scripts/rust-port/sdl-reference.sh) | New. Builds the pinned reference in a detached worktree under `target/sdl3-reference`, stopping if the commit, cleanliness or lock digest is wrong. Stages it with the reference's own install script and records provenance. Runs the probe, both suite profiles, both startup profiles, the hit-test comparator and the process-timing classification. |
| [SDL probe](../../../scripts/rust-port/fixtures/sdl_reference_probe.c) and [expectations](../../../scripts/rust-port/sdl-expectations/README.md) | Independent SDL observations, checked in before SDL is removed: rect fill/blend/clip rounding and channel packing (`draw.txt`), multi-click state and count (`clicks.txt`), and the C hit-test classifier grid (`hit-test-grid.txt`). |
| Source and fixtures | Removed the `LXL_TEST_C_EVENTS` early return from `src/system.rs` and the C-only branch from `tests/contract/cases/sysevents.lua`; the recorded value is unchanged. Corrected the stale C-baseline comments in `src/test_hooks.rs` and the process/sysevents cases. No production behaviour changed. |
| [flake.nix](../../../flake.nix) | Adds `xvfb`, `xauth` and `util-linux` (for `mcookie`) to the dev shell only; the package closure is unchanged. Both derivations evaluate. |

None of the sixteen goldens changed: their combined SHA-256 is `5d39e264…acbb5aa7` before and after.

## Reference identity

| | |
| --- | --- |
| Source | `c80459bc0f464df68b8d8cdff553cd8288519a67`, a clean detached worktree. It differs from the working tree's `HEAD` (`32eeabbb`) only in documentation. |
| Cargo.lock | SHA-256 `1a2b1485592910d3bc588c04d8a7f12f0d285b9997a6cab91eac66a718f72a35` (matches the handoff) |
| Build | debug, `--features test_hooks --locked`, rustc 1.97.1 / cargo 1.97.0 |
| Binary | SHA-256 `3fe9e7db20ed272aa03cfce20768d66c27b53b25e3412eb630efa5f200d36e36` |
| SDL | Bindings: `sdl3-sys 0.7.2+SDL-3.4.18`. Linked runtime: **SDL 3.4.14** (`SDL_GetVersion`, revision `SDL-3.4.14-GIT-NOTFOUND`) from `/nix/store/63ai3hhg…-sdl3-3.4.14-lib`. The editor and the probe resolve the same library. |
| Fixtures | Contract cases SHA-256 `729b1441…5e29957a`; plugin fixtures `00b27121…08e405`; staged fonts `ae4d6bdd…ea2c274` (identical in the reference and candidate stages) |
| Profiles | `offscreen`; `xvfb` = private Xvfb at 1280x800x24, dpi 96, backend recorded as `x11`. Window surface format is ARGB8888 on both. On Xvfb, SDL's window framebuffer goes through its Vulkan renderer (`SDL chose render backend 'vulkan'`). |

The full records stay in `target/sdl3-reference/evidence/` (`reference.txt`, per-profile outputs, `backends.txt`, `provenance.txt`, logs). They are local build products and not committed.

## Results

| Check | Reference (c80459bc) | Candidate (this tree) |
| --- | --- | --- |
| 16 suites, offscreen | 16/16 match the goldens | 16/16 match the goldens; byte-identical to the reference |
| 16 suites, private Xvfb | 11 match; 5 differ (below) | Byte-identical to the reference on all 16 |
| Startup checks, dummy and Xvfb | 12/12 each | 12/12 each |
| Hit-test | Rust grid = recorded C grid = rebuilt C comparator (1,239,848 points) | Rust grid = recorded C grid (no SDL) |
| SDL probe expectations | `draw.txt` matches on offscreen and Xvfb; `clicks.txt` matches on Xvfb (three independent runs agreed) | not applicable |
| Normal build | not applicable | no `lxl_test`, no hook strings (the hook build has 4), the retired variable has no effect, 41 `system` exports |

Failure propagation was exercised. A suite that errors exits 2 and FAILs; `RECORD=1` records nothing for it. A missing Xvfb fails before any suite runs. After every run, no Xvfb process is left and the child environment holds only the private display, cookie, runtime dir and bus path. The backend-mismatch branch was not exercised: the runner cannot produce a mismatch on demand.

## Classified: the process timing failure

The inventory recorded one `process` failure, `interrupt then dead`. It **reproduces deterministically on the reference** when the editor starts with SIGINT ignored. That happens to a background job of a non-interactive shell, the situation `startup-check.sh` already guards against. The ignored disposition survives `exec` into the child `sleep 5`, so `interrupt()` can't end it, and `wait_dead` gives up after 3 s. With the default disposition it passed 20/20 on offscreen and 20/20 on Xvfb, for both reference and candidate.

Classification: a **runner-environment input, outside the platform layer**. It is not a candidate regression and not a waiver. The runner now sets the disposition explicitly. `SUITE_SIGINT=ignore` reproduces the failure, and `sdl-reference.sh` records it as a classified observation (`process-sigint-ignored.log`). Inheriting SIG_IGN into children is existing editor behaviour. It is left to the signal resolution's "previous disposition" checks in [Choose process-signal quit and wake integration](../issues/37-choose-process-signal-quit-integration.md#resolution), not changed here.

## Pending review: matched SDL/Xvfb differences

These differ from the historical (offscreen) goldens. They are identical for the reference and the candidate, so they come from the profile, not from the implementation. Per [Design SDL-free platform contract testing](../issues/36-design-sdl-free-contract-testing.md#resolution), each needs an explained, reviewed expectation. No blanket normalization is allowed. **No Xvfb expectations have been written.**

1. **Mesa notice (rencache, renderer, sysevents, sysplugin, syswindow).** Mesa prints two stderr lines, `MESA: info: vulkan: No DRI3 support detected…` and `Note: you can probably enable DRI3…`, when SDL's X11 framebuffer path creates its Vulkan (software) renderer on Xvfb. This is driver output from the profile, not editor output. A candidate without SDL will not produce it.
2. **rencache, asynchronous resize.** After `set_window_size`, X11 reports the old size (`800.0x480.0` vs `400.0x300.0`), and the resize and grow-back frames have no dirty rect. Offscreen applies the size synchronously.
3. **syswindow, window manager-less X server.** On bare Xvfb, `maximized` and `minimized` persist where offscreen reports `normal`. Size and position changes lag (`400,300,…` vs `250,180,…`, and three more size lines).

The decision for the user: accept profile-specific Xvfb expectations for these keys, reviewed per item, or keep offscreen as the only strict golden profile and treat Xvfb outputs as matched reference observations that later candidates are diffed against (`target/sdl3-reference/evidence/contract-xvfb`). Step 4 needs this answer before it can judge the winit/Xvfb candidate. Step 2 doesn't, because it keeps SDL windows and runs on offscreen.

## Not covered by this step

- No desktop, Wayland, portal, clipboard-manager or notification interaction ran. Live evidence remains for the later steps, as the handoff dispositions require.
- `clicks.txt` pins SDL's defaults on Xvfb (500 ms, ±32 px) with no XSETTINGS daemon. Desktop double-click settings were not observed.
- The `draw.txt` blends use a surface in the window's format, not the window surface itself. The result was identical on both drivers.
- The scripts were run with the host's `sh` (bash in POSIX mode). dash was not tried.
