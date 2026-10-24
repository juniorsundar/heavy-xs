/* Observes the SDL behaviour the editor relies on, so the expectations exist independently of
 * SDL before SDL is removed (see docs/architecture/wayfinder/assets/sdl3-migration-handoff.md, Step 1).
 * Built and run by scripts/rust-port/sdl-reference.sh; never part of the editor.
 *   version   linked runtime version and revision (bindings do not prove it)
 *   draw      ren_draw_rect's SDL operations: opaque fill and the 1x1 RGBA32 linear scaled
 *             blend, both clipped, on a surface in the window surface's own pixel format
 *   clicks    SDL's multi-click count for real X button events (XTest) on the current display;
 *             only meaningful on the runner's private Xvfb */
#include <stdio.h>
#include <string.h>
#include <SDL3/SDL.h>
#ifdef PROBE_CLICKS
#include <X11/Xlib.h>
#include <X11/extensions/XTest.h>
#endif

/* The hints video_init sets (src/renderer.rs); clickthrough changes the first click's count. */
static void editor_hints(void) {
  SDL_SetHint(SDL_HINT_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR, "0");
  SDL_SetHint(SDL_HINT_MOUSE_FOCUS_CLICKTHROUGH, "1");
  SDL_SetHint(SDL_HINT_IME_IMPLEMENTED_UI, "1");
  SDL_SetHint("SDL_BORDERLESS_WINDOWED_STYLE", "1");
  SDL_SetHint("SDL_BORDERLESS_RESIZABLE_STYLE", "1");
}

static SDL_Window *editor_window(int w, int h) {
  return SDL_CreateWindow("probe", w, h,
                          SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY | SDL_WINDOW_HIDDEN);
}

static int version(void) {
  int v = SDL_GetVersion();
  printf("sdl_runtime %d.%d.%d revision %s\n", SDL_VERSIONNUM_MAJOR(v), SDL_VERSIONNUM_MINOR(v),
         SDL_VERSIONNUM_MICRO(v), SDL_GetRevision());
  return 0;
}

#define SW 12
#define SH 9

static void dump(const char *label, SDL_Surface *s) {
  const SDL_PixelFormatDetails *d = SDL_GetPixelFormatDetails(s->format);
  printf("%s\n", label);
  for (int y = 0; y < SH; y++) {
    printf(" ");
    for (int x = 0; x < SW; x++) {
      Uint32 p = *(Uint32 *)((Uint8 *)s->pixels + y * s->pitch + x * 4);
      Uint8 r, g, b;
      SDL_GetRGB(p, d, NULL, &r, &g, &b);
      printf(" %02x%02x%02x", r, g, b);
    }
    printf("\n");
  }
}

/* ren_draw_rect, minus the scale (always 1 here): fill for alpha 255, else manual clip plus a
 * linear scaled blit of one RGBA32 pixel. */
static void draw_rect(SDL_Surface *dst, SDL_Surface *one, SDL_Rect r, SDL_Color c) {
  if (c.a == 0) return;
  if (c.a == 0xff) {
    SDL_FillSurfaceRect(dst, &r, SDL_MapSurfaceRGB(dst, c.r, c.g, c.b));
    return;
  }
  SDL_Rect clip, clipped;
  SDL_GetSurfaceClipRect(dst, &clip);
  if (!SDL_GetRectIntersection(&clip, &r, &clipped)) return;
  *(Uint32 *)one->pixels = SDL_MapSurfaceRGBA(one, c.r, c.g, c.b, c.a);
  SDL_BlitSurfaceScaled(one, NULL, dst, &clipped, SDL_SCALEMODE_LINEAR);
}

static int draw(void) {
  editor_hints();
  if (!SDL_InitSubSystem(SDL_INIT_VIDEO)) { printf("init failed: %s\n", SDL_GetError()); return 1; }
  SDL_Window *win = editor_window(64, 48);
  SDL_Surface *ws = win ? SDL_GetWindowSurface(win) : NULL;
  if (!ws) { printf("window surface failed: %s\n", SDL_GetError()); return 1; }
  SDL_PixelFormat fmt = ws->format;
  printf("driver %s window_format %s\n", SDL_GetCurrentVideoDriver(), SDL_GetPixelFormatName(fmt));
  SDL_Surface *dst = SDL_CreateSurface(SW, SH, fmt);
  SDL_Surface *one = SDL_CreateSurface(1, 1, SDL_PIXELFORMAT_RGBA32);
  if (!dst || !one) { printf("surface failed: %s\n", SDL_GetError()); return 1; }

  static const SDL_Color bases[] = {{0, 0, 0, 255}, {255, 255, 255, 255}, {12, 200, 90, 255}};
  static const SDL_Color colors[] = {{255, 0, 0, 0}, {10, 20, 30, 0}, {255, 255, 255, 0}, {77, 140, 233, 0}};
  static const Uint8 alphas[] = {1, 2, 3, 64, 127, 128, 129, 191, 253, 254};
  static const SDL_Rect rects[] = {
    {2, 1, 1, 1}, {2, 1, 3, 2}, {1, 2, 7, 5}, {0, 0, SW, SH},
    {-3, -2, 6, 5}, {9, 6, 8, 8}, {4, 3, 0, 3}, {4, 3, 3, -1}, {SW, 0, 2, 2}, {-5, 0, 5, 3},
  };
  static const SDL_Rect clips[] = {{0, 0, SW, SH}, {3, 2, 5, 4}, {0, 0, 0, 0}};
  char label[160];

  for (size_t b = 0; b < SDL_arraysize(bases); b++)
    for (size_t c = 0; c < SDL_arraysize(colors); c++)
      for (size_t a = 0; a < SDL_arraysize(alphas); a++)
        for (int passes = 1; passes <= 2; passes++) {
          SDL_SetSurfaceClipRect(dst, NULL);
          SDL_FillSurfaceRect(dst, NULL, SDL_MapSurfaceRGB(dst, bases[b].r, bases[b].g, bases[b].b));
          SDL_Color col = colors[c];
          col.a = alphas[a];
          SDL_Rect r = {2, 1, 3, 2};
          for (int p = 0; p < passes; p++) draw_rect(dst, one, r, col);
          Uint32 px = *(Uint32 *)((Uint8 *)dst->pixels + 1 * dst->pitch + 2 * 4);
          Uint8 rr, gg, bb;
          SDL_GetRGB(px, SDL_GetPixelFormatDetails(fmt), NULL, &rr, &gg, &bb);
          printf("blend base=%u,%u,%u color=%u,%u,%u,%u passes=%d -> %u,%u,%u raw=%08x\n",
                 bases[b].r, bases[b].g, bases[b].b, col.r, col.g, col.b, col.a, passes, rr, gg, bb, px);
        }

  /* Geometry: which pixels each rect touches, opaque and blended, under each clip. */
  for (size_t k = 0; k < SDL_arraysize(clips); k++)
    for (size_t i = 0; i < SDL_arraysize(rects); i++)
      for (int blended = 0; blended <= 1; blended++) {
        SDL_SetSurfaceClipRect(dst, NULL);
        SDL_FillSurfaceRect(dst, NULL, SDL_MapSurfaceRGB(dst, 0, 0, 0));
        SDL_SetSurfaceClipRect(dst, &clips[k]);
        SDL_Color col = {200, 100, 50, blended ? 128 : 255};
        draw_rect(dst, one, rects[i], col);
        snprintf(label, sizeof label, "rect clip=%d,%d,%d,%d rect=%d,%d,%d,%d alpha=%u", clips[k].x,
                 clips[k].y, clips[k].w, clips[k].h, rects[i].x, rects[i].y, rects[i].w, rects[i].h, col.a);
        dump(label, dst);
      }

  /* Channel packing of the formats in play. */
  SDL_PixelFormat fmts[] = {fmt, SDL_PIXELFORMAT_RGBA32};
  for (size_t f = 0; f < 2; f++) {
    const SDL_PixelFormatDetails *d = SDL_GetPixelFormatDetails(fmts[f]);
    printf("format %s bpp=%u Rmask=%08x Gmask=%08x Bmask=%08x Amask=%08x map(1,2,3,4)=%08x\n",
           SDL_GetPixelFormatName(fmts[f]), d->bits_per_pixel, d->Rmask, d->Gmask, d->Bmask, d->Amask,
           SDL_MapRGBA(d, NULL, 1, 2, 3, 4));
  }
  SDL_DestroySurface(one);
  SDL_DestroySurface(dst);
  SDL_DestroyWindow(win);
  SDL_Quit();
  return 0;
}

#ifdef PROBE_CLICKS
static Display *xdpy;
static int wx, wy;

static void pump_report(const char *step, Uint32 settle_ms) {
  Uint64 until = SDL_GetTicks() + settle_ms;
  SDL_Event e;
  while (SDL_GetTicks() < until) {
    while (SDL_PollEvent(&e)) {
      if (e.type == SDL_EVENT_MOUSE_BUTTON_DOWN || e.type == SDL_EVENT_MOUSE_BUTTON_UP)
        printf("%s %s button=%u clicks=%u x=%.0f y=%.0f\n", step,
               e.type == SDL_EVENT_MOUSE_BUTTON_DOWN ? "down" : "up", e.button.button, e.button.clicks,
               e.button.x, e.button.y);
    }
    SDL_Delay(2);
  }
}

static void move(int x, int y) { XTestFakeMotionEvent(xdpy, -1, wx + x, wy + y, CurrentTime); XFlush(xdpy); }
static void click(int button) {
  XTestFakeButtonEvent(xdpy, button, True, CurrentTime);
  XTestFakeButtonEvent(xdpy, button, False, CurrentTime);
  XFlush(xdpy);
}

/* Settles long enough that SDL's click state resets (if it does), then labels the next run. */
static void reset(const char *step) { move(100, 75); pump_report(step, 1500); }

static int clicks(void) {
  editor_hints();
  if (!SDL_InitSubSystem(SDL_INIT_VIDEO)) { printf("init failed: %s\n", SDL_GetError()); return 1; }
  printf("driver %s\n", SDL_GetCurrentVideoDriver());
  if (strcmp(SDL_GetCurrentVideoDriver(), "x11") != 0) { printf("clicks needs the x11 driver\n"); return 1; }
  SDL_Window *win = editor_window(200, 150);
  SDL_ShowWindow(win);
  SDL_SyncWindow(win);
  xdpy = SDL_GetPointerProperty(SDL_GetWindowProperties(win), SDL_PROP_WINDOW_X11_DISPLAY_POINTER, NULL);
  SDL_GetWindowPosition(win, &wx, &wy);
  int ev, er, maj, min;
  if (!xdpy || !XTestQueryExtension(xdpy, &ev, &er, &maj, &min)) { printf("no XTest\n"); return 1; }
  reset("settle");

  printf("# rapid same-point clicks, 60ms apart\n");
  for (int i = 0; i < 5; i++) { click(1); pump_report("rapid", 60); }

  printf("# second click delayed by N ms\n");
  static const int gaps[] = {100, 300, 400, 450, 490, 510, 550, 700, 1000};
  for (size_t i = 0; i < SDL_arraysize(gaps); i++) {
    char step[32];
    reset("reset");
    snprintf(step, sizeof step, "gap%d", gaps[i]);
    click(1); pump_report(step, gaps[i]);
    click(1); pump_report(step, 60);
  }

  printf("# second click moved by N px (60ms apart)\n");
  static const int offsets[] = {1, 2, 3, 4, 5, 8, 16, 31, 32, 33, 64};
  for (size_t i = 0; i < SDL_arraysize(offsets); i++) {
    char step[32];
    reset("reset");
    snprintf(step, sizeof step, "move%d", offsets[i]);
    click(1); pump_report(step, 60);
    move(100 + offsets[i], 75); pump_report(step, 20);
    click(1); pump_report(step, 60);
  }

  printf("# second click moved by (dx,dy), and a chain measured from which click\n");
  static const int moves[][2] = {{0, 32}, {0, 33}, {32, 32}, {33, 33}, {-32, 0}, {-33, 0}};
  for (size_t i = 0; i < SDL_arraysize(moves); i++) {
    char step[32];
    reset("reset");
    snprintf(step, sizeof step, "move%d,%d", moves[i][0], moves[i][1]);
    click(1); pump_report(step, 60);
    move(100 + moves[i][0], 75 + moves[i][1]); pump_report(step, 20);
    click(1); pump_report(step, 60);
  }
  reset("reset");
  for (int i = 0; i < 4; i++) {
    move(60 + 20 * i, 75); pump_report("chain20", 20);
    click(1); pump_report("chain20", 60);
  }

  printf("# buttons interleaved (60ms apart)\n");
  reset("reset");
  click(1); pump_report("mixed", 60);
  click(3); pump_report("mixed", 60);
  click(1); pump_report("mixed", 60);
  click(3); pump_report("mixed", 60);
  click(2); pump_report("mixed", 60);

  SDL_DestroyWindow(win);
  SDL_Quit();
  return 0;
}
#endif

int main(int argc, char **argv) {
  const char *mode = argc > 1 ? argv[1] : "";
  setvbuf(stdout, NULL, _IOLBF, 0);
  if (!strcmp(mode, "version")) return version();
  if (!strcmp(mode, "draw")) return draw();
#ifdef PROBE_CLICKS
  if (!strcmp(mode, "clicks")) return clicks();
#endif
  fprintf(stderr, "usage: %s version|draw|clicks\n", argv[0]);
  return 2;
}
