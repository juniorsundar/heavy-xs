# Scripts

Various scripts and configurations used to build, install and test Lite XL (Linux only).

### Build and install

- **install-linux.sh**:          Installs a Cargo-built `lite-xl` with its data files.

### Testing (`rust-port/`)

- **verify.sh**:                 Builds the contract editor and diffs each suite against its golden (`PROFILE=offscreen|xvfb`).
- **startup-check.sh**:          Checks startup globals, the restart loop, exit codes and `error.txt`.
- **hit-test-check.sh**:         Compares the Rust window hit-test with the recorded C one (no SDL needed).
- **normal-build-check.sh**:     Checks that an ordinary build ships no test hooks.
- **sdl-reference.sh**:          Builds the pinned SDL reference apart and collects its evidence and SDL expectations.
- **private-display.sh**:        Sourced: the isolated environments (and private Xvfb) the checks run the editor in.
- **gen-plugin-syms.lua**:       Regenerates `src/plugin_syms.rs` from the native plugin API header.

### Utility

- **fontello-config.json**:      Used by the icons generator.
- **generate_header.sh**:        Generates a header file for native plugin API
- **keymap-generator**:          Generates a JSON file containing the keymap
