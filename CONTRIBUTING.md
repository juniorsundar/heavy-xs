# Contributing to Heavy XS

Heavy XS is developed independently of Lite XL. Report bugs and propose changes
in [this repository](https://github.com/juniorsundar/heavy-xs), not in the upstream
Lite XL issue tracker or chat channels.

## Development setup

Follow the [build instructions](README.md#building). This project supports Linux
and builds with Cargo; the former upstream Meson, Windows and macOS workflows do
not apply.

Use `LITE_USERDIR` to keep development settings separate from another editor
installation, as shown in the [configuration guide](README.md#configuration-and-plugins).
The executable and APIs still use Lite XL names.

## Checks

From the repository root, with the dependencies installed (or inside
`nix develop`):

```sh
cargo test --locked
scripts/rust-port/verify.sh
```

The contract runner builds and stages its own test executable and uses a private
Xvfb display. Keep the existing Lua-facing behaviour covered when changing the
Rust core. Add a regression test for fixes, and document intentional compatibility
changes in the relevant [API reference](docs/README.md).

## Pull requests

Open pull requests against this repository's default branch. Keep changes focused,
explain the problem they solve, and include the checks you ran and any known
limitations. Discuss changes to supported platforms or public interfaces before
implementing them.

For inherited code, preserve the copyright and license notices. See
[project provenance](docs/provenance.md) for the upstream baseline.
