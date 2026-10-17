# Resources

This folder contains resources used for building or packaging the project.

### Packaging

- `icons/icon.inl`: the 64x64 RGBA window icon as a C byte array; `rust/build.rs` extracts it.
- `icons/lite-xl.svg`: scalable icon installed by `scripts/install-linux.sh`.
- `linux/com.lite_xl.LiteXL.desktop`: Desktop file for Linux desktops.
- `linux/com.lite_xl.LiteXL.metainfo.xml.in`: AppStream metadata template.

### Development

- `include/lite_xl_plugin_api.h`: Native plugin API header. See the contents of `lite_xl_plugin_api.h` for more details.
