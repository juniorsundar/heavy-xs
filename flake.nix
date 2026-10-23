{
  description = "Lite XL: lightweight, extendable text editor (dev shell + package)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      # Libraries the default (X11) build loads at run time (winit/softbuffer dlopen them), kept
      # on the executable's runpath so it works outside the dev shell.
      platformDeps = pkgs: with pkgs; [
        libxkbcommon
        libx11
        libxcb
        libxcursor
        libxi
      ];

      # The native Wayland backend (`--features wayland`) additionally loads libwayland-client.
      waylandDeps = pkgs: with pkgs; [ wayland ];

      # SDL3 and the libraries behind its pkg-config Requires.private list: only for the pinned
      # SDL reference (scripts/rust-port/sdl-reference.sh), never for the editor.
      sdlDeps = pkgs: with pkgs; [
        sdl3
        alsa-lib          # alsa.pc
        jack2             # jack.pc
        pipewire          # libpipewire-0.3.pc
        pulseaudio        # libpulse.pc
        libx11       # x11.pc
        libxcb       # xcb.pc
        libxdmcp
        libxau
        xorgproto    # xproto.pc and friends
        libxext      # xext.pc
        libxcursor   # xcursor.pc
        libxi        # xi.pc
        libxfixes    # xfixes.pc
        libxrandr    # xrandr.pc
        libxscrnsaver # xscrnsaver.pc
        libxtst      # xtst.pc
        libdrm            # libdrm.pc
        libgbm            # gbm.pc
        libGL             # egl.pc
        libffi            # libffi.pc (required by wayland-client)
        wayland           # wayland-client.pc, wayland-egl.pc, wayland-cursor.pc
        libxkbcommon      # xkbcommon.pc
        libdecor          # libdecor-0.pc
        libusb1           # libusb-1.0.pc
      ];
    in
    {
      # Proper nix package. All dependencies come from the nix store and the
      # linker rpath is fixed up automatically, so the resulting binary runs
      # anywhere a nix store exists (i.e. outside of the dev shell too).
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          # Cargo builds Lua and the Rust backends from vendored, locked sources. `wayland` adds
          # the native Wayland backend; without it the editor is X11 (native or XWayland).
          editor = { pname, wayland }: let runtimeDeps = platformDeps pkgs ++ pkgs.lib.optionals wayland (waylandDeps pkgs); in
          pkgs.rustPlatform.buildRustPackage {
            inherit pname;
            version = "2.1.7";
            src = self;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes."softbuffer-0.4.8" = "sha256-X8gdXyRoFeGo1RhYarenBmW+c11xgu9NuQVo/UmpOmw=";
            };
            # The contract suites need a display driver and the staged data dir;
            # scripts/rust-port/verify.sh runs them outside the sandbox.
            doCheck = false;
            buildFeatures = pkgs.lib.optionals wayland [ "wayland" ];

            # build.rs asks pkg-config where the run-time libraries live, for the runpath.
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = runtimeDeps;

            # Dlopen-only libraries would otherwise be removed by shrink-rpath.
            postFixup = ''
              patchelf --add-rpath "${pkgs.lib.makeLibraryPath runtimeDeps}" "$out/bin/lite-xl"
            '';

            # Data files, desktop entry etc. live outside the crate: same script as the dev flow.
            installPhase = ''
              runHook preInstall
              LITE_RELEASE=1 sh scripts/install-linux.sh \
                "$(ls target/*/release/lite-xl target/release/lite-xl 2>/dev/null | head -1)" "" "$out"
              runHook postInstall
            '';

            meta = with pkgs.lib; {
              description = "Lightweight, extensible text editor";
              homepage = "https://lite-xl.com";
              license = licenses.mit;
              mainProgram = "lite-xl";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        in rec {
          lite-xl = editor { pname = "lite-xl"; wayland = false; };
          lite-xl-wayland = editor { pname = "lite-xl-wayland"; wayland = true; };
          default = lite-xl;
        });

      # Interactive shell for hacking on the source.
      #   nix develop
      #   cargo build
      #   scripts/install-linux.sh target/debug/lite-xl $(pwd)/target/stage
      #   LITE_PREFIX=$(pwd)/target/stage/usr/local target/debug/lite-xl
      # `nix develop .#reference` adds SDL3 for scripts/rust-port/sdl-reference.sh and
      # HIT_TEST_REFERENCE=c only; the editor itself never uses it.
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          shell = extra: pkgs.mkShell {
            packages = pkgs.lib.unique (platformDeps pkgs ++ waylandDeps pkgs ++ extra) ++ (with pkgs; [
              cargo
              rustc
              stdenv.cc # vendored Lua and the contract plugin fixtures
              git # build.rs embeds the commit in debug builds, optional
              lua5_4 # scripts/rust-port/gen-plugin-syms.lua

              # Test tools only, never runtime dependencies: the private X display that
              # scripts/rust-port/private-display.sh starts for PROFILE=xvfb runs.
              xvfb
              xauth
              util-linux # mcookie
              dbus # private session bus for service contract checks
              jq # select the exact test fixture artifact from Cargo JSON
              # The XTest input drivers that native-check.sh and dialog-check.sh compile.
              pkg-config
              libx11
              libxtst
              libxext
              libxi
              libxfixes
              xorgproto

              # LSP servers for the editor (clangd comes with clang-tools)
              clang-tools
              lua-language-server
              # nixd: picked up from the user's system PATH when not in a
              # dev shell that provides it
            ]);

            shellHook = ''
              echo "lite-xl dev shell"
              echo "  cargo build                                   # X11 (add --features wayland)"
              echo "  scripts/install-linux.sh target/debug/lite-xl \$(pwd)/target/stage"
              echo "  LITE_PREFIX=\$(pwd)/target/stage/usr/local target/debug/lite-xl"
              echo "  scripts/rust-port/verify.sh                   # build + contract suites"
            '';
          };
        in {
          default = shell [ ];
          reference = shell (sdlDeps pkgs);
        });
    };
}
