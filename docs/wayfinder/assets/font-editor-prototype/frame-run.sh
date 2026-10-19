#!/usr/bin/env sh
# PROTOTYPE: paired, alternating real-window frame timing. FreeType and candidate runs alternate
# (A B B A ...) on one host with identical inputs. Opens maximized editor windows; do not use
# the machine while it runs.
# Usage: frame-run.sh <stage prefix> <outdir> <config: user|bundled> [rounds, default 3]
set -eu
prefix=$1; out=$2; config=$3; rounds=${4:-3}
bin=$prefix/bin/lite-xl
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"
work=$(mktemp -d /tmp/lxl-frame-run.XXXXXX)
trap 'rm -rf "$work"' EXIT INT TERM
# Plain-text copy of a real source file: identical text, and no language server attaches.
cp "$here/../../../../src/renderer.rs" "$work/sample.txt"
userdir=$work/userdir
if [ "$config" = user ]; then
  cp -r "${XDG_CONFIG_HOME:-$HOME/.config}/lite-xl" "$userdir"
  rm -f "$userdir/session.lua" "$userdir/error.txt"
else
  mkdir -p "$userdir"
  printf 'local style = require "core.style"\nstyle.font = renderer.font.load(DATADIR .. "/fonts/FiraSans-Regular.ttf", 15 * SCALE)\nstyle.code_font = renderer.font.load(DATADIR .. "/fonts/JetBrainsMono-Regular.ttf", 15 * SCALE)\n' > "$userdir/init.lua"
fi
cat "$here/frame-probe.lua" >> "$userdir/init.lua"
i=0
while [ $i -lt "$rounds" ]; do
  # Alternate the order each round so drift does not favour one backend.
  if [ $((i % 2)) -eq 0 ]; then order="freetype candidate"; else order="candidate freetype"; fi
  for b in $order; do
    rm -f "$userdir/session.lua"
    FRAME_OUT="$out/$config-$b-r$i.tsv" FRAME_FILE="$work/sample.txt" LITE_FONT_BACKEND=$b \
      LITE_USERDIR="$userdir" LITE_PREFIX="$prefix" timeout -k 2 180 "$bin" >/dev/null 2>&1 \
      || echo "run $config $b r$i exited $?"
  done
  i=$((i + 1))
done
echo "done: $out"
