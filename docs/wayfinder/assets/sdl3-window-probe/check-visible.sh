#!/usr/bin/env bash
# Local compositor regression check. Leave the desktop idle while it runs.
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
artifacts=${PROBE_CAPTURE_DIR:-$(mktemp -d)}
mkdir -p "$artifacts"
# Build before starting the six-second desktop observation.
nix develop "$here/../../../.." --command env CARGO_TARGET_DIR="$here/target" \
  cargo build --locked --manifest-path "$here/Cargo.toml" > "$artifacts/build.log" 2>&1
bash "$here/run.sh" --backend wayland --borderless --smoke-seconds 6 > "$artifacts/probe.log" 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do
  grep -q 'probe: focused=true' "$artifacts/probe.log" && break
  sleep 0.1
done
grep -q 'probe: focused=true' "$artifacts/probe.log" || { echo 'No probe focus; capture invalid'; exit 2; }
sleep 0.5
timeout 5 spectacle --activewindow --background --nonotify --no-decoration --no-shadow --output "$artifacts/window.png" > "$artifacts/capture.log" 2>&1
wait "$pid"
trap - EXIT
read -r expected scale < <(awk '/probe: presented frame/{size=$4; scale=$7} END{print size,scale}' "$artifacts/probe.log")
size=$(magick identify -format '%wx%h' "$artifacts/window.png")
colors=$(magick identify -format '%k' "$artifacts/window.png")
frames=$(grep -c 'probe: presented frame' "$artifacts/probe.log" || true)
printf 'capture=%s expected=%s colors=%s frames=%s\n' "$size" "$expected" "$colors" "$frames"
[[ "$size" == "$expected" ]] || { echo 'Invalid capture: dimensions do not match the probe'; exit 2; }
(( colors >= 10 )) || { echo 'FAIL: uniform/near-uniform window, expected grid and text'; exit 1; }
x=$(( ${size%x*} / 2 ))
y=$(awk -v s="$scale" 'BEGIN{print int(20*s)}')
magick "$artifacts/window.png" -crop "1x1+$x+$y" -alpha off -depth 8 txt:- | grep -qi '#244C3A' || { echo 'FAIL: expected green drag strip is missing'; exit 1; }
(( frames <= 12 )) || { echo 'FAIL: idle redraw storm'; exit 1; }
echo "PASS: visible probe pixels and bounded idle redraws. Local artifacts: $artifacts"
