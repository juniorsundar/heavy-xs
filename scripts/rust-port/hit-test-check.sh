#!/usr/bin/env sh
# Diffs the Rust window hit-test (src/hit.rs) against the original C hit_test() over a
# coordinate grid. It needs no window or display: the C function (kept verbatim in
# fixtures/hit_test_reference.h, copied from the deleted src/api/system.c) is compiled with a
# stub SDL_GetWindowSize that returns the grid's size.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

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
cc -O1 -w -I"$tmp" $(pkg-config --cflags sdl3) "$tmp/c_main.c" -o "$tmp/c_grid"
"$tmp/c_grid" > "$tmp/c.txt"

rustc --edition 2021 --test "$root/src/hit.rs" -o "$tmp/r_test" 2>/dev/null
"$tmp/r_test" --nocapture --test-threads 1 2>/dev/null | sed 's/^test grid::print_grid ... //' | grep -E '^[0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+ [0-9-]+$' > "$tmp/r.txt"

lines=$(wc -l < "$tmp/c.txt")
if cmp -s "$tmp/c.txt" "$tmp/r.txt"; then echo "OK   hit-test ($lines grid points)"; else
  echo "FAIL hit-test"; diff "$tmp/c.txt" "$tmp/r.txt" | head -10; exit 1
fi
