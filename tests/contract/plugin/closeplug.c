/* lua_closethread is in the plugin header but not in the C resolver's table. */
#define LITE_XL_PLUGIN_ENTRYPOINT
#include "lite_xl_plugin_api.h"
static int p_resolves(lua_State *L) {
  lua_pushboolean(L, 1);
  return 1;
}
int luaopen_lite_xl_closeplug(lua_State *L, void *XL) {
  void *(*resolve)(const char *) = XL;
  lite_xl_plugin_init(XL);
  lua_newtable(L);
  const char *names[] = { "lua_closethread", "lua_resetthread", "luaL_checkversion_", "luaopen_base", "api_load_libs", "lua_pushinteger", "no_such_symbol", "", "lua_", "LUA_pushinteger", NULL };
  for (int i = 0; names[i]; i++) {
    lua_pushstring(L, names[i]);
    lua_pushboolean(L, resolve(names[i]) != NULL);
    lua_settable(L, -3);
  }
  (void) p_resolves;
  return 1;
}
