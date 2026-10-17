# Scripts

Various scripts and configurations used to build, install and test Lite XL (Linux only).

### Build and install

- **install-linux.sh**:          Installs a Cargo-built `lite-xl` with its data files.

### Testing (`rust-port/`)

- **verify.sh**:                 Builds the editor and diffs each contract suite against its baseline.
- **startup-check.sh**:          Checks startup globals, the restart loop, exit codes and `error.txt`.
- **hit-test-check.sh**:         Compares the Rust window hit-test with the original C one.
- **gen-plugin-syms.lua**:       Regenerates `rust/src/plugin_syms.rs` from the native plugin API header.

### Utility

- **fontello-config.json**:      Used by the icons generator.
- **generate_header.sh**:        Generates a header file for native plugin API
- **keymap-generator**:          Generates a JSON file containing the keymap
