//! Rust port of the Lite XL C core. Each module exports `luaopen_<name>` so the
//! existing `src/api/api.c` registration table keeps working unchanged.

#[cfg(feature = "probe")]
mod probe;
