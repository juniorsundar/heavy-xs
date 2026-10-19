#!/usr/bin/env bash
# Throwaway native window probe. Does not start or modify the editor.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
usage() {
  cat <<'EOF'
Usage: run.sh --backend wayland|x11 [--borderless] [--smoke-seconds N] [--wayland-padding zero|ff]

--help prints this text without building or launching the probe.

--wayland-padding zero|ff
    Wayland XRGB8888 high-byte padding. zero (default) follows softbuffer's
    documented 0RGB format. ff keeps the earlier diagnostic workaround and is
    rejected with --backend x11.

PROBE_NIX=1 builds and runs inside this repository's nix develop shell
(useful when the system X11/Wayland libraries are missing). Without it the
local rust toolchain and system libraries are used.
PROBE_FONT=/path/to/font overrides the optional fallback font.
EOF
}
if [[ $# -eq 0 ]]; then usage; exit 0; fi
for arg in "$@"; do
  case $arg in --help|-h) usage; exit 0;; esac
done
if [[ -n ${SSH_CONNECTION:-}${SSH_CLIENT:-}${SSH_TTY:-} ]]; then
  echo 'probe: SSH session; run locally at the desktop instead' >&2
  exit 2
fi
if [[ ${PROBE_NIX:-0} == 1 && ${PROBE_IN_SHELL:-0} != 1 ]]; then
  root=$(cd "$here/../../../.." && pwd)
  exec nix develop "$root" --command env PROBE_IN_SHELL=1 bash "$here/run.sh" "$@"
fi
if [[ ${PROBE_IN_SHELL:-0} == 1 ]]; then
  # The crates dlopen these system libraries; Nix does not put them in /usr/lib.
  for package in xkbcommon wayland-client wayland-cursor x11 xcursor xi xrandr xcb; do
    libdir=$(pkg-config --variable=libdir "$package")
    export LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  done
fi
if [[ -z ${PROBE_FONT:-} ]] && command -v fc-match >/dev/null; then
  PROBE_FONT=$(fc-match -f '%{file}' ':lang=zh')
  export PROBE_FONT
fi
export CARGO_TARGET_DIR="$here/target"
exec cargo run --locked --manifest-path "$here/Cargo.toml" -- "$@"
