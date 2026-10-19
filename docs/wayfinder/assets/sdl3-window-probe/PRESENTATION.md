# First portable Wayland presentation diagnostic (prototype)

**Status: throwaway diagnostic pending user observation. No production stack, fix or culprit is claimed.**

## Question and scope

Question: on a Wayland desktop, does the earlier black/blank window reproduce with
softbuffer 0.4.8 + winit 0.30.13 while the high byte of every XRGB8888 pixel is zero,
and does forcing that byte to `0xff` change what the user sees?

In scope: one throwaway probe, one explicit `--wayland-padding zero|ff` toggle, and a
comparison of two matched live observations. Out of scope: editor integration, IME or
scale re-qualification, transparency, screen capture, compositor/driver attribution,
and any product decision or supported remedy.

`zero` is the default and restores the documented 0RGB reproduction. `ff` keeps the
earlier Wayland-only opaque-padding workaround. The drawn scene is identical in both
modes; only the window title names the mode. `ff` is rejected with `--backend x11`
rather than silently ignored.

## Originals preserved

The original probe, README and evidence are preserved in the parent commit `f8dacae0`
(`prototype/sdl3-window-probe` content). That README is historical evidence and is not
rewritten. This file supersedes only the launcher defaults and the live procedure: the
launcher now uses the local toolchain by default, Nix is opt-in, and the diagnostic wrapper requires `--run`. The lower-level `run.sh` also launches
when supplied backend arguments; use only its help path during offline preparation.

## Rebuild offline

```sh
git clone https://github.com/juniorsundar/heavy-xs.git
cd heavy-xs
git switch prototype/wayland-presentation
probe=docs/wayfinder/assets/sdl3-window-probe
cargo build --locked --offline --manifest-path "$probe/Cargo.toml"
cargo test --locked --offline --manifest-path "$probe/Cargo.toml"
cargo fmt --manifest-path "$probe/Cargo.toml" --check
```

The branch must be available on the clone's remote; if the checkout fails, the
prototype branch has not been published yet. `--offline` works once the locked
dependencies are in the Cargo cache (`cargo fetch --locked` once while online, or
enter `nix develop` for tools/system libraries; Cargo still needs its own dependency cache).
From the repository root, `bash -n "$probe/run.sh" "$probe/presentation-check.sh"`
checks the launchers without launching anything.
The help paths build nothing:

```sh
bash "$probe/run.sh" --help
bash "$probe/presentation-check.sh" --help
```

## Linux system requirements

- A Wayland desktop session for live runs; this diagnostic refuses X11-only or
  missing-session environments instead of faking one. It also refuses SSH sessions
  (checks `SSH_CONNECTION`, `SSH_CLIENT`, `SSH_TTY`), even with inherited Wayland variables.
- Rust and Cargo compatible with `Cargo.lock`, plus a C linker/toolchain for build
  dependencies.
- Runtime libraries opened by the crates: `libwayland-client`, `libwayland-cursor`,
  `libxkbcommon`, and a Wayland compositor. For a forced X11 comparison: `libX11`,
  `libXcursor`, `libXi`, `libXrandr`, `libxcb`.
- Optional: fontconfig (`fc-match`) for a CJK fallback face. `PROBE_FONT` overrides it.
  Package names and libdirs differ by distribution; install the development packages
  when pkg-config/linking fails.
- Opt-in Nix shell (uses this repository's `flake.nix`): prefix a command with
  `PROBE_NIX=1`. Only that path exports Nix pkg-config libdirs. Without it, the local
  toolchain and system library paths are used; no host path is hardcoded.

## Live commands - only after explicit user permission

Run these from the repository root, locally at the Wayland desktop, not through SSH.
Nothing below starts a GUI without `--run`. No capture is taken. Logs may contain
keyboard text and Wayland protocol contents, so use throwaway test text and keep the
output directory private. Leave the desktop idle.

```sh
# zero (default, documented 0RGB reproduction)
bash docs/wayfinder/assets/sdl3-window-probe/presentation-check.sh --run zero /tmp/lxl-present-zero

# ff (Wayland-only diagnostic high byte)
bash docs/wayfinder/assets/sdl3-window-probe/presentation-check.sh --run ff /tmp/lxl-present-ff

# Capture the protocol (no screen capture); compare set_opaque_region, create_buffer,
# attach and commit ordering. The trace may contain private typed text.
WAYLAND_DEBUG=client bash docs/wayfinder/assets/sdl3-window-probe/presentation-check.sh --run zero /tmp/lxl-present-zero-trace

# Same cases through the opt-in Nix shell
PROBE_NIX=1 bash docs/wayfinder/assets/sdl3-window-probe/presentation-check.sh --run zero /tmp/lxl-present-zero-nix
PROBE_NIX=1 bash docs/wayfinder/assets/sdl3-window-probe/presentation-check.sh --run ff /tmp/lxl-present-ff-nix
```

Each case runs for eight seconds (`--smoke-seconds 8`) and writes a new `OUTDIR`;
existing directories are refused. `metadata.txt` records the commit, branch, kernel,
toolchain, session variables and mode, without serial numbers. `live.log` records the
probe output. If `WAYLAND_DEBUG=client` is set, stderr goes to the separate
`wayland-debug.log`. Exit 0 means at least one frame was presented; it is not proof of
visibility or correctness.

## Matched-condition visual checklist

Compare only matched conditions: same machine, compositor, Wayland session, output,
scale factor, decoration mode and IME state. Do not change scale or environment
variables between cases; restart the probe for each mode. Observe the window yourself; this first diagnostic takes no screenshots.
Any later capture check needs separate permission and must identify the correct probe window.

For each mode record pass / fail / pending:

1. Window title shows `wayland-padding=zero` or `wayland-padding=ff` (mode identity only).
2. Grid, two-tone background, blue/purple border, green top strip and the four text
   lines plus the text field are visible.
3. The window is not uniform black/transparent, and there are no stale, partial or
   flickering areas.
4. `live.log` contains `probe: presented frame` and the exit status is 0.

Record the two runs separately; do not infer one mode from the other. Add the actual
scale factor and output name, compositor/version, GPU/driver/version, and whether the
GPU or desktop differs from the original reproducing machine. These are manually
recorded context; the wrapper does not detect them or change them.

The protocol trace is the next diagnostic, not a remedy: inspect whether winit's
`set_opaque_region` was committed before the first non-null `attach`, and whether
`create_buffer` advertises XRGB8888. Winit's default non-transparent window already
requests an opaque region; a missing hint alone cannot explain a blank XRGB image
under the protocol's intended semantics. Do not infer an actual wire sequence from
source code alone.

## Output interpretation

- `presented frame` plus exit 0: softbuffer returned success for a buffer commit. It does
  not prove that the compositor displayed it.
- `zero` blank and `ff` visible: the historical observation reproduces; the high-byte
  difference remains a live hypothesis, not proof of a compositor, driver or softbuffer
  defect.
- Both visible: the earlier blank window did not reproduce under these conditions;
  this prototype cannot identify why.
- Both blank: zero padding is not sufficient to explain the symptom.
- `zero` visible and `ff` blank: weakens the high-byte hypothesis.
- Nonzero exit: startup or presentation failed or the eight-second smoke timeout saw no
  present. Read the log; this is not a visual result.
- Refusal for SSH, a non-Wayland session or unset `WAYLAND_DISPLAY`: the case did not run.

No supported remedy is claimed. Softbuffer documents a zero high byte; `ff` is an
observed workaround outside that contract and remains diagnostic only.

## Offline validation of this revision

- `cargo test --locked --offline`: **9 passed**, including default/invalid padding
  parsing, pre-event-loop rejection of X11 + `ff`, and RGB preservation in both modes.
- `cargo build --locked --offline`, `cargo fmt --check`, `bash -n` and
  `git diff --check`: passed.
- Both launchers' help/no-args paths do not build; both refuse SSH launch attempts.
  These guards were checked with a sentinel `cargo` executable so no real launcher
  could run even if a guard regressed. Invalid mode is also refused without output.
- Independent review found no remaining blockers.
- No completed live run or capture of this revision. During review, an inadvertent
  launch-path attempt failed to load Wayland before creating an event loop/window.
  Explicit SSH refusals were added afterward. Desktop visibility, actual protocol
  order and the Nix live path remain **unverified**; this does not resolve the ticket.

## Pinned references

- softbuffer 0.4.8 pixel data representation (high byte zero):
  <https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Buffer.html> and
  <https://docs.rs/softbuffer/0.4.8/src/softbuffer/lib.rs.html>
- softbuffer 0.4.8 Wayland backend (`wl_shm::Format::Xrgb8888`):
  <https://docs.rs/softbuffer/0.4.8/src/softbuffer/backends/wayland/buffer.rs.html>
- winit 0.30.13 transparency is a hint, including `with_transparent`:
  <https://docs.rs/winit/0.30.13/winit/window/struct.WindowAttributes.html#method.with_transparent>
  and <https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.set_transparent>
- winit 0.30.13 window source:
  <https://docs.rs/winit/0.30.13/src/winit/window.rs.html>
