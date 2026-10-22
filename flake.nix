{
  description = "Lite XL: lightweight, extendable text editor (dev shell + package)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      # Providers loaded by winit/softbuffer/arboard, retained outside the dev shell.
      platformDeps = pkgs: with pkgs; [
        wayland
        libxkbcommon
        libx11
        libxcb
        libxcursor
        libxi
      ];

      # SDL3 and the libraries behind its pkg-config Requires.private list.
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
        let pkgs = nixpkgs.legacyPackages.${system}; in rec {
          # Cargo builds Lua and the Rust backends from vendored, locked sources;
          # SDL3 stays linked until the final packaging step.
          lite-xl = pkgs.rustPlatform.buildRustPackage rec {
            pname = "lite-xl";
            version = "2.1.7";
            src = self;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes."softbuffer-0.4.8" = "sha256-X8gdXyRoFeGo1RhYarenBmW+c11xgu9NuQVo/UmpOmw=";
            };
            # The contract suites need a display driver and the staged data dir;
            # scripts/rust-port/verify.sh runs them outside the sandbox.
            doCheck = false;

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = pkgs.lib.unique (sdlDeps pkgs ++ platformDeps pkgs);

            # Dlopen-only libraries would otherwise be removed by shrink-rpath.
            postFixup = ''
              patchelf --add-rpath "${pkgs.lib.makeLibraryPath (platformDeps pkgs)}" "$out/bin/lite-xl"
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
          default = lite-xl;
        });

      # Interactive shell for hacking on the source.
      #   nix develop
      #   cargo build
      #   scripts/install-linux.sh target/debug/lite-xl $(pwd)/target/stage
      #   LITE_PREFIX=$(pwd)/target/stage/usr/local target/debug/lite-xl
      devShells = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.mkShell {
            packages = pkgs.lib.unique (sdlDeps pkgs ++ platformDeps pkgs) ++ (with pkgs; [
              cargo
              rustc
              pkg-config
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

              # note: ibus is only dlopen()ed by SDL at runtime for input
              # methods; it is not needed at build time.

              # LSP servers for the editor (clangd comes with clang-tools)
              clang-tools
              lua-language-server
              # nixd: picked up from the user's system PATH when not in a
              # dev shell that provides it
            ]);

            shellHook = ''
              echo "lite-xl dev shell"
              echo "  cargo build"
              echo "  scripts/install-linux.sh target/debug/lite-xl \$(pwd)/target/stage"
              echo "  LITE_PREFIX=\$(pwd)/target/stage/usr/local target/debug/lite-xl"
              echo "  scripts/rust-port/verify.sh                   # build + contract suites"
            '';
          };
        });
    };
}