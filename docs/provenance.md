# Project provenance

Heavy XS began as a fork of [Lite XL](https://github.com/lite-xl/lite-xl), itself
derived from [rxi/lite](https://github.com/rxi/lite). It is now developed
independently. The Lua editor layer and interfaces carry forward work by those
projects and their contributors; the native core has since been ported to Rust.

## Fork point

The last upstream commit before Heavy XS development was:

- [`234fa0998016f3da9eb9ceaf3c4bc79b93462276`](https://github.com/lite-xl/lite-xl/commit/234fa0998016f3da9eb9ceaf3c4bc79b93462276)
- `fix(treeview): make delete and rename commands available only when the view is active (#2211)`
- March 11, 2026, authored by Lukasz Pelszynski

An earlier cleanup of this repository's history changed that commit's local ID
to `5997dde0e75e5c92d10fc97c4d71958b01abe2c0` without changing its file tree. The
condensed history represents that tree as a single **Import Lite XL baseline**
commit, followed by the independent development commits. The first of those is
`feat(rust): bootstrap Rust core as a staticlib linked into the Meson build` by
Junior Sundar.

The baseline condenses 2,489 inherited commits. For the full upstream record, see
the [source at the fork point](https://github.com/lite-xl/lite-xl/tree/234fa0998016f3da9eb9ceaf3c4bc79b93462276),
its [commit history](https://github.com/lite-xl/lite-xl/commits/234fa0998016f3da9eb9ceaf3c4bc79b93462276/)
and the [inherited changelog](https://github.com/lite-xl/lite-xl/blob/234fa0998016f3da9eb9ceaf3c4bc79b93462276/changelog.md).
The local [changelog](../changelog.md) covers Heavy XS development instead.

## Attribution and licensing

Credit for the inherited work remains with rxi, Francesco Abbate, the Lite XL
Team and the other upstream contributors. Condensing Git history does not change
the authorship or licensing of that work.

The original copyright and MIT permission notices remain in [LICENSE](../LICENSE).
Bundled fonts and other components retain their own notices in
[licenses/](../licenses/). Existing source-level notices are preserved.

## Remaining upstream names

The executable (`lite-xl`), Cargo crate, installation and configuration paths,
Lua APIs and some assets retain inherited names. These names do not indicate
that this is an official Lite XL release. Renaming them is separate from
condensing the project's history and documentation.
