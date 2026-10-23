/* Evidence helper: private Xvfb only, not a host-desktop input driver. */
#include <X11/Xlib.h>
#include <X11/keysym.h>
#include <X11/extensions/XTest.h>
#include <assert.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
  assert(argc == 2);
  const char *expected = getenv("GUI_BENCH_DISPLAY"), *actual = getenv("DISPLAY");
  assert(expected && actual && !strcmp(expected, actual));
  Display *d = XOpenDisplay(NULL); assert(d);
  Window root = DefaultRootWindow(d), tree, parent, *children = NULL, win = 0;
  unsigned n = 0; assert(XQueryTree(d,root,&tree,&parent,&children,&n));
  for (unsigned i=0;i<n;i++) {
    XWindowAttributes a;
    if (XGetWindowAttributes(d,children[i],&a) && a.map_state == IsViewable) { win=children[i]; break; }
  }
  if (children) XFree(children);
  assert(win);
  if (!strcmp(argv[1],"focus")) XSetInputFocus(d,win,RevertToParent,CurrentTime);
  else if (!strcmp(argv[1],"blur")) XSetInputFocus(d,root,RevertToParent,CurrentTime);
  else {
    assert(!strcmp(argv[1],"tap"));
    KeyCode code=XKeysymToKeycode(d,XK_F12); assert(code);
    assert(XTestFakeKeyEvent(d,code,True,CurrentTime));
    assert(XTestFakeKeyEvent(d,code,False,CurrentTime));
  }
  XSync(d,False);
  if (strcmp(argv[1],"tap")) {
    Window focused; int revert;
    XGetInputFocus(d,&focused,&revert);
    assert(focused == (!strcmp(argv[1],"focus") ? win : root));
  }
  XCloseDisplay(d); return 0;
}
