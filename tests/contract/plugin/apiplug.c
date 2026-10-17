/* Native plugin that calls (nearly) every function of the plugin API through the XL resolver,
 * built against the shipped header like any third-party plugin. Each t_* returns plain values
 * the Lua suite records; nothing here prints addresses or counts that vary between runs. */
#define LITE_XL_PLUGIN_ENTRYPOINT
#include <string.h>
#include <errno.h>
#include <stdarg.h>
#include "lite_xl_plugin_api.h"

#define T(name) static int t_##name(lua_State *L)

/* ---- stack ------------------------------------------------------------------------------ */
T(stack) {
  lua_settop(L, 0);
  for (int i = 1; i <= 5; i++) lua_pushinteger(L, i);
  lua_pushinteger(L, lua_absindex(L, -1));      /* 6 */
  lua_pushinteger(L, lua_gettop(L));            /* 7 */
  lua_rotate(L, 1, 2);                          /* 4 5 1 2 3 6 7 */
  lua_copy(L, 1, 3);                            /* 4 5 4 2 3 6 7 */
  lua_pushvalue(L, 2);                          /* ... 5 */
  lua_pushboolean(L, lua_checkstack(L, 100));
  lua_settop(L, lua_gettop(L));
  return lua_gettop(L);
}
T(xmove) {
  lua_State *co = lua_newthread(L);
  lua_pushinteger(L, 1); lua_pushstring(L, "x");
  lua_xmove(L, co, 2);
  lua_pushinteger(L, lua_gettop(co));
  lua_pushstring(L, lua_tolstring(co, -1, NULL));
  lua_pushinteger(L, lua_tointegerx(co, 1, NULL));
  return 3;
}

/* ---- type checks and conversions -------------------------------------------------------- */
static int cf(lua_State *L) { return 0; }
T(types) {
  lua_settop(L, 0);
  lua_pushinteger(L, 10); lua_pushnumber(L, 2.5); lua_pushstring(L, "12"); lua_pushstring(L, "abc");
  lua_pushcfunction(L, cf); lua_newuserdatauv(L, 4, 1); lua_pushlightuserdata(L, (void *) 0);
  lua_pushboolean(L, 1); lua_pushnil(L); lua_createtable(L, 0, 0); lua_pushthread(L);
  char out[512]; out[0] = 0;
  for (int i = 1; i <= 11; i++) {
    char b[64];
    sprintf(b, "%d:%s n%d s%d c%d i%d u%d;", i, lua_typename(L, lua_type(L, i)), lua_isnumber(L, i),
            lua_isstring(L, i), lua_iscfunction(L, i), lua_isinteger(L, i), lua_isuserdata(L, i));
    strcat(out, b);
  }
  lua_pushstring(L, out);
  return 1;
}
T(convert) {
  int ok = 0; char b[512]; b[0] = 0;
  lua_settop(L, 0);
  lua_pushstring(L, "0x10"); lua_pushstring(L, " 7 "); lua_pushnumber(L, 3.0); lua_pushnumber(L, 3.5);
  lua_pushstring(L, "x"); lua_pushboolean(L, 0); lua_pushinteger(L, 0); lua_pushnil(L);
  for (int i = 1; i <= 8; i++) {
    lua_Number n = lua_tonumberx(L, i, &ok); int nok = ok;
    lua_Integer k = lua_tointegerx(L, i, &ok); int kok = ok;
    char t[160]; sprintf(t, "%d:%g/%d,%lld/%d,b%d;", i, n, nok, (long long) k, kok, lua_toboolean(L, i));
    strcat(b, t);
  }
  lua_pushstring(L, b);
  lua_pushinteger(L, lua_rawlen(L, 2));
  lua_pushinteger(L, (lua_Integer) lua_stringtonumber(L, "1e2")); lua_pushinteger(L, (lua_Integer) lua_stringtonumber(L, "nope"));
  lua_pushboolean(L, lua_tocfunction(L, 1) == NULL);
  lua_pushboolean(L, lua_touserdata(L, 1) == NULL);
  lua_pushboolean(L, lua_tothread(L, 1) == NULL);
  lua_pushboolean(L, lua_topointer(L, 1) == NULL);
  return 8;
}
T(userdata) {
  lua_settop(L, 0);
  void *p = lua_newuserdatauv(L, 16, 2);
  memset(p, 7, 16);
  lua_pushboolean(L, lua_touserdata(L, 1) == p);
  lua_pushboolean(L, lua_topointer(L, 1) == p);
  lua_pushinteger(L, lua_rawlen(L, 1));
  lua_pushstring(L, "uv1"); lua_pushinteger(L, lua_setiuservalue(L, 1, 1));
  lua_pushstring(L, "uv3"); lua_pushinteger(L, lua_setiuservalue(L, 1, 3));
  lua_pushinteger(L, lua_getiuservalue(L, 1, 1)); lua_pushinteger(L, lua_getiuservalue(L, 1, 2));
  lua_pushinteger(L, lua_getiuservalue(L, 1, 3));
  lua_pushboolean(L, lua_isuserdata(L, 1));
  return lua_gettop(L) - 1;
}

/* ---- arithmetic, comparison, strings ---------------------------------------------------- */
T(arith) {
  char b[400]; b[0] = 0;
  static const int ops[] = { LUA_OPADD, LUA_OPSUB, LUA_OPMUL, LUA_OPMOD, LUA_OPPOW, LUA_OPDIV, LUA_OPIDIV,
                             LUA_OPBAND, LUA_OPBOR, LUA_OPBXOR, LUA_OPSHL, LUA_OPSHR };
  for (int i = 0; i < 12; i++) {
    lua_pushinteger(L, 13); lua_pushinteger(L, 4);
    lua_arith(L, ops[i]);
    char t[48]; sprintf(t, "%d=%s;", i, lua_tolstring(L, -1, NULL)); strcat(b, t);
    lua_pop(L, 1);
  }
  lua_pushnumber(L, 5); lua_arith(L, LUA_OPUNM); strcat(b, "unm="); strcat(b, lua_tolstring(L, -1, NULL)); lua_pop(L, 1);
  lua_pushinteger(L, 5); lua_arith(L, LUA_OPBNOT); strcat(b, ";bnot="); strcat(b, lua_tolstring(L, -1, NULL)); lua_pop(L, 1);
  lua_pushstring(L, b);
  return 1;
}
T(compare) {
  lua_settop(L, 0);
  lua_pushinteger(L, 1); lua_pushnumber(L, 1.0); lua_pushinteger(L, 2); lua_pushstring(L, "1");
  char b[200];
  sprintf(b, "raw12=%d raw14=%d eq=%d lt=%d le=%d lt23=%d eq14=%d bad=%d", lua_rawequal(L, 1, 2), lua_rawequal(L, 1, 4),
          lua_compare(L, 1, 2, LUA_OPEQ), lua_compare(L, 1, 3, LUA_OPLT), lua_compare(L, 2, 3, LUA_OPLE),
          lua_compare(L, 3, 2, LUA_OPLT), lua_compare(L, 1, 4, LUA_OPEQ), lua_compare(L, 1, 99, LUA_OPEQ));
  lua_pushstring(L, b);
  return 1;
}
T(concat) {
  lua_settop(L, 0);
  lua_pushstring(L, "a"); lua_pushinteger(L, 2); lua_pushnumber(L, 3.5); lua_pushstring(L, "z");
  lua_concat(L, 4);
  lua_concat(L, 0);
  lua_pushstring(L, "solo"); lua_concat(L, 1);
  return 3;
}
T(len) {
  lua_settop(L, 0);
  lua_pushstring(L, "hello"); lua_len(L, 1);
  lua_createtable(L, 3, 0); lua_pushinteger(L, 1); lua_rawseti(L, -2, 1); lua_pushinteger(L, 1); lua_rawseti(L, -2, 2);
  lua_pushinteger(L, luaL_len(L, 3)); lua_len(L, 3);
  return 3;
}
T(pushes) {
  lua_settop(L, 0);
  lua_pushnil(L); lua_pushnumber(L, 1.5); lua_pushinteger(L, -3);
  lua_pushlstring(L, "a\0b", 3); lua_pushstring(L, "str"); lua_pushboolean(L, 7);
  lua_pushfstring(L, "%d|%s|%f|%c|%U|%%|%I|%s", 42, "s", 1.5, 'x', 0x20AC, (lua_Integer) 1 << 40, "end");
  lua_pushinteger(L, lua_pushstring(L, NULL) == NULL);   /* pushes nil, returns NULL */
  lua_pushinteger(L, (lua_Integer) lua_rawlen(L, 4));
  lua_pushstring(L, lua_pushfstring(L, "n%d", 5)); /* returns pointer to internal copy */
  return lua_gettop(L);
}
static int up_fn(lua_State *L) {
  lua_pushvalue(L, lua_upvalueindex(1)); lua_pushvalue(L, lua_upvalueindex(2));
  lua_pushinteger(L, lua_upvalueindex(3) == lua_upvalueindex(3));
  return 3;
}
T(closure) {
  lua_pushstring(L, "u1"); lua_pushinteger(L, 22);
  lua_pushcclosure(L, up_fn, 2);
  return 1;
}

/* ---- tables ----------------------------------------------------------------------------- */
T(tables) {
  lua_settop(L, 0);
  lua_createtable(L, 4, 4);
  lua_pushstring(L, "v"); lua_setfield(L, 1, "k");
  lua_pushinteger(L, 5); lua_seti(L, 1, 3);
  lua_pushstring(L, "key"); lua_pushstring(L, "val"); lua_settable(L, 1);
  lua_pushinteger(L, 9); lua_pushstring(L, "nine"); lua_rawset(L, 1);
  lua_pushstring(L, "r1"); lua_rawseti(L, 1, 1);
  int a = 0; lua_pushstring(L, "ptr"); lua_rawsetp(L, 1, &a);
  char b[300]; b[0] = 0;
  strcat(b, lua_getfield(L, 1, "k") == LUA_TSTRING ? "k:" : "k?"); strcat(b, lua_tolstring(L, -1, NULL)); lua_pop(L, 1);
  lua_pushinteger(L, lua_geti(L, 1, 3)); strcat(b, ";i3="); strcat(b, lua_tolstring(L, -1, NULL)); lua_pop(L, 2);
  lua_pushstring(L, "key"); lua_pushinteger(L, lua_gettable(L, 1)); strcat(b, ";tbl="); strcat(b, lua_tolstring(L, -1, NULL)); lua_pop(L, 2);
  lua_pushinteger(L, 9); lua_pushinteger(L, lua_rawget(L, 1)); strcat(b, ";raw="); strcat(b, lua_tolstring(L, -2, NULL)); lua_pop(L, 2);
  lua_pushinteger(L, lua_rawgeti(L, 1, 1)); strcat(b, ";rgi="); strcat(b, lua_tolstring(L, -2, NULL)); lua_pop(L, 2);
  lua_pushinteger(L, lua_rawgetp(L, 1, &a)); strcat(b, ";rgp="); strcat(b, lua_tolstring(L, -2, NULL)); lua_pop(L, 2);
  lua_pushinteger(L, lua_getfield(L, 1, "missing")); strcat(b, ";miss="); strcat(b, lua_typename(L, lua_tointegerx(L, -1, NULL))); lua_pop(L, 2);
  lua_pushstring(L, b);
  return 1;
}
T(globals) {
  lua_pushstring(L, "gval"); lua_setglobal(L, "plugin_api_global");
  lua_pushinteger(L, lua_getglobal(L, "plugin_api_global"));
  lua_pushinteger(L, lua_getglobal(L, "plugin_api_no_such"));
  return 4;
}
T(next) {
  lua_settop(L, 0);
  lua_createtable(L, 0, 3);
  lua_pushinteger(L, 1); lua_setfield(L, 1, "a"); lua_pushinteger(L, 2); lua_setfield(L, 1, "b"); lua_pushinteger(L, 3); lua_setfield(L, 1, "c");
  long sum = 0; int n = 0;
  lua_pushnil(L);
  while (lua_next(L, 1)) { sum += lua_tointegerx(L, -1, NULL); n++; lua_pop(L, 1); }
  lua_pushinteger(L, n); lua_pushinteger(L, sum);
  return 2;
}
T(metatables) {
  lua_settop(L, 0);
  lua_newtable(L);                                       /* 1: object */
  lua_newtable(L); lua_pushstring(L, "mv"); lua_setfield(L, 2, "__index");   /* 2: its metatable */
  lua_pushinteger(L, lua_setmetatable(L, 1));            /* sets 2 as metatable of 1, pops it; returns 1 */
  lua_pushinteger(L, lua_getmetatable(L, 1));            /* pushes metatable, returns 1 */
  lua_pop(L, 1);
  lua_newtable(L);                                       /* a table with no metatable */
  lua_pushinteger(L, lua_getmetatable(L, lua_gettop(L) - 1));   /* returns 0, pushes nothing */
  lua_pushinteger(L, lua_getfield(L, 1, "anything"));    /* via __index: string */
  return lua_gettop(L) - 1;
}

/* ---- calls, errors, load/dump ------------------------------------------------------------ */
static int msgh(lua_State *L) { lua_pushfstring(L, "H(%s)", lua_tolstring(L, 1, NULL)); return 1; }
static int thrower(lua_State *L) { return luaL_error(L, "boom %d", 7); }
static int adder(lua_State *L) { lua_pushinteger(L, lua_tointegerx(L, 1, NULL) + lua_tointegerx(L, 2, NULL)); return 1; }
T(calls) {
  lua_settop(L, 0);
  lua_pushcfunction(L, adder); lua_pushinteger(L, 2); lua_pushinteger(L, 3); lua_callk(L, 2, 1, 0, NULL);   /* 1: 5 */
  lua_pushcfunction(L, msgh);                                                                              /* 2: handler */
  lua_pushcfunction(L, thrower);
  int rc1 = lua_pcallk(L, 0, 0, 2, 0, NULL);                                                               /* 3: "H(...)" */
  lua_pushinteger(L, rc1);                                                                                 /* 4 */
  lua_pushcfunction(L, thrower);
  int rc2 = lua_pcallk(L, 0, 0, 0, 0, NULL);                                                               /* 5: message */
  lua_pushinteger(L, rc2);                                                                                 /* 6 */
  lua_pushinteger(L, 5);
  int rc3 = lua_pcallk(L, 0, 0, 0, 0, NULL);                                                               /* 7: calling a number */
  lua_pushinteger(L, rc3);                                                                                 /* 8 */
  lua_remove(L, 2);
  return lua_gettop(L);
}
T(error) {
  lua_pushstring(L, "plain error value");
  return lua_error(L);
}
T(error_table) {
  lua_newtable(L); lua_pushinteger(L, 5); lua_setfield(L, -2, "code");
  return lua_error(L);
}
struct rd { const char *s; int done; };
static const char *reader(lua_State *L, void *ud, size_t *sz) {
  struct rd *r = ud; if (r->done) { *sz = 0; return NULL; }
  r->done = 1; *sz = strlen(r->s); return r->s;
}
static char dumpbuf[4096]; static size_t dumplen;
static int writer(lua_State *L, const void *p, size_t sz, void *ud) {
  if (dumplen + sz > sizeof dumpbuf) return 1;   /* a non-zero return makes lua_dump fail */
  memcpy(dumpbuf + dumplen, p, sz); dumplen += sz; return 0;
}
T(load_dump) {
  lua_settop(L, 0);
  struct rd r = { "return 6 * 7, ...", 0 };
  int rc1 = lua_load(L, reader, &r, "=chunk", NULL);     /* 1: function */
  lua_pushinteger(L, 1); lua_callk(L, 1, 2, 0, NULL);    /* 1: 42, 2: 1 */
  struct rd bad = { "return +", 0 };
  int rc2 = lua_load(L, reader, &bad, "=bad", NULL);     /* 3: error message */
  lua_pushinteger(L, rc1); lua_pushinteger(L, rc2);
  return lua_gettop(L);
}
T(dump) {
  lua_settop(L, 0);
  luaL_loadstring(L, "local a = ... return a * 2");      /* 1 */
  dumplen = 0;
  int rcd = lua_dump(L, writer, NULL, 1);
  int is_bin = dumplen > 4 && memcmp(dumpbuf, "\x1bLua", 4) == 0;
  int rcb = luaL_loadbufferx(L, dumpbuf, dumplen, "=bin", "b");     /* 2: function */
  lua_pushinteger(L, 21); lua_callk(L, 1, 1, 0, NULL);               /* 2: 42 */
  int rct = luaL_loadbufferx(L, dumpbuf, dumplen, "=bin", "t");     /* 3: error message */
  lua_pushinteger(L, rcd); lua_pushboolean(L, is_bin); lua_pushinteger(L, rcb); lua_pushinteger(L, rct);
  return lua_gettop(L) - 1;
}
T(loadfuncs) {
  lua_settop(L, 0);
  int a = luaL_loadstring(L, "return 'ls'"); lua_callk(L, 0, 1, 0, NULL);                       /* 1 */
  int b = luaL_loadbufferx(L, "return 'lb'", 11, "=buf", NULL); lua_callk(L, 0, 1, 0, NULL);   /* 2 */
  int c = luaL_loadfilex(L, "/nonexistent/lxl-api-file.lua", NULL);                            /* 3: message */
  int d = luaL_loadstring(L, "syntax error here");                                             /* 4: message */
  lua_pushinteger(L, a); lua_pushinteger(L, b); lua_pushinteger(L, c); lua_pushinteger(L, d);
  return lua_gettop(L);
}

/* ---- coroutines ------------------------------------------------------------------------- */
static int yielder(lua_State *L) { lua_pushinteger(L, 100); return lua_yieldk(L, 1, 0, NULL); }
T(coroutine) {
  lua_settop(L, 0);
  lua_State *co = lua_newthread(L);
  lua_pushcfunction(co, yielder);
  int nres = 0;
  lua_pushinteger(L, lua_status(co));
  lua_pushinteger(L, lua_isyieldable(L));
  lua_pushinteger(L, lua_resume(co, L, 0, &nres));        /* LUA_YIELD */
  lua_pushinteger(L, nres); lua_pushinteger(L, lua_tointegerx(co, -1, NULL));
  lua_pushinteger(L, lua_status(co));
  lua_pushinteger(L, lua_resume(co, L, 0, &nres));        /* finishes */
  lua_pushinteger(L, lua_status(co));
  lua_pushinteger(L, lua_resetthread(co));
  return lua_gettop(L) - 1;
}
T(thread_error) {
  lua_settop(L, 0);
  lua_State *co = lua_newthread(L);
  lua_pushcfunction(co, thrower);
  int nres = 0;
  lua_pushinteger(L, lua_resume(co, L, 0, &nres));
  lua_pushstring(L, lua_tolstring(co, -1, NULL));
  lua_pushinteger(L, lua_status(co));
  return 3;
}

/* ---- to-be-closed, warnings, panic, gc, allocator --------------------------------------- */
static int closed_count = 0;
static int closer(lua_State *L) { closed_count++; return 0; }
T(toclose) {
  lua_settop(L, 0);
  closed_count = 0;
  lua_newtable(L); lua_newtable(L); lua_pushcfunction(L, closer); lua_setfield(L, 2, "__close"); lua_setmetatable(L, 1);
  lua_toclose(L, 1);
  lua_pushinteger(L, closed_count);
  lua_closeslot(L, 1);
  lua_pushinteger(L, closed_count);
  return 2;
}
static char warnbuf[256];
static void warnf(void *ud, const char *msg, int tocont) { strncat(warnbuf, msg, 100); strcat(warnbuf, tocont ? "+" : "."); }
T(warning) {
  warnbuf[0] = 0;
  lua_setwarnf(L, warnf, NULL);
  lua_warning(L, "part1", 1); lua_warning(L, "part2", 0);
  lua_setwarnf(L, NULL, NULL);
  lua_warning(L, "ignored", 0);
  lua_pushstring(L, warnbuf);
  return 1;
}
static int panicked(lua_State *L) { return 0; }
T(atpanic) {
  lua_CFunction old = lua_atpanic(L, panicked);
  lua_CFunction mid = lua_atpanic(L, old);
  lua_pushboolean(L, mid == panicked);
  lua_pushboolean(L, old != NULL);
  return 2;
}
T(gc) {
  lua_settop(L, 0);
  char b[300];
  lua_gc(L, LUA_GCCOLLECT);
  int count = lua_gc(L, LUA_GCCOUNT), countb = lua_gc(L, LUA_GCCOUNTB);
  int running = lua_gc(L, LUA_GCISRUNNING);
  int stop = lua_gc(L, LUA_GCSTOP), runs2 = lua_gc(L, LUA_GCISRUNNING);
  int restart = lua_gc(L, LUA_GCRESTART), runs3 = lua_gc(L, LUA_GCISRUNNING);
  int step = lua_gc(L, LUA_GCSTEP, 0);
  int inc = lua_gc(L, LUA_GCINC, 0, 0, 0);          /* returns the previous mode */
  int gen = lua_gc(L, LUA_GCGEN, 20, 100);
  int back = lua_gc(L, LUA_GCINC, 200, 100, 13);
  int pause = lua_gc(L, LUA_GCSETPAUSE, 200), stepmul = lua_gc(L, LUA_GCSETSTEPMUL, 100);
  sprintf(b, "count>0:%d countb<1024:%d running:%d stop:%d runs:%d restart:%d runs:%d step:%d inc:%d gen:%d back:%d pause:%d stepmul:%d",
          count > 0, countb >= 0 && countb < 1024, running, stop, runs2, restart, runs3, step == 0 || step == 1,
          inc, gen, back, pause > 0, stepmul > 0);
  lua_pushstring(L, b);
  return 1;
}
T(alloc) {
  void *ud = NULL;
  lua_Alloc f = lua_getallocf(L, &ud);
  lua_pushboolean(L, f != NULL);
  void *p = f(ud, NULL, 0, 32);   /* allocate */
  lua_pushboolean(L, p != NULL);
  f(ud, p, 32, 0);                /* free */
  lua_setallocf(L, f, ud);        /* set it back unchanged */
  lua_pushinteger(L, lua_setcstacklimit(L, 200) > 0);
  return 3;
}
T(version) {
  lua_pushnumber(L, lua_version(L));
  luaL_checkversion_(L, 504, LUAL_NUMSIZES);
  lua_pushstring(L, "checkversion ok");
  return 2;
}

/* ---- debug interface -------------------------------------------------------------------- */
static int dbg_target(lua_State *L) {
  lua_Debug ar; char b[300]; b[0] = 0;
  int lvl = lua_getstack(L, 0, &ar);
  lua_getinfo(L, "nSltu", &ar);
  sprintf(b, "lvl=%d what=%s src=%s curline=%d nups=%d nparams=%d", lvl, ar.what, ar.short_src, ar.currentline, ar.nups, ar.nparams);
  lua_pushstring(L, b);
  lua_pushinteger(L, lua_getstack(L, 50, &ar));
  return 2;
}
T(debug_info) {
  lua_settop(L, 0);
  lua_pushcfunction(L, dbg_target); lua_callk(L, 0, 2, 0, NULL);                  /* 1: info string, 2: getstack(50) */
  luaL_loadstring(L, "local up = 5 return function(a, b) local c = a + b + up return c end");
  lua_callk(L, 0, 1, 0, NULL);                                                     /* 3: the inner function */
  lua_Debug ar;
  lua_pushvalue(L, 3);
  lua_getinfo(L, ">S", &ar);
  char b[200]; sprintf(b, "what=%s line=%d last=%d", ar.what, ar.linedefined, ar.lastlinedefined);
  lua_pushstring(L, b);                                                            /* 4 */
  lua_remove(L, 3);
  return 3;                                                                        /* info, getstack result, summary */
}
T(upvalues) {
  lua_settop(L, 0);
  luaL_loadstring(L, "local x, y = 1, 2 return function() return x + y end");
  lua_callk(L, 0, 1, 0, NULL);                                       /* 1: closure */
  const char *n1 = lua_getupvalue(L, 1, 1);                           /* 2: value of upvalue 1 */
  lua_pushstring(L, n1 ? n1 : "(nil)");                               /* 3: its name */
  lua_pushinteger(L, 40); const char *n2 = lua_setupvalue(L, 1, 2);   /* sets upvalue 2 to 40, pops */
  lua_pushstring(L, n2 ? n2 : "(nil)");                               /* 4 */
  lua_pushvalue(L, 1); lua_callk(L, 0, 1, 0, NULL);                   /* 5: 1 + 40 */
  lua_pushboolean(L, lua_getupvalue(L, 1, 9) == NULL);                /* 6: no such upvalue */
  luaL_loadstring(L, "local a = 1 return function() return a end, function() return a end");
  lua_callk(L, 0, 2, 0, NULL);                                        /* 7, 8: share one upvalue */
  lua_pushboolean(L, lua_upvalueid(L, 7, 1) == lua_upvalueid(L, 8, 1));      /* 9 */
  luaL_loadstring(L, "local b = 2 return function() return b end");
  lua_callk(L, 0, 1, 0, NULL);                                        /* 10: independent upvalue */
  lua_pushboolean(L, lua_upvalueid(L, 7, 1) != lua_upvalueid(L, 10, 1));     /* 11 */
  lua_upvaluejoin(L, 7, 1, 10, 1);
  lua_pushboolean(L, lua_upvalueid(L, 7, 1) == lua_upvalueid(L, 10, 1));     /* 12 */
  lua_pushvalue(L, 7); lua_callk(L, 0, 1, 0, NULL);                   /* 13: now returns b = 2 */
  return lua_gettop(L) - 6;                                           /* skip the closures: results 2.. */
}
static int hook_hits;
static void hook(lua_State *L, lua_Debug *ar) { hook_hits++; }
T(hooks) {
  lua_settop(L, 0);
  hook_hits = 0;
  lua_sethook(L, hook, LUA_MASKCOUNT, 10);
  lua_pushboolean(L, lua_gethook(L) == hook);
  lua_pushinteger(L, lua_gethookmask(L)); lua_pushinteger(L, lua_gethookcount(L));
  luaL_loadstring(L, "local n = 0 for i = 1, 1000 do n = n + i end return n");
  lua_callk(L, 0, 1, 0, NULL);
  lua_sethook(L, NULL, 0, 0);
  lua_pushboolean(L, hook_hits > 10);
  lua_pushboolean(L, lua_gethook(L) == NULL);
  lua_pushinteger(L, lua_gethookmask(L));
  return lua_gettop(L);
}
static int probe(lua_State *L) {
  lua_Debug ar; char b[300]; b[0] = 0;
  lua_getstack(L, 1, &ar);
  lua_getinfo(L, "nl", &ar);
  for (int i = 1; ; i++) {
    const char *name = lua_getlocal(L, &ar, i);
    if (!name) break;
    char t[80]; sprintf(t, "%s=%s;", name, lua_type(L, -1) == LUA_TNUMBER ? "num" : "str"); strcat(b, t);
    lua_pop(L, 1);
  }
  lua_pushinteger(L, 99); lua_pushstring(L, lua_setlocal(L, &ar, 1));
  lua_pushstring(L, b);
  return 2;
}
T(locals2) {
  lua_settop(L, 0);
  lua_pushcfunction(L, probe); lua_setglobal(L, "plugin_probe");
  luaL_loadstring(L, "local function f(p, q)\n  local r = p + q\n  return plugin_probe(r)\nend\nreturn f(3, 4)");
  lua_callk(L, 0, 2, 0, NULL);
  return 2;
}

/* ---- auxiliary library ------------------------------------------------------------------ */
T(metafield) {
  lua_settop(L, 0);
  lua_newtable(L); lua_newtable(L);
  lua_pushcfunction(L, adder); lua_setfield(L, 2, "__call");
  lua_pushstring(L, "name!"); lua_setfield(L, 2, "__name");
  lua_pushcfunction(L, adder); lua_setfield(L, 2, "__sample");
  lua_setmetatable(L, 1);
  lua_pushinteger(L, luaL_getmetafield(L, 1, "__name")); lua_pushinteger(L, luaL_getmetafield(L, 1, "__nope"));
  lua_pushvalue(L, 1); lua_pushinteger(L, 4); lua_pushinteger(L, luaL_callmeta(L, 1, "__sample") ? 1 : 0);
  return lua_gettop(L) - 1;
}
T(tolstring) {
  lua_settop(L, 0);
  lua_pushinteger(L, 5); lua_pushnumber(L, 2.0); lua_pushnil(L); lua_pushboolean(L, 1); lua_pushstring(L, "s");
  char b[200]; b[0] = 0;
  for (int i = 1; i <= 5; i++) { size_t l; const char *s = luaL_tolstring(L, i, &l); char t[40]; sprintf(t, "%s/%d;", s, (int) l); strcat(b, t); lua_pop(L, 1); }
  lua_newtable(L); lua_newtable(L); lua_pushstring(L, "MyType"); lua_setfield(L, -2, "__name"); lua_setmetatable(L, -2);
  const char *s = luaL_tolstring(L, -1, NULL);
  lua_pushboolean(L, strncmp(s, "MyType: ", 8) == 0);
  lua_pushstring(L, b);
  return 2;
}
T(argcheck) {
  /* argument i selects the check to run, so the Lua side can drive every outcome */
  int which = (int) luaL_checkinteger(L, 1);
  switch (which) {
    case 1: lua_pushstring(L, luaL_checklstring(L, 2, NULL)); return 1;
    case 2: lua_pushstring(L, luaL_optlstring(L, 2, "dflt", NULL)); return 1;
    case 3: lua_pushnumber(L, luaL_checknumber(L, 2)); return 1;
    case 4: lua_pushnumber(L, luaL_optnumber(L, 2, 1.25)); return 1;
    case 5: lua_pushinteger(L, luaL_checkinteger(L, 2)); return 1;
    case 6: lua_pushinteger(L, luaL_optinteger(L, 2, 77)); return 1;
    case 7: luaL_checktype(L, 2, LUA_TTABLE); lua_pushstring(L, "table ok"); return 1;
    case 8: luaL_checkany(L, 2); lua_pushstring(L, "any ok"); return 1;
    case 9: return luaL_argerror(L, 2, "custom detail");
    case 10: return luaL_typeerror(L, 2, "WantedType");
    case 11: { static const char *const opts[] = { "one", "two", "three", NULL };
               lua_pushinteger(L, luaL_checkoption(L, 2, "two", opts)); return 1; }
    case 12: luaL_checkstack(L, 1000000000, "too much"); return 0;
    case 13: luaL_where(L, 1); return 1;
    case 14: return luaL_error(L, "formatted %s %d", "error", 5);
    case 15: lua_pushstring(L, luaL_checkudata(L, 2, "NoSuchUdataType") ? "udata" : "null"); return 1;
  }
  return 0;
}
T(udata) {
  lua_settop(L, 0);
  void *p = lua_newuserdatauv(L, 8, 0);                  /* 1 */
  int created = luaL_newmetatable(L, "ApiPlug.Type");   /* 2: new metatable, created = 1 */
  int again = luaL_newmetatable(L, "ApiPlug.Type");     /* 3: the same one, again = 0 */
  lua_pop(L, 1);
  lua_pushstring(L, "x"); lua_setfield(L, 2, "tag");
  lua_pop(L, 1);
  luaL_setmetatable(L, "ApiPlug.Type");                  /* sets it on the userdata, pops nothing extra */
  lua_pushboolean(L, luaL_testudata(L, 1, "ApiPlug.Type") == p);
  lua_pushboolean(L, luaL_testudata(L, 1, "Other") == NULL);
  lua_pushboolean(L, luaL_checkudata(L, 1, "ApiPlug.Type") == p);
  lua_pushinteger(L, created); lua_pushinteger(L, again);
  int mt_type = luaL_getmetatable(L, "ApiPlug.Type");   /* macro: lua_getfield on the registry; pushes the table */
  lua_getfield(L, -1, "tag"); lua_remove(L, -2);
  lua_pushboolean(L, mt_type == LUA_TTABLE);
  return lua_gettop(L) - 1;
}
T(refs) {
  lua_settop(L, 0);
  lua_newtable(L);
  lua_pushstring(L, "first"); int r1 = luaL_ref(L, 1);
  lua_pushstring(L, "second"); int r2 = luaL_ref(L, 1);
  lua_pushnil(L); int r3 = luaL_ref(L, 1);
  lua_rawgeti(L, 1, r1); lua_rawgeti(L, 1, r2);
  luaL_unref(L, 1, r1);
  lua_pushstring(L, "third"); int r4 = luaL_ref(L, 1);   /* reuses the freed slot */
  lua_pushinteger(L, r1 == r4); lua_pushinteger(L, r1 != r2); lua_pushinteger(L, r3 == LUA_REFNIL);
  luaL_unref(L, 1, LUA_NOREF); luaL_unref(L, 1, LUA_REFNIL);
  return 5;
}
T(strfuncs) {
  lua_settop(L, 0);
  const char *g = luaL_gsub(L, "a.b.c", ".", "--");
  lua_pushstring(L, g);
  luaL_Buffer b; luaL_buffinit(L, &b); luaL_addgsub(&b, "xAyAz", "A", "[]"); luaL_pushresult(&b);
  return 3;
}
T(buffer) {
  lua_settop(L, 0);
  luaL_Buffer b;
  luaL_buffinit(L, &b);
  luaL_addstring(&b, "head|");
  luaL_addlstring(&b, "a\0b", 3);
  lua_pushstring(L, "|val"); luaL_addvalue(&b);
  char *p = luaL_prepbuffsize(&b, 8000);                /* forces the box (userdata) path */
  memset(p, 'z', 8000); luaL_addsize(&b, 8000);
  luaL_pushresult(&b);
  lua_pushinteger(L, (lua_Integer) lua_rawlen(L, -1));
  luaL_Buffer c; char *q = luaL_buffinitsize(L, &c, 5); memcpy(q, "12345", 5); luaL_pushresultsize(&c, 5);
  luaL_Buffer d; luaL_buffinit(L, &d); char *r = luaL_prepbuffer(&d); memcpy(r, "ab", 2); luaL_addsize(&d, 2); luaL_addchar(&d, 'c'); luaL_pushresult(&d);
  return 4;
}
static int regf1(lua_State *L) { lua_pushstring(L, lua_tolstring(L, lua_upvalueindex(1), NULL)); return 1; }
T(setfuncs) {
  lua_settop(L, 0);
  static const luaL_Reg fs[] = { {"one", regf1}, {"two", regf1}, {"placeholder", NULL}, {NULL, NULL} };
  lua_newtable(L);
  lua_pushstring(L, "upv");
  luaL_setfuncs(L, fs, 1);
  lua_pushinteger(L, luaL_getsubtable(L, 1, "sub"));
  lua_pushinteger(L, luaL_getsubtable(L, 1, "sub"));
  lua_pushinteger(L, lua_rawlen(L, 3) == 0);
  lua_getfield(L, 1, "placeholder");
  return 5;
}
static int req_open(lua_State *L) { lua_newtable(L); lua_pushstring(L, "opened"); lua_setfield(L, -2, "state"); return 1; }
T(requiref) {
  lua_settop(L, 0);
  luaL_requiref(L, "apiplug_required", req_open, 1);
  lua_getglobal(L, "apiplug_required");
  lua_pushboolean(L, lua_rawequal(L, 1, 2));
  lua_pop(L, 2);
  luaL_requiref(L, "apiplug_required2", req_open, 0);
  lua_getglobal(L, "apiplug_required2"); lua_pushboolean(L, lua_type(L, -1) == LUA_TNIL); lua_remove(L, -2);
  return 3;
}
T(traceback) {
  lua_settop(L, 0);
  luaL_traceback(L, L, "msg", 0);
  size_t l; const char *s = lua_tolstring(L, -1, &l);
  lua_pushboolean(L, strncmp(s, "msg\nstack traceback:", 20) == 0);
  return 1;
}
T(results) {
  lua_settop(L, 0);
  /* These read errno, so set it: the value left by earlier calls is not part of the contract. */
  errno = 0;      lua_pushinteger(L, luaL_fileresult(L, 1, NULL));
  errno = ENOENT; lua_pushinteger(L, luaL_fileresult(L, 0, "fname"));
  errno = EACCES; lua_pushinteger(L, luaL_fileresult(L, 0, NULL));
  errno = 0;      lua_pushinteger(L, luaL_execresult(L, 0));
  errno = ENOENT; lua_pushinteger(L, luaL_execresult(L, -1));
  errno = 0;      lua_pushinteger(L, luaL_execresult(L, 256 * 3));   /* exit code 3 */
  return lua_gettop(L);
}

/* ---- states and standard libraries ------------------------------------------------------ */
T(newstate) {
  lua_State *N = luaL_newstate();
  lua_pushboolean(L, N != NULL);
  luaL_openlibs(N);
  lua_pushinteger(L, luaL_loadstring(N, "return string.rep('ab', 3) .. math.floor(2.5) .. utf8.char(8364)"));
  lua_pushinteger(L, lua_pcallk(N, 0, 1, 0, 0, NULL));
  lua_pushstring(L, lua_tolstring(N, -1, NULL));
  lua_close(N);
  return 4;
}
static int open_lib(lua_State *L) { return 0; }
T(openlibs) {
  static const struct { const char *name; lua_CFunction f; } libs[] = {
    {"_G", luaopen_base}, {"coroutine", luaopen_coroutine}, {"table", luaopen_table}, {"io", luaopen_io},
    {"os", luaopen_os}, {"string", luaopen_string}, {"utf8", luaopen_utf8}, {"math", luaopen_math},
    {"debug", luaopen_debug}, {"package", luaopen_package}, {NULL, NULL} };
  lua_State *N = luaL_newstate();
  char b[400]; b[0] = 0;
  for (int i = 0; libs[i].name; i++) {
    luaL_requiref(N, libs[i].name, libs[i].f, 1);
    /* Count the entries so a swapped opener (math <-> string, ...) changes the record. */
    int n = 0; lua_pushnil(N);
    while (lua_next(N, -2)) { n++; lua_pop(N, 1); }
    char t[60]; sprintf(t, "%s=%s/%d;", libs[i].name, lua_typename(N, lua_type(N, -1)), n); strcat(b, t);
    lua_pop(N, 1);
  }
  luaL_loadstring(N, "return type(print) .. type(string.format) .. type(io.write)");
  lua_pcallk(N, 0, 1, 0, 0, NULL);
  lua_pushstring(L, b); lua_pushstring(L, lua_tolstring(N, -1, NULL));
  lua_close(N);
  (void) open_lib;
  return 2;
}

static const luaL_Reg tests[] = {
  {"stack", t_stack}, {"xmove", t_xmove}, {"types", t_types}, {"convert", t_convert}, {"userdata", t_userdata},
  {"arith", t_arith}, {"compare", t_compare}, {"concat", t_concat}, {"len", t_len}, {"pushes", t_pushes},
  {"closure", t_closure}, {"tables", t_tables}, {"globals", t_globals}, {"next", t_next}, {"metatables", t_metatables},
  {"calls", t_calls}, {"error", t_error}, {"error_table", t_error_table}, {"load_dump", t_load_dump}, {"dump", t_dump},
  {"loadfuncs", t_loadfuncs}, {"coroutine", t_coroutine}, {"thread_error", t_thread_error}, {"toclose", t_toclose},
  {"warning", t_warning}, {"atpanic", t_atpanic}, {"gc", t_gc}, {"alloc", t_alloc}, {"version", t_version},
  {"debug_info", t_debug_info}, {"upvalues", t_upvalues}, {"hooks", t_hooks}, {"locals", t_locals2},
  {"metafield", t_metafield}, {"tolstring", t_tolstring}, {"argcheck", t_argcheck}, {"udata", t_udata},
  {"refs", t_refs}, {"strfuncs", t_strfuncs}, {"buffer", t_buffer}, {"setfuncs", t_setfuncs}, {"requiref", t_requiref},
  {"traceback", t_traceback}, {"results", t_results}, {"newstate", t_newstate}, {"openlibs", t_openlibs},
  {NULL, NULL}
};

int luaopen_lite_xl_apiplug(lua_State *L, void *XL) {
  lite_xl_plugin_init(XL);
  luaL_newlib(L, tests);
  return 1;
}
