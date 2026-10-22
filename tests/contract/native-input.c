/* XTEST input on the check's private X server, never the host display. */
#include <X11/Xlib.h>
#include <X11/XKBlib.h>
#include <X11/keysym.h>
#include <X11/extensions/XTest.h>
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static Window find_window(Display *d, Window root) {
  char *name = NULL;
  if (XFetchName(d, root, &name) && name) {
    int found = !strcmp(name, "native-input-check");
    XFree(name);
    if (found) return root;
  }
  Window parent, tree_root, *children = NULL, found = 0;
  unsigned count = 0;
  if (XQueryTree(d, root, &tree_root, &parent, &children, &count)) {
    for (unsigned i = 0; i < count && !found; ++i) found = find_window(d, children[i]);
    if (children) XFree(children);
  }
  return found;
}

static void key(Display *d, KeySym sym, int down) {
  KeyCode code = XKeysymToKeycode(d, sym);
  assert(code && XTestFakeKeyEvent(d, code, down, CurrentTime));
}
static void tap(Display *d, KeySym sym) { key(d, sym, True); key(d, sym, False); }

static int ignore_xerror(Display *d, XErrorEvent *e) { (void)d; (void)e; return 0; }

int main(int argc, char **argv) {
  assert(argc == 2 || argc == 4);
  Display *d = XOpenDisplay(NULL);
  assert(d);
  if (argc == 4) {
    assert(!strcmp(argv[1], "parent"));
    XSetErrorHandler(ignore_xerror);
    XWindowAttributes attrs;
    int exists = XGetWindowAttributes(d, strtoul(argv[2], NULL, 16), &attrs);
    int ok = !strcmp(argv[3], "absent") ? !exists : exists && attrs.map_state == (!strcmp(argv[3], "hidden") ? IsUnmapped : IsViewable);
    XCloseDisplay(d);
    return ok ? 0 : 1;
  }
  if (!strcmp(argv[1], "focus")) {
    Window window = find_window(d, DefaultRootWindow(d));
    assert(window);
    XSetInputFocus(d, window, RevertToParent, CurrentTime);
  } else {
    assert(!strcmp(argv[1], "keys"));
    tap(d, XK_a);
    key(d, XK_Shift_L, True); tap(d, XK_a); key(d, XK_Shift_L, False);
    key(d, XK_Control_R, True); tap(d, XK_s); key(d, XK_Control_R, False);
    unsigned num = XkbKeysymToModifiers(d, XK_Num_Lock);
    assert(num);
    assert(XkbLockModifiers(d, XkbUseCoreKbd, num, 0));
    tap(d, XK_KP_End);
    assert(XkbLockModifiers(d, XkbUseCoreKbd, num, num));
    tap(d, XK_KP_1);
    tap(d, XK_KP_Enter);
    tap(d, XK_F12);
  }
  XSync(d, False);
  XCloseDisplay(d);
  return 0;
}
