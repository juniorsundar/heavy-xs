# Wayland CPU presentation — upstream triage

Research date: 2026-10-20. Supports **Qualify Wayland CPU presentation without
unsupported pixel padding**. No builds, dependency changes, desktop actions,
upstream posts or production changes were made during this research.

## Established reproduction

The pinned `winit 0.30.13` / `softbuffer 0.4.8` probe on KWin 6.8.0, DP-8 at
scale 1.20 produces blank content with zero XRGB high bytes. The unsupported
`0xff` control is visible with the same RGB content and XRGB8888 wire format.
Clearing the opaque-region hint did not restore zero-padding content; neither
did decorations. Physical-display observations agree with the guarded captures.
See the qualification ticket for evidence, sizes, limitations and user observations.
These facts do not identify a faulty component or prove a scaling-only cause.

## Primary-source findings

1. **XRGB and ARGB are distinct formats.** The Wayland specification describes
   XRGB8888 as `x:R:G:B` and ARGB8888 as `A:R:G:B`, with format detail deferred
   to `drm_fourcc.h`. The Linux header defines the same layouts. Wayland's
   default for formats carrying alpha is premultiplied alpha. An ARGB buffer
   with alpha `0xff` is therefore a distinct format experiment, not permission
   to write `0xff` into softbuffer's XRGB padding. None of this establishes
   what caused the observed failure.
   Sources: [Wayland protocol, wl_shm](https://gitlab.freedesktop.org/wayland/wayland/-/raw/main/protocol/wayland.xml),
   [Linux drm_fourcc.h](https://raw.githubusercontent.com/torvalds/linux/master/include/uapi/drm/drm_fourcc.h).
2. **The opaque region is an optimization hint, not a correctness requirement.**
   The specification says it is “not required for correct behaviour.” A full
   region declares opaque content; omitting it is not inherently incorrect.
   The probe traces already verify both the set and cleared cases. Their blank
   content does not prove a particular blending or GPU-upload mechanism.
   Source: [Wayland protocol, wl_surface.set_opaque_region](https://gitlab.freedesktop.org/wayland/wayland/-/raw/main/protocol/wayland.xml).
3. **The latest release identified in this pass is softbuffer 0.4.8.** Both
   GitHub's releases page and docs.rs identify it as latest. Its Wayland buffer
   creation and resize paths hardcode `wl_shm::Format::Xrgb8888`. No supported
   format-selection API was found in the checked 0.4.8 path. The corresponding
   alpha/format API appears under “Unreleased” in the changelog. softbuffer's
   present path does not set an opaque region; winit supplies it in this probe.
   Sources: [releases](https://github.com/rust-windowing/softbuffer/releases),
   [docs.rs latest](https://docs.rs/crate/softbuffer/latest),
   [0.4.8 buffer.rs](https://raw.githubusercontent.com/rust-windowing/softbuffer/v0.4.8/src/backends/wayland/buffer.rs),
   [0.4.8 mod.rs](https://raw.githubusercontent.com/rust-windowing/softbuffer/v0.4.8/src/backends/wayland/mod.rs),
   [changelog](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/CHANGELOG.md).
4. **Unreleased upstream offers a different wire format.** In master,
   `Surface::configure(width, height, alpha_mode)` selects XRGB8888 for
   `Opaque`/`Ignored` and ARGB8888 for `Premultiplied`; `next_buffer()` then
   acquires the configured buffer. The upstream alpha-mode work is merged, but
   this is not the checked release API. It changes APIs including buffer
   acquisition/configuration and pixel representation. A probe would need an
   explicit commit pin and review of the actual dependency/API delta; the
   current probe must remain preserved. Success is not predicted or verified.
   Sources: [master Wayland backend](https://raw.githubusercontent.com/rust-windowing/softbuffer/master/src/backends/wayland/mod.rs),
   [upstream alpha-mode change](https://github.com/rust-windowing/softbuffer/pull/321).

## Next decision, not an adopted remedy

- **Pinned upstream experiment:** with explicit user approval, prepare a separate
  throwaway probe using upstream's ARGB8888/premultiplied route and opaque alpha.
  Preserve the RGB scene and original zero-padding reproduction. A git pin is
  neither a fork nor a release, and must not silently become a production choice.
  Any later desktop run still needs bounded permission and live review. A positive
  result would demonstrate a route on that stack, not identify the original culprit.
- **Released-crates-only investigation:** retain the current pin and localize the
  failure. A reference-compositor run could narrow it, but must record actual
  backend, GPU path, output scale and extent. Nested Weston changes several parts
  of the path and may still depend on KWin; neither result alone proves “client
  wrong” or “KWin wrong.” Check availability first; installs and launches need approval.
- **Upstream report:** prepare a local, reviewed draft from the existing wire
  facts. The user must approve the destination and public contents before posting;
  do not upload private images, full traces or environment details by default.

Qualification remains unresolved. No production dependency adoption, fork,
handwritten client, SDL fallback, feature loss or acceptance-gate waiver is
authorized by this brief.

### User-authorized experiment

After the purpose and unreleased-dependency limitation were explained, the user
said **“do the test then”**. The selected upstream commit is
`9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`. Source inspection confirms that the
XRGB/zero arm must use **`AlphaMode::Ignored`**, not `Opaque`: upstream debug builds
require alpha `0xff` in `Opaque` mode. The ARGB arm uses `Premultiplied` and alpha
`0xff`. Both arms use the same upstream pin. The released baseline stays intact;
this authorizes only the isolated comparison and bounded guarded desktop runs,
not adopting the dependency in the editor.
Source: [pinned API and alpha requirements](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/lib.rs),
[pinned Wayland format mapping](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/backends/wayland/mod.rs).

Upstream buffers may have padded row strides. The experiment must preserve the
existing packed RGB drawing through row-aware conversion to typed pixels; a
packed whole-buffer copy would skew rows at widths not divisible by four.

## Search and retrieval limits

- No narrowly matching KWin/NVIDIA/Mesa/softbuffer issue or fixing commit was
  identified in this bounded search. Search results were weak; this is not evidence
  that no report or fix exists. KWin source-path tracing and NVIDIA/Mesa release-note
  checking remain undone. The actual render-GPU path is not established.
- The [Plasma 6.7.5→6.8.0 changelog](https://kde.org/announcements/changelogs/plasma/6/6.7.5-6.8.0/)
  contains subsurface opaque-region and recording-quality changes, but neither was
  established as a match. Screencast behavior alone cannot identify the display fault.
- crates.io HTML was blocked (403); release identification relies on GitHub and
  docs.rs. Other historic releases were not exhaustively diffed. Moving master URLs
  document this research, not a reproducible dependency pin.
- The Wayland fetch preview was truncated; the researcher inspected the relevant
  sections from the complete downloaded artifact. GitHub issue search extraction
  was limited to its first page. No proposed ARGB or reference-compositor run occurred.
