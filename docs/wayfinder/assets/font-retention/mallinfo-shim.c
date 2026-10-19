/* Evidence-only LD_PRELOAD shim for the font-retention probe. Not linked into Lite XL.
 * io.open("/lxl-malloc/stats") returns a readable stream with glibc mallinfo2 counters;
 * io.open("/lxl-malloc/trim") runs malloc_trim(0) first, then returns the same counters.
 * Every other path goes to the real fopen/fopen64. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <malloc.h>
#include <stdio.h>
#include <string.h>

static FILE *stats(int trim) {
  int trimmed = trim ? malloc_trim(0) : -1;
  struct mallinfo2 m = mallinfo2();
  char buf[512];
  int n = snprintf(buf, sizeof buf,
    "arena=%zu\nordblks=%zu\nsmblks=%zu\nhblks=%zu\nhblkhd=%zu\nfsmblks=%zu\nuordblks=%zu\nfordblks=%zu\nkeepcost=%zu\ntrimmed=%d\n",
    m.arena, m.ordblks, m.smblks, m.hblks, m.hblkhd, m.fsmblks, m.uordblks, m.fordblks, m.keepcost, trimmed);
  FILE *f = tmpfile();
  if (!f) return NULL;
  fwrite(buf, 1, (size_t)n, f);
  rewind(f);
  return f;
}

static FILE *route(const char *p, const char *m, const char *sym) {
  if (!strcmp(p, "/lxl-malloc/stats")) return stats(0);
  if (!strcmp(p, "/lxl-malloc/trim")) return stats(1);
  FILE *(*real)(const char *, const char *) = (FILE *(*)(const char *, const char *))dlsym(RTLD_NEXT, sym);
  return real(p, m);
}
FILE *fopen(const char *p, const char *m) { return route(p, m, "fopen"); }
FILE *fopen64(const char *p, const char *m) { return route(p, m, "fopen64"); }
