/* Exports a symbol for consplug; whether consplug can see it shows the dlopen scope. */
int lxl_contract_provided(void) { return 1; }
int luaopen_provplug(void *L) { (void) L; return 1; }
