#ifndef RENWINDOW_H
#define RENWINDOW_H

#include "renderer.h"

/* struct RenWindow and the renwin_* helpers now live in rust/src/renwindow.rs, which owns the
** struct: its layout changes with LITE_USE_SDL_RENDERER, so C must not mirror it. The one
** function the remaining C (renderer.c's update_font_scale) still calls is declared here. */

RenSurface renwin_get_surface(RenWindow *ren);

#endif
