/* Verbatim copy of hit_test() and its struct from the deleted src/api/system.c (last present at
 * commit 0448df7d). scripts/rust-port/hit-test-check.sh compiles it as the reference that
 * src/hit.rs is diffed against. Do not "fix" it: it is the oracle. */
struct HitTestInfo {
  int title_height;
  int controls_width;
  int resize_border;
};
typedef struct HitTestInfo HitTestInfo;
#define RESIZE_FROM_TOP 0
#define RESIZE_FROM_RIGHT 0
static SDL_HitTestResult SDLCALL hit_test(SDL_Window *window, const SDL_Point *pt, void *data) {
  const HitTestInfo *hit_info = (HitTestInfo *) data;
  const int resize_border = hit_info->resize_border;
  const int controls_width = hit_info->controls_width;
  int w, h;

  SDL_GetWindowSize(window, &w, &h);

  if (pt->y < hit_info->title_height &&
    #if RESIZE_FROM_TOP
    pt->y > hit_info->resize_border &&
    #endif
    pt->x > resize_border && pt->x < w - controls_width) {
    return SDL_HITTEST_DRAGGABLE;
  }

  #define REPORT_RESIZE_HIT(name) { \
    return SDL_HITTEST_RESIZE_##name; \
  }

  if (pt->x < resize_border && pt->y < resize_border) {
    REPORT_RESIZE_HIT(TOPLEFT);
  #if RESIZE_FROM_TOP
  } else if (pt->x > resize_border && pt->x < w - controls_width && pt->y < resize_border) {
    REPORT_RESIZE_HIT(TOP);
  #endif
  } else if (pt->x > w - resize_border && pt->y < resize_border) {
    REPORT_RESIZE_HIT(TOPRIGHT);
  #if RESIZE_FROM_RIGHT
  } else if (pt->x > w - resize_border && pt->y > resize_border && pt->y < h - resize_border) {
    REPORT_RESIZE_HIT(RIGHT);
  #endif
  } else if (pt->x > w - resize_border && pt->y > h - resize_border) {
    REPORT_RESIZE_HIT(BOTTOMRIGHT);
  } else if (pt->x < w - resize_border && pt->x > resize_border && pt->y > h - resize_border) {
    REPORT_RESIZE_HIT(BOTTOM);
  } else if (pt->x < resize_border && pt->y > h - resize_border) {
    REPORT_RESIZE_HIT(BOTTOMLEFT);
  } else if (pt->x < resize_border && pt->y < h - resize_border && pt->y > resize_border) {
    REPORT_RESIZE_HIT(LEFT);
  }

  return SDL_HITTEST_NORMAL;
}
