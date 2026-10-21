# SDL3 replacement, Step 3 evidence

Evidence for [Step 3 — Retire the alternate SDL renderer build](sdl3-migration-handoff.md#step-3--retire-the-alternate-sdl-renderer-build), collected on 2026-10-21 on Linux (NixOS), with no live desktop interaction.

**Status: implemented.** The `sdl_renderer` feature is removed, not ported. `cargo build --features sdl_renderer` now fails with "the package 'lite_xl_core' does not contain this feature". `test_hooks` remains the only feature and is not on by default.

## What was removed

| File | Removed |
| --- | --- |
| `Cargo.toml` | the `sdl_renderer` feature and its comment |
| `src/renwindow.rs` | the `SDL_Renderer`/`SDL_Texture`/own-surface fields, `query_surface_scale`, `setup_renderer`, the renderer branches of init/free/get-surface/resize/scale/update/foreground-refresh, and the point-size division in `ren_get_size`. `ren_resize_window` now only redoes the scale, which is all the default build ever did. |
| `src/renderer.rs` | the per-font `scale`, the scaled raster size, the width division, `update_font_scale`, and `SDL_HINT_RENDER_DRIVER=software` |
| `src/renderer_api.rs` | the `update_font_scale` call in font retrieval and the target-window scale lookup in `font:set_size`. `ren_font_group_set_size` no longer takes the `surface_scale` argument, which was always 1 in the default build. |
| `src/font.rs` | the comment that tied the logical/raster size split to the feature |

`RenSurface.scale` stays, always 1. The draw path still multiplies by it and the test hooks read it, so removing it now would change drawing code. That storage belongs to Step 4. Opacity and internal alpha drawing are untouched.

The render-driver hint only ever reached `SDL_CreateRenderer`. On the private Xvfb, SDL's window framebuffer chose the Vulkan render backend both with the hint (pinned reference) and without it (this build). Neither binary imports any `SDL_Renderer` or `SDL_Texture` function, since the default build never compiled that path.

Historical records that mention the feature are left as history: `changelog.md`, `docs/rust-port/PLAN.md` and the wayfinder tickets. No current script or build documentation advertises it.

## Results

| Check | Result |
| --- | --- |
| Builds | ordinary (`target/normal`) and hook-enabled (`target/contract`), `--locked`, no warnings |
| 18 suites, offscreen | all match their goldens. The 16 historical outputs, including renderer/rencache text geometry, dirty rects and RGB hashes, are byte-identical to the pinned reference's. |
| 16 suites, private Xvfb | byte-identical to the reference's Xvfb outputs. The 5 profile differences from Step 1 are still awaiting review. |
| Startup checks, dummy and Xvfb | 12/12 each |
| Hit-test, normal build, unit tests | pass: Rust grid = recorded C grid; no hooks and 41 exports; 17 unit tests |

No expected image or golden was changed.
