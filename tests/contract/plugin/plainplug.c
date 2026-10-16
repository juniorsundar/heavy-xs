/* A plugin with only the plain luaopen_<name>(L) entrypoint (no XL argument). Because the
 * host does not export Lua symbols, it cannot call the Lua API at all: this one returns a
 * count without touching the stack, which is enough to prove the fallback entrypoint ran
 * (load_native_plugin treats a non-zero return as success). */
int luaopen_plainplug(void *L) { (void) L; return 1; }
