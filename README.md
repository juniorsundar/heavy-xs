# Heavy XS

Heavy XS is a Linux text editor with a Rust core and a Lua editor layer. It began
as a fork of [Lite XL](https://github.com/lite-xl/lite-xl) and is now developed
independently. See [project provenance](docs/provenance.md) for the fork point and
upstream credits.

## Building

The editor builds on **Linux only**, with Cargo. Windows and macOS are not
supported by this project.

The executable, installation paths, configuration directory and Lua APIs still
use Lite XL names. The commands below use those existing names; this is not an
upstream Lite XL build.

### Prerequisites

- Rust (stable) and Cargo
- A C compiler for vendored Lua 5.4
- At run time, the X11 client libraries (libX11, libxcb, libXcursor, libXi) and
  libxkbcommon, plus libwayland-client for a `--features wayland` build. They are
  loaded on demand, not linked.

With Nix, `nix develop` provides these dependencies and the test tools.
`nix build .#lite-xl` builds the package with the runtime libraries on its runpath;
`nix build .#lite-xl-wayland` builds with native Wayland support.
`nix develop .#reference` adds SDL3 for the pinned reference tooling under
`scripts/rust-port`; the editor itself does not use SDL.

### Build and install

Run from the repository root, since Cargo reads `.cargo/config.toml` from the
current directory:

```sh
cargo build --release
LITE_RELEASE=1 scripts/install-linux.sh target/release/lite-xl "$(pwd)/stage"
LITE_PREFIX="$(pwd)/stage/usr/local" ./stage/usr/local/bin/lite-xl
```

`scripts/install-linux.sh BINARY [DESTDIR] [PREFIX]` installs the binary, `data/`,
the desktop file, metainfo, icon and licenses. `PREFIX` defaults to `/usr/local`.
Without `LITE_RELEASE=1`, the installed version string includes the Git commit.

The default build opens an X11 window, natively or through XWayland.
`cargo build --release --features wayland` adds the native Wayland backend, used
when `WAYLAND_DISPLAY` is set. Native Wayland currently lacks file drops and a
window-provided icon because winit 0.30.13 implements neither there. The compositor
can use the installed desktop file's icon. Both features work through XWayland.

## Configuration and plugins

Configure the editor in `~/.config/lite-xl/init.lua`. To avoid sharing settings
with an upstream Lite XL installation, launch with a separate user directory:

```sh
LITE_USERDIR="$HOME/.config/heavy-xs" LITE_PREFIX="$(pwd)/stage/usr/local" \
  ./stage/usr/local/bin/lite-xl
```

The inherited Lua plugin interfaces remain, but compatibility with upstream
plugins is not guaranteed. See the [Lua API reference](docs/README.md),
[font documentation](docs/fonts.md) and
[regex syntax and migration notes](docs/regex.md). Regex search uses fancy-regex,
not PCRE2.

## Development

Run `cargo test --locked` for the Rust tests and
`scripts/rust-port/verify.sh` for the contract suites. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and
[changelog.md](changelog.md) for the changes since the fork.

Historical plans and architecture records are described in
[docs/README.md](docs/README.md#architecture-and-planning). They are optional and
not needed to build the editor.

## License and credits

Heavy XS is distributed under the [MIT license](LICENSE). The original copyright
notices are retained. Thanks to the Lite XL contributors and to
[rxi/lite](https://github.com/rxi/lite), on which Lite XL was based.

See [project provenance](docs/provenance.md) and the
[dependency and bundled-font licenses](licenses/licenses.md).
