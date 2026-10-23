#!/usr/bin/env sh
# Checks the Rust window hit-test (src/hit.rs) over a coordinate grid against the classification
# the original C hit_test() produced, recorded as sdl-expectations/hit-test-grid.txt (line count
# and sha256 of the C grid). That check needs no SDL, so it outlives SDL.
#   HIT_TEST_REFERENCE=c   also rebuild the C grid (fixtures/hit_test_reference.h, copied from the
#                          deleted src/api/system.c, with a stub SDL_GetWindowSize; needs the SDL3
#                          headers from `nix develop .#reference`) and check it against both the recording and the Rust grid
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
expect=$root/scripts/rust-port/sdl-expectations/hit-test-grid.txt
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

grid_id() { echo "lines $(wc -l <"$1" | tr -d ' ') sha256 $(sha256sum "$1" | cut -d' ' -f1)"; }

rustc --edition 2021 --test "$root/src/hit.rs" -o "$tmp/r_test" 2>/dev/null
"$tmp/r_test" --nocapture --test-threads 1 2>/dev/null | sed 's/^test grid::print_grid ... //' | grep -E '^[0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+$' > "$tmp/r.txt"
recorded=$(cat "$expect")
fail=0
if [ "$(grid_id "$tmp/r.txt")" = "$recorded" ]; then echo "OK   hit-test vs recorded C grid ($recorded)"; else
  echo "FAIL hit-test: Rust grid is $(grid_id "$tmp/r.txt"), recorded C grid is $recorded"; fail=1
fi

if [ "${HIT_TEST_REFERENCE:-}" = c ]; then
  cp "$root/scripts/rust-port/fixtures/hit_test_reference.h" "$tmp/c_body.h"
  cat > "$tmp/c_main.c" <<'C'
#include <stdio.h>
#include <SDL3/SDL.h>
static int G_W, G_H;
#define SDL_GetWindowSize(win, pw, ph) (*(pw) = G_W, *(ph) = G_H, true)
#include "c_body.h"
int main(void) {
  static const int sizes[][2] = {{400,300},{50,40},{3,3},{1,1},{1000,20}};
  static const int infos[][3] = {{0,0,0},{30,90,6},{30,0,6},{1000,5,5},{10,10,1},{30,90,40},{-5,-5,-5},{0,0,100}};
  for (int s = 0; s < 5; s++) for (int i = 0; i < 8; i++) {
    G_W = sizes[s][0]; G_H = sizes[s][1];
    HitTestInfo info = { infos[i][0], infos[i][1], infos[i][2] };
    for (int y = -3; y < G_H + 4; y++) for (int x = -3; x < G_W + 4; x++) {
      SDL_Point pt = { x, y };
      printf("%d %d %d %d %d %d %d %d\n", G_W, G_H, info.title_height, info.controls_width,
             info.resize_border, x, y, (int) hit_test(NULL, &pt, &info));
    }
  }
  return 0;
}
C
  # shellcheck disable=SC2046
  cc -O1 -w -I"$tmp" $(pkg-config --cflags sdl3) "$tmp/c_main.c" -o "$tmp/c_grid"
  "$tmp/c_grid" > "$tmp/c.txt"
  if [ "$(grid_id "$tmp/c.txt")" = "$recorded" ]; then echo "OK   C reference grid matches the recording"; else
    echo "FAIL C reference grid is $(grid_id "$tmp/c.txt"), recorded $recorded"; fail=1
  fi
  if cmp -s "$tmp/c.txt" "$tmp/r.txt"; then echo "OK   hit-test vs C reference ($(wc -l < "$tmp/c.txt" | tr -d ' ') grid points)"; else
    echo "FAIL hit-test vs C reference"; diff "$tmp/c.txt" "$tmp/r.txt" | head -10; fail=1
  fi
fi
exit $fail
