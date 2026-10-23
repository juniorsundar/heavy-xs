# GUI runtime and customisation iteration baseline

Evidence for [Establish GUI runtime and customisation iteration baselines](../issues/42-baseline-gui-and-development-costs.md). This records the current editor, not a candidate design, acceptance budget or optimisation recommendation.

## Scope and provenance

- Source: `e556d388892280d8343a82a625941137e3a6ac5f`; production source, data, tests and Cargo inputs were not modified. Existing developer artifact targets were neither rebuilt nor cleared; cached Cargo sources were reused. Owned detached-worktree and normal-checkout copies were used for build experiments; both sources were restored and checked clean.
- Normal **release** executable, default X11 features, **no `test_hooks` or `wayland`**; thin LTO/codegen-units 1 as configured. Binary SHA-256 `bde5ae899aef0302c41201ad54569f045bdef0db6a46874bd0a6f835642cb186`, size 8,419,392 bytes (8.03 MiB). Input/font/helper hashes and bundled-plugin list are in the [measurement snapshot](gui-cost-baseline/results.json).
- Host: Intel Core i9-12900H, 20 logical CPUs, approximately 31 GiB RAM, Linux x86-64; Rust 1.99.1/Cargo 1.99.1. Builds used four jobs, `--offline --locked`, warm Cargo source caches and initially empty owned artifact targets. No OS cache clearing, network download, governor change, CPU affinity or desktop-idle guarantee.
- Runtime: private **1280×800×24 Xvfb, 96 DPI**, no window manager/compositor, private runtime/user/home/project directories and unreachable session bus, using the existing [private-display helper](../../../scripts/rust-port/private-display.sh). Window **1024×640**, scale 1, default 60 fps. Xvfb/server CPU and memory are not included in editor `/proc` figures.
- Final runtime collection: 2026-10-23 09:47:02–09:48:45 UTC (13:47–13:48 local). **49 normal-runtime probes plus six Lua edit/save/restart probes**. Preliminary smoke/focus-debugging runs are not included in the reported numbers.
- Raw logs and per-frame samples remain in `target/gui-cost-baseline.M60jmM/`; retained data and reproducible helpers are under [gui-cost-baseline/](gui-cost-baseline/). This owned experiment occupies approximately **1.85 GiB**; it was not removed automatically. No user configuration or existing build cache was cleared.

These observations are **not** real-desktop input-to-photon latency, true cold-cache startup, Wayland qualification or compatibility evidence for third-party plugins.

## Workloads and measurement boundaries

| Configuration | Exact workload |
| --- | --- |
| Bundled | All 27 bundled Lua plugin files available under the normal loader/configuration; no plugins disabled by the probe. A generated 2,000-line Lua file, 164,000 bytes, in a one-file project. |
| Custom | Same file and bundled environment; explicitly enables whitespace drawing and line guides at columns 80/100/120, then adds a Lua `View` beside the document, drawing 80 coloured rectangles. This is a specific synthetic customisation, not a third-party plugin sample. |
| Large | Bundled environment with a 20,000-line/1,640,000-byte version of the same repetitive fixture. Size stress only: no claim to represent complex real-world syntax or very long lines. |

The probes run **normal `core.init()` and `core.run()`**, not a replacement headless runtime. Instrumentation is installed through disposable user init. They assert a document view, window dimensions, process exit, absence of `error.txt`, focus handshake and expected sample counts.

### Clocks and overhead

- The outer Node clock measures spawn-to-observed flushed marker. A **first-frame-completion marker** is emitted after the first successful `core.step()` redraw returns. It includes process launch, bootstrap, fonts/plugins, initial layout/drawing, backend submission and pipe-observation overhead. Window mapping is subsequently verified by the X11 helper. It is **not** the time pixels become visible on a physical display.
- After that marker, the driver focuses the private window, waits **30 ms**, and starts a separate XTest helper that taps F12. The Lua event wrapper records receipt and asserts native focus. The *XTest-helper-launch-to-Lua-observation* interval includes helper launch/X11/queue/pipe overhead; the deliberate 30 ms wait is **not** included in that interval. This verifies input handling, not the earliest possible input-ready timestamp or physical keyboard latency.
- Inner timings use `system.get_time()` (`CLOCK_MONOTONIC_RAW`). No subtraction mixes its absolute timestamps with Node's clock. Each frame records action, root update, Lua drawing-command traversal, `renderer.end_frame`, step and action-to-step-end durations. **`end_frame` combines cache/raster/presentation work**, not presentation alone.
- The 10,000-clock-call probe takes **0.36–0.83 ms** across the final runs. That calibration itself is included in startup. Wrappers add calls/dispatch, and every measured frame emits one flushed line **after** its timed section. No uninstrumented control or subtraction of estimated overhead was performed; scheduling/pipe effects remain a limitation.
- Each active run warms up 30 drawn frames, then measures 90 frames per action at the normal loop's pacing. There are three separate processes/configuration. Typing injects text through `core.on_event`; scrolling calls the document view's wheel handler; layout adds a second view of the same document and alternates its split divider. These are **scripted active-work measurements**, excluding prior OS event wait and later compositor/display work.
- Statistics use nearest-rank quantiles. With ten startup samples, p95 is the maximum; it is not a well-estimated tail distribution. Frame tables retain between-process variability rather than hiding it in a pooled percentile.

## Startup and input-handling observations

Fresh editor processes and user/project directories, but already staged files and warm source/OS caches. **Do not call these cold-cache startup results.** Configurations were collected in bundled/custom/large order, not randomised or paired.

| Configuration | Launches | First observed launch | First-frame median | First-frame p95 / max | XTest-helper-to-Lua median |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bundled | 10 | 50.57 ms | **48.19 ms** | 54.44 ms | 2.55 ms |
| Custom | 10 | 106.01 ms | **105.15 ms** | 117.97 ms | 4.95 ms |
| Large | 10 | 65.36 ms | **67.05 ms** | 77.62 ms | 2.63 ms |

The first observation is reported separately for transparency, not as a cold-start estimate. Source ordering makes font loading, synchronous plugin loading and first draw plausible areas to investigate; this experiment does not apportion startup cost or establish a cause for the configuration differences.

## Scripted active GUI work

Milliseconds, from scripted action start to completed step. Each row contains three per-process medians and three p95s, with 90 observations/process/action:

| Configuration / action | Run medians | Run p95s |
| --- | --- | --- |
| Bundled / typing | 2.79, 3.30, 3.46 | 4.43, 6.14, 5.05 |
| Bundled / scrolling | 1.03, 1.24, 1.19 | 2.76, 2.77, 2.72 |
| Bundled / layout | 2.91, 3.20, 3.78 | 4.21, 4.84, 6.39 |
| Custom / typing | 6.40, 6.12, 6.35 | 10.98, 11.20, 11.63 |
| Custom / scrolling | 2.25, 2.41, 2.16 | 7.88, 5.65, 6.80 |
| Custom / layout | 5.43, 5.31, 5.23 | 9.57, 8.36, 8.26 |
| Large / typing | 4.08, 3.96, 3.82 | 7.52, 6.36, 5.71 |
| Large / scrolling | 1.31, 1.25, 1.27 | 3.57, 3.04, 3.44 |
| Large / layout | 3.23, 3.43, 3.19 | 4.03, 4.31, 4.57 |

The snapshot also keeps update/draw/end-frame breakdowns. For bundled typing, median Lua draw-command traversal was 0.44–0.46 ms and end-frame processing 1.33–1.70 ms across runs. For custom typing those ranges were 1.41–1.57 ms and 2.50–2.74 ms. They describe these probes, not general renderer or plugin bottlenecks. Scroll frames can have little dirty-region work; equal counts of `core.step()` redraws do not imply equal numbers of presentation copies.

## Idle CPU and bounded memory evidence

Two processes/configuration; five-second focused and five-second unfocused windows after settling. The driver verifies X11 focus and the Lua `focuslost` event verifies the native focus state. CPU is the editor process's aggregate user+system ticks as **percent of one core**. Context switches come from the main thread's `/proc/PID/status`, not a sum across threads.

| Configuration | Focused CPU, two runs | Focused voluntary switches/s | RSS at interval end | Unfocused interval |
| --- | --- | --- | --- | --- |
| Bundled | 3.40%, 3.40% | 137.45, 136.87 | 22.05–22.06 MiB | No recorded CPU ticks or main-thread switches; RSS flat |
| Custom | 4.00%, 3.40% | 125.40, 133.53 | 22.04–22.15 MiB | No recorded CPU ticks or main-thread switches; RSS flat |

At 100 Hz and five seconds, one CPU tick is approximately **0.2 percentage points of one core**. Zero recorded ticks does not prove zero CPU/energy cost. These intervals do not establish long-duration idle behaviour. Earlier measurements using X11 `PointerRoot` for blur were discarded: the pointer could still leave the editor effectively focused. The final helper explicitly focuses the X root and checks the resulting focus; all reported idle samples passed the native focus-loss assertion.

For retention, three processes each for bundled/custom perform **60 add/remove cycles** of a temporary document view containing 200 lines, requesting full Lua GC after removal. Node layout/redraw and normal document cleanup still run. Per-cycle RSS is retained in the snapshot. Every run's cycle-1-to-cycle-60 change was **+4 KiB**, and the last 30 samples spanned 4 KiB. RSS was approximately 21.86–22.16 MiB across these runs. This is a bounded, **GC-conditioned** view/document-lifecycle observation, not proof of no leaks or a peak-memory benchmark. No CJK/font-format, huge-file peak or native-window churn test was performed.

## Cost of trying a change

### Lua: edit/save/restart, not automatic hot reload

The bundled [autorestart plugin](../../../data/plugins/autorestart.lua#L12-L18) wraps `Doc.save` for user init. The probe opens its own user-init file, adds a small caret-style change and saves through the ordinary document API. It observes the new Lua state through the first completed redraw. No Rust compile occurs. Three trials/configuration:

| Configuration | Save-path-start to restarted-frame observation |
| --- | --- |
| Bundled | **48.06, 39.70, 39.14 ms** |
| Custom | **43.52, 48.81, 41.65 ms** |

The interval includes opening/editing/saving the init document, normal restart, reinitialisation and marker transport; it excludes human editing time. It is not general in-place hot reload or plugin unload/reload. Restart normally changes document restoration state; the probe ensures its fixture view exists if necessary, and that work is included. There is no supported claim that an arbitrary external write of a Lua file automatically reloads it. The existing shallow `core.reload_module` mechanism was not timed or treated as an equivalent workflow.

### Rust: separate clean artifacts, no-op and a core source edit

GNU time observations, four Cargo jobs, warm dependency-source caches. These are small samples on an interactive host, not controlled compiler benchmarks. Debug/release use separate owned target directories; current Cargo profiles are unchanged. The core edits are small algebraically equivalent arithmetic rewrites in `system.rs::get_time`, in the isolated checkout only, followed by exact source restoration and rebuild. They represent a small core edit, not adding a substantial feature or dependency.

| Measurement | Debug | Release |
| --- | --- | --- |
| Empty artifact target; one trial | **29.77 s** | **60.38 s** |
| True normal-checkout no-op; three trials | **0.14, 0.18, 0.16 s** | **0.14, 0.18, 0.20 s** |
| Small core-edit rebuild; two trials | **1.57, 0.95 s** | **50.41, 47.59 s** |
| Restored-source rebuild; two trials | 1.49, 1.81 s | 27.69, 35.95 s |
| Owned target size after trials | **1.29 GiB** | **0.51 GiB** |

Cold-artifact builds peaked at GNU-time max RSS of approximately **798 MiB debug / 882 MiB release**. This reports the maximum individual process observed through the wrapped build tree, **not aggregate concurrent compiler memory**. Debug executable size was approximately 168 MiB including its debug information; that is not its runtime RSS. Build caches and the staged install are also distinct from the shipped binary's size.

**Linked-worktree trap, observed and corrected:** initial linked-worktree no-op attempts took 0.81–0.86 s debug and 25.97–26.20 s release, recompiling the package each time. The [build script](../../../build.rs#L5-L10) watches `.git/logs/HEAD`; `.git` is a file in that checkout, making the watched path absent. A normal isolated checkout with a real `.git` directory produced the true no-op results above. The bad no-op measurements are diagnostic only, not the main developer-loop baseline. Source restoration and executable hashes were checked before runtime use.

Release edit/build numbers varied substantially, as did the restore rebuilds. They show that the release compile/link path can be materially more expensive than the Lua restart or debug edit path here; they do **not** isolate thin LTO, codegen-units, link work or background contention as the cause. No build flags were tuned.

## Reproduction and checks

Helpers: [runtime runner](gui-cost-baseline/runtime.mjs), [disposable user init](gui-cost-baseline/probe.lua), [private-X11 focus/input helper](gui-cost-baseline/focus.c), [normal-checkout build follow-up](gui-cost-baseline/build-followup.mjs). They are evidence-only, not shipped editor/test infrastructure.

From a matching Linux development shell with cached dependencies, make an **owned** experiment directory, not the developer's target:

```sh
ROOT=$(mktemp -d "$PWD/target/gui-cost-baseline.XXXXXX")
git worktree add --detach "$ROOT/source" e556d388892280d8343a82a625941137e3a6ac5f
mkdir -p "$ROOT/logs"
# GNU time -v provides the cold-build evidence; each target must start empty.
(cd "$ROOT/source" && CARGO_TARGET_DIR="$ROOT/build/target-debug" \
  /run/current-system/sw/bin/time -v -o "$ROOT/logs/cold-debug.time" \
  timeout -k 5 480 cargo build --offline --locked -j 4)
(cd "$ROOT/source" && CARGO_TARGET_DIR="$ROOT/build/target-release" \
  /run/current-system/sw/bin/time -v -o "$ROOT/logs/cold-release.time" \
  timeout -k 5 480 cargo build --offline --locked -j 4 --release)
# Use a real .git directory for no-op/edit trials; only this owned copy is edited.
git clone --shared --no-checkout "$PWD" "$ROOT/normal-source"
git -C "$ROOT/normal-source" checkout --detach e556d388892280d8343a82a625941137e3a6ac5f
node docs/wayfinder/assets/gui-cost-baseline/build-followup.mjs "$ROOT"
LITE_RELEASE=1 scripts/install-linux.sh \
  "$ROOT/build/target-release/release/lite-xl" "$ROOT/stage" /usr/local
cc -O2 -Wall -Wextra -Werror docs/wayfinder/assets/gui-cost-baseline/focus.c \
  -o "$ROOT/focus" $(pkg-config --cflags --libs x11 xtst)
node docs/wayfinder/assets/gui-cost-baseline/runtime.mjs --self-test
. scripts/rust-port/private-display.sh
trap 'pd_profile_stop' EXIT
pd_profile_start xvfb "$ROOT/display"
pd_run env GUI_BENCH_DISPLAY="$pd_display" \
  node "$PWD/docs/wayfinder/assets/gui-cost-baseline/runtime.mjs" "$ROOT"
pd_run env GUI_BENCH_DISPLAY="$pd_display" \
  node "$PWD/docs/wayfinder/assets/gui-cost-baseline/runtime.mjs" "$ROOT" --edit-only
```

The observed experiment also retained a copy of its clean release executable before build-edit trials. Check source cleanliness and binary identity before using any measured executable. Do not automatically delete a developer cache, drop OS caches, expose a host display or treat a new empty target as an ordinary warm rebuild. Budget resource use before reproducing; the experiment used 480-second cold/300-second edit-build and 60-second per-runtime limits, and checked owned artifact size below 15 GiB between follow-up builds. That size check is not an instantaneous hard filesystem quota.

Validation performed: Node nearest-rank self-check; C compile with warnings as errors; smoke launch; runtime assertions including focus/sample counts and clean exit; successful bounded final collection; clean isolated sources; frozen production/test inputs. No existing contract suite, clippy or full test run was performed: this was measurement, not production implementation.

## Evidence still needed for acceptance criteria

- Controlled physical-desktop startup, earliest useful input-ready and input-to-visible latency; compositor/refresh-rate/normal+fractional-density effects, real IME/input and optional native Wayland. The available host desktop was deliberately not exercised, not proven unavailable.
- A defensible cold-cache startup procedure that does not damage the developer's working environment, and uninstrumented/paired observations that quantify observer and host noise. No p99 or stable regression tolerance is inferred from this sample count.
- The pinned external-plugin corpus and interactions from [Assemble the GUI and plugin compatibility reference evidence](../issues/45-assemble-gui-compatibility-reference.md). Synthetic/bundled configurations are not a substitute.
- Broader workloads: real projects, long/complex lines, background language tools, expensive plugins, sustained sessions, peak memory and retention without forced GC. Current data cannot bound arbitrary Lua work or guarantee a responsiveness budget.
- Replicated/paired Rust-build measurements with controlled host load and broader source/dependency edits if compiler cost becomes a design discriminator. Artifact-clean with cached sources is not a pristine-machine installation benchmark.

These are explicit missing observations, not waived requirements or passing gates. [Define GUI performance and customisation acceptance criteria](../issues/43-define-gui-acceptance-criteria.md) must decide usable criteria and the treatment of missing evidence; no previous map's numerical thresholds are inherited.
