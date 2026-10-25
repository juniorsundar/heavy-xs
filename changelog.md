# Heavy XS changelog

This file summarizes development since the Lite XL fork. The inherited release
history belongs to Lite XL; see [project provenance](docs/provenance.md) for the
exact baseline and its original changelog.

## Unreleased

- Replaced the C core and Meson build with a standalone Rust core built by Cargo.
  The editor layer remains Lua, with vendored Lua 5.4.
- Made Linux the supported platform and added Nix packages and development shells.
- Replaced PCRE2 with fancy-regex. See the
  [regex migration notes](docs/regex.md) for syntax and replacement differences.
- Replaced FreeType with Skrifa, Zeno and tiny-skia. See
  [font behaviour and limitations](docs/fonts.md).
- Replaced SDL with winit windows and input, softbuffer presentation, arboard
  clipboard access and ashpd file dialogs. X11 is the default; native Wayland is
  opt-in.
- Added Rust tests, isolated contract suites and pinned historical reference
  tooling to check behaviour across the port.
- Fixed inherited fuzzy-match and quit-step bugs, input state after focus loss,
  render-cache tab sizing and directory-watch recovery after inotify overflow.

The Git history retains the individual development commits after the imported
upstream baseline.
