#!/usr/bin/env bash
# Throwaway loop/services probe. Clipboard and portals require explicit user actions.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../../../.." && pwd)
if [[ ${PROBE_IN_SHELL:-} != 1 ]]; then
  exec nix develop "$root" --command env PROBE_IN_SHELL=1 bash "$here/run.sh" "$@"
fi
# The crates dlopen these system libraries; Nix does not put them in /usr/lib.
for package in xkbcommon wayland-client wayland-cursor x11 xcursor xi xrandr xcb; do
  libdir=$(pkg-config --variable=libdir "$package")
  export LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
done
if [[ -z ${PROBE_FONT:-} ]] && command -v fc-match >/dev/null; then
  PROBE_FONT=$(fc-match -f '%{file}' ':lang=zh')
  export PROBE_FONT
fi
backend=
args=("$@")
for ((i=0; i<${#args[@]}; i++)); do
  if [[ ${args[i]} == --backend ]]; then backend=${args[i+1]:-}; fi
done
# arboard selects its own transport. Prevent silent cross-backend fallback in this child.
case "$backend" in
  wayland) unset DISPLAY ;;
  x11) unset WAYLAND_DISPLAY ;;
esac
export CARGO_TARGET_DIR="$here/target"
exec cargo run --locked --manifest-path "$here/Cargo.toml" -- "$@"
