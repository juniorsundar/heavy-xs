/* A real native plugin, built against the shipped header exactly as a user would.
 * It exercises the Lua C API through the XL resolver: stack ops, strings, tables,
 * a metatable, a callback into Lua, an error, and a userdata with __gc. */
#define LITE_XL_PLUGIN_ENTRYPOINT
#include "lite_xl_plugin_api.h"

static int gc_count = 0;

static int p_add(lua_State *L) {
  lua_pushinteger(L, luaL_checkinteger(L, 1) + luaL_checkinteger(L, 2));
  return 1;
}

static int p_join(lua_State *L) {
  luaL_Buffer b;
  int n = lua_gettop(L);
  luaL_buffinit(L, &b);
  for (int i = 1; i <= n; i++) {
    if (i > 1) luaL_addstring(&b, "|");
    size_t len; const char *s = luaL_tolstring(L, i, &len);
    luaL_addlstring(&b, s, len);
    lua_pop(L, 1);
  }
  luaL_pushresult(&b);
  return 1;
}

static int p_table(lua_State *L) {
  lua_createtable(L, 3, 1);
  for (int i = 1; i <= 3; i++) { lua_pushinteger(L, i * i); lua_rawseti(L, -2, i); }
  lua_pushstring(L, "v"); lua_setfield(L, -2, "k");
  return 1;
}

static int p_call(lua_State *L) {        /* calls arg1(arg2) through lua_pcall */
  luaL_checktype(L, 1, LUA_TFUNCTION);
  lua_pushvalue(L, 1); lua_pushvalue(L, 2);
  int rc = lua_pcall(L, 1, 1, 0);
  lua_pushinteger(L, rc);
  lua_insert(L, -2);
  return 2;
}

static int p_fail(lua_State *L) { return luaL_error(L, "plugin error %d", 42); }

static int p_gc(lua_State *L) { gc_count++; return 0; }
static int p_gc_count(lua_State *L) { lua_pushinteger(L, gc_count); return 1; }

static int p_userdata(lua_State *L) {
  int *p = lua_newuserdatauv(L, sizeof(int), 0);
  *p = 7;
  if (luaL_newmetatable(L, "contractplug.Thing")) {
    lua_pushcfunction(L, p_gc); lua_setfield(L, -2, "__gc");
  }
  lua_setmetatable(L, -2);
  return 1;
}

static int p_version(lua_State *L) { lua_pushnumber(L, lua_version(L)); return 1; }

static int p_typename(lua_State *L) {
  lua_pushstring(L, lua_typename(L, lua_type(L, 1)));
  return 1;
}

static const luaL_Reg funcs[] = {
  {"add", p_add}, {"join", p_join}, {"table", p_table}, {"call", p_call},
  {"fail", p_fail}, {"userdata", p_userdata}, {"gc_count", p_gc_count},
  {"version", p_version}, {"typename", p_typename}, {NULL, NULL}
};

int luaopen_lite_xl_contractplug(lua_State *L, void *XL) {
  lite_xl_plugin_init(XL);
  luaL_newlib(L, funcs);
  return 1;
}
