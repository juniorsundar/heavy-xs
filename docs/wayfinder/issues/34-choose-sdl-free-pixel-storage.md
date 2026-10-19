# Choose SDL-free pixel storage and presentation conversion

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-pixel-storage
Blocked by: 24, 26, 27, 32

## Question

What owned pixel representation and conversion boundary replace SDL surfaces for glyph atlases and draw targets while preserving the existing CPU renderer's pixels, geometry and lifetime behavior?

Choose between existing code/standard Rust buffers and an already-present dependency only where it earns its place. Specify dimensions, stride, channel ordering, alpha semantics, allocation/resize/drop ownership and the boundary to softbuffer's typed, potentially padded presentation rows. Cover the supported Wayland and X11 modes without assuming the same alpha mode works on both. Preserve zero-size handling, clipping, blitting and glyph-atlas behavior; this is not a rasterizer redesign or GPU rendering work.

Use [Select the Rust-native platform stack](27-select-platform-stack.md#resolution) and [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md#resolution) as constraints. The probe's scratch allocation and full-frame copy are evidence of correct conversion, not a selected production design or a demonstrated cost pass. Whole-window opacity remains a no-op; internal renderer alpha blending and the working opaque-ARGB Wayland route remain required.

Consult codebase-design when locating the storage/presentation seam. Resolve with the representation, ownership, conversion rules and test/cost implications needed by the implementation handoff. Produce a decision, not production code; any new dependency or pixel mismatch requires explicit review.

## Comments

### Live decisions — storage and frame ownership

The user chose **“Plain Rust buffers”** and **“Persistent frame plus copy.”** Preserve the existing renderer and compact coverage atlases rather than adopting tiny-skia Pixmap as the storage foundation. Each window owns a reusable draw target, separate from softbuffer's rotating presentation buffers; copying and its explicit memory cost remain subject to the existing gates. No per-frame scratch allocation is selected.

Source inspection: [renderer storage and blending](../../../src/renderer.rs) uses one-byte grayscale and three-byte subpixel coverage atlases, custom integer glyph blending, and SDL fills/blits for rectangles. [The command cache](../../../src/rencache.rs) redraws dirty regions only. [Pixel/hash test hooks](../../../src/test_hooks.rs) compare logical RGB and exclude alpha; presentation alpha therefore needs separate assertions.

The user then chose **“Full copy first.”** Initially copy every visible row of the retained draw target into each acquired presentation buffer; do not add buffer-age/damage-history state. Skip presentation for unchanged cache frames in normal operation, but retain OS-driven refresh/restore behavior. Any copying optimization must follow evidence from the existing gates, not a waiver of them. The user subsequently confirmed the complete proposal below.

### Resolution

The user chose **“Plain Rust buffers”**, **“Persistent frame plus copy”** and **“Full copy first”**, then **“Confirm storage design”** for the complete layout, lifecycle and validation proposal. This resolves storage and presentation conversion for planning, not implementation or a performance pass.

#### Representation and ownership

- **Coverage atlases:** font-owned `Vec<u8>` storage, zero-initialized, with width/height and byte stride. Grayscale rows contain one coverage byte per pixel; subpixel rows contain three coverage bytes in the existing RGB order. Pack rows as width times channel count; preserve the existing atlas bump placement, glyph metrics, subpixel cache and font teardown. Do not inflate coverage to four-channel color images or introduce a new packing algorithm.
- **Draw target:** one window-owned, reusable, row-major `Vec<u32>` with physical width/height and clip state; row stride is width pixels / `width * 4` bytes. The numeric packing is **`0xFFRRGGBB`**. Initialize to opaque black; keep the high byte **`0xFF` after every drawing operation**. This numeric convention is not a license to assume host byte order when copying into another type.
- **Drawing:** retain the existing glyph arithmetic, geometry, clipping, command-cache behavior and rectangle RGB results. Input alpha and glyph coverage affect RGB through compositing into the opaque draw target; they are not window transparency. Replace SDL map/fill/blit/clip operations with equivalent Rust operations, including their observed rounding. Do not normalize blend arithmetic, change `RenColor` command layout/hashing or silently update expected images. tiny-skia remains available for its existing font work, not as a new draw-target/compositing engine.
- **Lifetime:** allocate on creation or physical-extent change and reuse between frames, with no per-frame scratch image. Validate extents, checked size/stride calculations and indexing before allocation/access, preserving existing Lua-visible error behavior. On recreation, initialize the new target, release old storage, reset clipping and invalidate the drawing cache before presentation. Drop frame and presentation resources with their window, and coverage storage with its font. Retain the target while minimized, but do not configure, acquire or present a zero-sized surface. Restore/recreation must arrange a valid full refresh.

#### Presentation seam and backend modes

Keep CPU storage/drawing independent of a live display. The window module owns the draw target and softbuffer surface; conversion belongs at the existing window-presentation seam (`ren_update_rects` / `update_rects`), not in font rasterization or command hashing. The presenter borrows completed rows for the duration of the copy; do not retain borrowed softbuffer pixels across presents, resizes or teardown. Adapt internal `RenSurface` users without changing the public Lua interface. No generic framebuffer/backend framework or SDL-surface compatibility shim is selected.

At each required presentation, acquire a softbuffer buffer and copy **all visible pixels row by row** using `pixel_rows()` and typed `Pixel` channel construction/access. Respect the returned row stride; do not zip a packed source against the entire padded destination or assume the numeric `u32` layout can be reinterpreted as `Pixel`. Write unchanged RGB and alpha `0xFF`:

- **Wayland:** the accepted pinned softbuffer **`AlphaMode::Premultiplied`**, yielding ARGB8888. Opaque alpha makes premultiplication the identity; do not switch back to the failing XRGB route.
- **X11:** **`AlphaMode::Opaque`**, supported at the accepted pin and requiring alpha `0xFF`. This is an interface-supported planning choice, not live proof that the upgraded X11/editor path works. Preserve the existing backend-validation dispositions and capability/error checks; do not apply Wayland's premultiplied mode unconditionally.

Retain dirty-region **drawing**. Valid dirty rectangles can remain presentation-damage hints, but must not limit this full copy. Use full damage after recreation or a forced full refresh. Normal unchanged cache frames do not copy or present; explicit OS foreground/refresh/restore requirements can still re-present the retained image, as the existing foreground-refresh path does. Buffer contents/age may be arbitrary: no buffer-age or damage-history optimization is selected initially.

#### Required checks, costs and stop conditions

Carry these into [Design SDL-free platform contract testing](36-design-sdl-free-contract-testing.md) and [Define the SDL3 replacement handoff](28-plan-sdl3-migration.md):

- Retain renderer/rencache contract outputs, text geometry and RGB hashes. Test grayscale/subpixel coverage, negative/empty/out-of-range clipping and rectangle blends including alpha boundaries/repeated overlap. Pin the observed SDL rectangle-rounding results before replacement; a naive idealized blend is not sufficient evidence of parity.
- Add direct assertions for packed channel order and `0xFF` alpha after initialization, glyph/rectangle drawing and presentation conversion: existing RGB-only pixel/hash hooks cannot establish this. Test odd-width destination rows with padding sentinels and distinct channel values to catch row skew and red/blue swaps.
- Simulate rotating presentation buffers with unrelated prior contents and only a small changed drawing region: every visible destination pixel must still match the retained image. Cover unchanged-frame skipping separately from forced refresh, resize/invalidation, minimize/restore and owner teardown. No new live launch is authorized by these requirements.
- Account for the retained frame (four bytes per physical pixel, about 20 MiB at 2543×2076), actual allocated capacity, backend buffers and transient resize allocations. Measure copy/conversion within the presentation phase and report it separately without removing it from comparable presentation totals. Preserve the [calibrated frame, startup, idle and memory gates](31-calibrate-platform-cost-gates.md#resolution); repeated resize/create/drop checks must detect accumulation rather than assume Rust ownership proves its absence.
- A pixel/geometry mismatch, allocation/lifetime defect or failed cost gate is not accepted. Stop and report evidence; propose a bounded copy/buffer-age optimization or design revision for review if needed. Do not silently add history state, relax gates, introduce a dependency, change expected images or recover whole-window opacity.

#### Evidence and touch scope

Source inspection covered [renderer atlases, glyph blends and rectangle operations](../../../src/renderer.rs), [window ownership, clipping, resize, presentation and foreground refresh](../../../src/renwindow.rs), [dirty-region replay](../../../src/rencache.rs), [font raster output](../../../src/font.rs), [drawing-interface scale use](../../../src/renderer_api.rs), and [test-only pixel access](../../../src/test_hooks.rs). The downstream implementation must adapt these users plus [renderer contracts](../../../tests/contract/cases/renderer.lua), [cache contracts](../../../tests/contract/cases/rencache.lua), and the harness/build steps specified by their own decisions; do not treat replacing only surface allocation as complete.

Pinned-source checks: [typed pixel/channel representation](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/pixel.rs), [buffer lifetime/rows/age and alpha modes](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/lib.rs), and [X11 alpha capabilities](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/backends/x11.rs). The unreleased-pin and existing desktop-validation limits remain those of the [presentation qualification](32-qualify-wayland-presentation.md#resolution) and [stack selection](27-select-platform-stack.md#resolution). No production files, dependencies, desktop state or acceptance results are changed by this resolution.
