/* Needs provplug's symbol at load time: it only resolves if provplug was opened RTLD_GLOBAL. */
extern int lxl_contract_provided(void);
int luaopen_consplug(void *L) { (void) L; return lxl_contract_provided(); }
