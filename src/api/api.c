#include "api.h"

int luaopen_system(lua_State *L);
int luaopen_renderer(lua_State *L);
int luaopen_renwindow(lua_State *L);
int luaopen_regex(lua_State *L);
int luaopen_process(lua_State *L);
int luaopen_dirmonitor(lua_State* L);
int luaopen_utf8extra(lua_State* L);
void lxl_rust_system_overlay(lua_State *L); // rust/src/lib.rs: ported system.* functions

static const luaL_Reg libs[] = {
  { "system",     luaopen_system     },
  { "renderer",   luaopen_renderer   },
  { "renwindow",  luaopen_renwindow  },
  { "regex",      luaopen_regex      },
  { "process",    luaopen_process    },
  { "dirmonitor", luaopen_dirmonitor },
  { "utf8extra",  luaopen_utf8extra  },
  { NULL, NULL }
};


void api_load_libs(lua_State *L) {
  for (int i = 0; libs[i].name; i++)
    luaL_requiref(L, libs[i].name, libs[i].func, 1);
  // W5 overlay: replaces the system.* functions that now live in Rust. Removed with system.c.
  lxl_rust_system_overlay(L);
}

