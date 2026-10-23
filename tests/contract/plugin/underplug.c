/* Only an underscore-prefixed plain entrypoint: the loader retries every name with a leading "_"
 * after a failed lookup, as SDL_LoadFunction did. Returns a count without touching the stack,
 * like plainplug. */
int _luaopen_underplug(void *L) { (void) L; return 1; }
