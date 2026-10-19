#!/usr/bin/env sh
# PROTOTYPE live review: open the same file in two editor windows, FreeType and candidate.
# Usage: review.sh <staged prefix, e.g. $R/stage/usr/local> [file] [config]
#   config: user     -> your ~/.config/lite-xl (default)
#           bundled  -> a throwaway user dir with the stock fonts at the defaults (subpixel/slight)
#           mode=AA/HINT -> stock fonts forced to one mode, e.g. mode=none/full
# Each window title starts with [freetype] or [candidate]. Both windows share nothing but the file on disk.
set -eu
prefix=$1
file=${2:-$prefix/share/lite-xl/core/init.lua}
config=${3:-user}
bin=$prefix/bin/lite-xl

case $config in
  user) userdir=${XDG_CONFIG_HOME:-$HOME/.config}/lite-xl ;;
  *)
    userdir=$(mktemp -d /tmp/lxl-font-review-userdir.XXXXXX)
    opts=""
    case $config in
      mode=*) aa=${config#mode=}; hint=${aa#*/}; aa=${aa%/*}
              opts=", {antialiasing=\"$aa\", hinting=\"$hint\"}" ;;
    esac
    cat > "$userdir/init.lua" <<EOF
local style = require "core.style"
style.font = renderer.font.load(DATADIR .. "/fonts/FiraSans-Regular.ttf", 15 * SCALE$opts)
style.code_font = renderer.font.load(DATADIR .. "/fonts/JetBrainsMono-Regular.ttf", 15 * SCALE$opts)
EOF
    ;;
esac

for backend in freetype candidate; do
  LITE_FONT_BACKEND=$backend LITE_USERDIR=$userdir LITE_PREFIX=$prefix \
    "$bin" "$file" >/dev/null 2>&1 &
done
echo "Opened two windows ($config); titles start with [freetype] / [candidate]. Place them side by side."
wait
