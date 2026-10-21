/* Only an underscore-prefixed plain entrypoint: SDL_LoadFunction retried every name with a
 * leading "_" after a failed lookup. Returns a count without touching the stack, like plainplug. */
int _luaopen_underplug(void *L) { (void) L; return 1; }
