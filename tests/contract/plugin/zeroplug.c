/* Returns 0 from its entrypoint: load_native_plugin must report that as an error. */
#define LITE_XL_PLUGIN_ENTRYPOINT
#include "lite_xl_plugin_api.h"
int luaopen_lite_xl_zeroplug(lua_State *L, void *XL) { lite_xl_plugin_init(XL); return 0; }
