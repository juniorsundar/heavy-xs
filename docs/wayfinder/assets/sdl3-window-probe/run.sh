#!/usr/bin/env bash
# Throwaway native window probe. Does not start or modify the editor.
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
export CARGO_TARGET_DIR="$here/target"
exec cargo run --locked --manifest-path "$here/Cargo.toml" -- "$@"
