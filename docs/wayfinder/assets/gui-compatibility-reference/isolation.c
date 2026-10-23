/* Trusted negative checks; run before exposing any plugin to the sandbox. */
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

int main(int argc, char **argv) {
  if (argc == 2 && !strcmp(argv[1], "--launch")) {
    const char *names[] = {"session.lua", "fixture.lua"};
    const char *dest[] = {"/work/user/session.lua", "/work/project/fixture.lua"};
    for (int i = 0; i < 2; i++) {
      char src[128], buf[8192];
      snprintf(src, sizeof(src), "/input/%s", names[i]);
      FILE *in = fopen(src, "rb"), *out = fopen(dest[i], "wb");
      assert(in && out); size_t n;
      while ((n = fread(buf, 1, sizeof(buf), in))) assert(fwrite(buf, 1, n, out) == n);
      assert(!ferror(in)); fclose(in); assert(!fclose(out));
    }
    execl("/opt/bin/lite-xl", "lite-xl", "/work/project/fixture.lua", (char *)NULL);
    perror("exec reference"); return 1;
  }
  assert(argc == 4);
  assert(getpid() < 10 && !isatty(STDIN_FILENO));
  assert(!getenv("SSH_AUTH_SOCK") && !getenv("WAYLAND_DISPLAY") && !getenv("LD_PRELOAD"));
  assert(!strcmp(getenv("HOME"), "/work/home"));
  assert(open(argv[1], O_RDONLY) == -1); /* existing host-only canary */
  assert(open(argv[2], O_RDONLY) == -1); /* existing checkout file */
  assert(access("/opt/bin/lite-xl", W_OK) == -1);
  char net[128];
  ssize_t n = readlink("/proc/self/ns/net", net, sizeof(net)-1);
  assert(n > 0); net[n] = 0;
  assert(strcmp(net, argv[3]) != 0);
  /* An isolated network namespace has no host routes. */
  FILE *f = fopen("/proc/net/route", "r");
  assert(f); char line[512]; assert(fgets(line, sizeof(line), f));
  assert(!fgets(line, sizeof(line), f)); fclose(f);
  struct rlimit limit;
  assert(!getrlimit(RLIMIT_AS, &limit) && limit.rlim_cur <= 2147483648ULL);
  assert(!getrlimit(RLIMIT_NPROC, &limit) && limit.rlim_cur <= 128);
  assert(!getrlimit(RLIMIT_FSIZE, &limit) && limit.rlim_cur <= 16777216);
  int out = open("/work/write-canary", O_WRONLY | O_CREAT | O_EXCL, 0600);
  assert(out >= 0 && write(out, "ok", 2) == 2); close(out);
  /* tmpfs size, mounts, and namespace declarations are retained by the runner. */
  puts("ISOLATION filesystem=private network=private pid=private env=clean bounds=set");
  return 0;
}
