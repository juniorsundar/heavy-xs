# Sourced by run.sh and check-loop.sh. Probe-only environment; the repository flake is unchanged.
# The repository dev shell plus libdbus, which rfd's portal backend dlopens. Same pinned nixpkgs.
probe_enter_shell() {
  local script=$1; shift
  PROBE_ROOT=$root exec nix develop --impure --expr '
    let
      root = builtins.getFlake (builtins.getEnv "PROBE_ROOT");
      system = builtins.currentSystem;
      pkgs = root.inputs.nixpkgs.legacyPackages.${system};
    in root.devShells.${system}.default.overrideAttrs (old: {
      nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ pkgs.dbus ];
    })' --command env PROBE_IN_SHELL=1 bash "$script" "$@"
}
# The crates dlopen these system libraries; Nix does not put them in /usr/lib.
probe_library_path() {
  local package libdir
  for package in xkbcommon wayland-client wayland-cursor x11 xcursor xi xrandr xcb dbus-1; do
    libdir=$(pkg-config --variable=libdir "$package")
    export LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  done
}
