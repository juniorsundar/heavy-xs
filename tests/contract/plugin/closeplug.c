/* Asks the XL resolver for a fixed list of names, including lua_closethread (in the plugin
 * header, but missing from the C build's resolver table), and returns name -> resolved. */
#define LITE_XL_PLUGIN_ENTRYPOINT
#include "lite_xl_plugin_api.h"
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
  return 1;
}
