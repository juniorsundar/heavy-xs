# SDL3 Step 5 final artifact evidence

Proof for Step 5 of the [implementation handoff](sdl3-migration-handoff.md#step-5--remove-sdl-from-buildpackage-and-prove-the-final-artifact), 2026-10-23. SDL is removed from the editor's build, package and development shell; `nix develop .#reference` keeps it only for the pinned reference tooling.

## Identities

- Source `40be2737` (branch `sdl3/step4-native-cutover`), `Cargo.lock` SHA-256 `3ecc86193707…`, rustc 1.97.1, Nix 2.34.9.
- `nix build .#lite-xl`: `/nix/store/v28jhvspiq1pncaf7mwpbfcrwc3jxwda-lite-xl-2.1.7`, `bin/lite-xl` SHA-256 `2b7070168cd9…`. Default features: X11, native or through XWayland.
- `nix build .#lite-xl-wayland`: `/nix/store/y9di4zfx906qgzaxmw22478l36h8fhbm-lite-xl-wayland-2.1.7`, `bin/lite-xl` SHA-256 `ab5a9d437883…`. Adds `--features wayland`.

## Static review

- **Cargo:** `sdl3-sys` is gone from the manifest and the lock; it was the only lock change. No production source imports or calls SDL. The `SDL` mentions left in `src/` are comments naming the behaviour being matched.
- **Build:** `pkg-config` stays in the package's build inputs because `build.rs` asks it for the runtime library directories it writes into the runpath. `libwayland-client` is queried only for the `wayland` feature.
- **Linking:** both binaries link only `libgcc_s`, `libm`, `libc` and the loader.
- **Runpath:** both have glibc, gcc-lib, libxkbcommon, libX11, libxcb, libXcursor and libXi. The Wayland output adds `wayland-1.26.0/lib`. Neither binary contains an SDL library name, and only the Wayland output names `libwayland-client`.
- **Runtime closure:** no SDL, audio (alsa, pulse, pipewire, jack), libdecor or libusb. `wayland` is in the default closure only through nixpkgs' `libxkbcommon`; the default binary never loads it.
- **Dev shell:** the default shell has no `sdl3` (`pkg-config --exists sdl3` fails). The `reference` shell provides SDL 3.4.14.
- **Scripts:** candidate scripts no longer set SDL variables. `private-display.sh` and `verify.sh` apply SDL drivers, logging and log filters only with `PD_SDL=1`, which `sdl-reference.sh` exports.

## Automated checks

Run inside the new SDL-free default dev shell:
- `verify.sh`: all 18 contract suites, 61 OK, historical goldens unchanged.
- `normal-build-check.sh` 4/4, `startup-check.sh` 20/20, `hit-test-check.sh` 1/1, `native-check.sh` 14/14.
- `cargo test` 71 passed (9 ignored).
- All four feature builds (default, `wayland`, `test_hooks`, both) compile without warnings.

Against the package binaries themselves (byte-identical copies of each `bin/lite-xl` with its
`share/`, since the check writes test runtimes beside the data): `startup-check.sh` 20/20 for both
`lite-xl` and `lite-xl-wayland`.

## Runtime trace of the installed binaries

Each package binary ran outside the dev shell with an empty environment except:
- `HOME`, `XDG_RUNTIME_DIR` and `LITE_USERDIR` in a scratch directory;
- `DISPLAY` and `XAUTHORITY` for a private Xvfb;
- `DBUS_SESSION_BUS_ADDRESS` for a private session bus on an abstract socket, with no services.

It ran under `strace -f -e trace=openat,connect`, so the trace holds file opens and socket addresses only, no payloads. A profile script took it through:
- startup, window and drawing;
- clipboard and primary-selection round trips with nonprivate markers (both returned their markers);
- `core.open_file_dialog`, which returned `error`: no portal is provided on the bus;
- `system.show_fatal_error`, which printed its diagnostic and returned 0 values at once, with no notification service on the bus;
- a normal exit.

On the private display, which has no window manager, the profile reports the window as focused (see the bare-X11 focus finding in the [cost results](sdl3-step4-cost-results.md#behaviour-findings-during-the-run-no-code-kept)).

Results, identical for both packages:
- **No path containing `SDL` was opened or looked up.**
- **Libraries opened**, each from its own store path: libc, libm, libpthread, libgcc_s, libX11, libX11-xcb, libxcb, libxcb-xkb, libXau, libXdmcp, libXext, libXfixes, libXi, libXcursor, libXrender, libxkbcommon, libxkbcommon-x11.
- **Failed lookups** were only the loader's runpath probing for those same names; each was then opened successfully.
- **Connections:** the private X server socket and the private bus only.

## Not covered

- **The native Wayland path of `lite-xl-wayland`** was not traced: no headless Wayland compositor is available, and without one the build uses X11 as designed. Its Wayland behaviour has the live Step 4 review only, run from a Cargo build.
- **Real portal dialogs and a real notification daemon** were not exercised against the package binaries. Their transports are covered by `dialog-check.sh` on scripted private services, and the dialogs by the Step 4 desktop review.
- **Pending dispositions** for IME, non-US layouts, clipboard without a manager, native X11, touch, sandbox and cold-cache startup are unchanged.
