#!/usr/bin/env bash
# Throwaway loop/services probe. Clipboard and portals require explicit user actions.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../../../.." && pwd)
source "$here/env.sh"
[[ ${PROBE_IN_SHELL:-} == 1 ]] || probe_enter_shell "$here/run.sh" "$@"
probe_library_path
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
