#!/usr/bin/env bash
# Headless loop checks only: never connects to the user's clipboard or portal.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../../../.." && pwd)
if [[ ${PROBE_IN_SHELL:-} != 1 ]]; then
  exec nix develop "$root" --command env PROBE_IN_SHELL=1 bash "$here/check-loop.sh"
fi
export CARGO_TARGET_DIR="$here/target"
cargo test --locked --manifest-path "$here/Cargo.toml"
cargo build --locked --manifest-path "$here/Cargo.toml"
out=$(mktemp -d)
Xvfb -displayfd 3 -screen 0 1200x900x24 -nolisten tcp 3> "$out/display" > "$out/xvfb.log" 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do [[ -s "$out/display" ]] && break; sleep 0.1; done
read -r display < "$out/display"
for mode in decorated borderless; do
  flags=(); [[ $mode == borderless ]] && flags+=(--borderless)
  env -u WAYLAND_DISPLAY DISPLAY=:$display DBUS_SESSION_BUS_ADDRESS="unix:path=$out/no-session-bus" \
    timeout 12 bash "$here/run.sh" --backend x11 --self-check "${flags[@]}" > "$out/$mode.log" 2>&1
  grep -q 'SELF-CHECK PASS (no clipboard or real dialog calls)' "$out/$mode.log"
  printf '%s: ' "$mode"
  grep 'probe: loop summary' "$out/$mode.log"
done
if "$here/target/debug/sdl3-loop-probe" > "$out/missing-backend.log" 2>&1; then
  echo 'FAIL: missing backend accepted'; exit 1
fi
grep -q -- '--backend wayland|x11 is required' "$out/missing-backend.log"
echo "PASS: private-Xvfb checks only; local logs: $out"
