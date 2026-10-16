#ifndef RENWINDOW_H
#define RENWINDOW_H

#include "renderer.h"

/* Rust owns RenWindow and the renwin_* helpers. C must not mirror the struct: its layout
** changes with LITE_USE_SDL_RENDERER. */

RenSurface renwin_get_surface(RenWindow *ren);

#endif
