/* Calls a function no library defines: loading with immediate binding must fail. */
extern int lxl_contract_symbol_that_does_not_exist(void);
int luaopen_undefplug(void *L) { (void) L; return lxl_contract_symbol_that_does_not_exist(); }
