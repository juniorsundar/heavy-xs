# SDL3 replacement, Step 2 evidence

Evidence for [Step 2 — Independent non-window ports](sdl3-migration-handoff.md#step-2--independent-non-window-ports), collected on 2026-10-21 on Linux (NixOS), with no live desktop interaction. SDL windows, events, surfaces, clipboard, dialogs, `setenv`/sandbox and font error transport are unchanged, as the step requires.

**Status: implemented.** It deviates from the handoff once, on the clock, by the user's decision (below).

## What moved off SDL

| Function | Before | After |
| --- | --- | --- |
| `system.get_time` | `SDL_GetPerformanceCounter / Frequency` | `rustix::time::clock_gettime(MonotonicRaw)` with the same arithmetic: u64 nanoseconds to f64, divided by 1e9 |
| `system.sleep` | `SDL_Delay((n * 1000.0) as u32)` | The same clamp and `as u32` conversion, then `std::thread::sleep`. For 0 ms, a zero-length `rustix::thread::nanosleep`, matching the `clock_nanosleep(0)` SDL issues (verified with strace). |
| `system.list_dir` | `SDL_EnumerateDirectory` | `std::fs::read_dir` on the raw path bytes, in readdir order, without `.`/`..`. A mid-listing read error ends the list, as SDL's loop did. Message: `Can't open directory: <strerror>`. |
| `system.rmdir` | `SDL_RemovePath` | C `remove` semantics in std: `remove_file`, then `remove_dir` on `IsADirectory`. `NotFound` returns `true`. Message: `Can't remove path: <strerror>`. |
| `system.load_native_plugin` and `__gc` | `SDL_LoadObject` / `LoadFunction` / `UnloadObject` | `libloading =0.8.9` `os::unix::Library::open(path, RTLD_NOW \| RTLD_LOCAL)`. The handle stays a light userdata owned by the `NativePlugin` table and is released once by `__gc` (NULL ignored). Symbol lookup retries with a leading `_`, as SDL did. Message: `Failed loading <path>: <dlerror>`, byte-exact. |
| Command buffer | `SDL_realloc` / `SDL_free` | `std::alloc` with 16-byte (`max_align_t`) alignment. One `free_command_buffer` frees it, sized from `command_buf_size`. Layout, padding, hashing and replay are unchanged. |

`strerror` text comes from std's `io::Error` display with its ` (os error N)` suffix removed. libloading keeps `dlerror` private apart from an `escape_ascii` debug form, so `dlerror_bytes` reverses that form to keep non-UTF-8 paths intact.

Nine SDL imports are gone from both the normal and the contract binary: `SDL_Delay`, `SDL_EnumerateDirectory`, `SDL_GetPerformanceCounter`, `SDL_GetPerformanceFrequency`, `SDL_LoadFunction`, `SDL_LoadObject`, `SDL_realloc`, `SDL_RemovePath`, `SDL_UnloadObject`. The reference still imports all nine.

New dependencies: `libloading =0.8.9`, as the handoff specified, and `rustix =1.1.5` with the `time` and `thread` features (the version winit 0.30.13 locks). On Linux, rustix compiles only `bitflags` and `linux-raw-sys`. The libc and errno entries in Cargo.lock are for other targets.

## Deviation: the clock

The handoff asked for a process-lived `std` monotonic anchor. That would start `get_time` near 0 in every process. `core/init.lua:458` derives each run's temp-file prefix from `get_time() * 1000`, so concurrent editors could share a prefix, and `core.delete_temp_files` / `core.temp_filename` would touch each other's files. SDL's counter is CLOCK_MONOTONIC_RAW in nanoseconds since boot (measured). The user rejected a libc implementation and asked for a Rust-native one. `rustix` reads the same clock with no libc, so values keep SDL's origin and rate: the reference and candidate both read about 16790 s in the same moment. Restart cannot reset it, and the conversion has no new panics.

## New goldens, recorded from the SDL reference

The existing goldens reduce these functions' failure messages to their type. Before porting, two suites were recorded from the pinned reference (`c80459bc`, built by `sdl-reference.sh`). Each recording was taken twice and both runs were identical. Recording went through `RECORD=1`, which only accepts passing runs.

- `sysfs` (`tests/contract/cases/sysfs.lua`): every `list_dir`/`rmdir` failure message (missing, not a directory, empty string, unreadable, through a file, dangling link, link loop, name too long, `EINVAL` on `.`, read-only parent), non-UTF-8 names listed and removed by their bytes, NUL truncation, readdir order, and symlink/fifo/trailing-slash removal (links removed, targets kept).
- `sysloader` (`tests/contract/cases/sysloader.lua`, new fixtures `underplug`, `undefplug`, `provplug`, `consplug`): load-failure messages, including a non-UTF-8 path byte for byte; `RTLD_NOW` (an undefined symbol fails the load) and `RTLD_LOCAL` (a second plugin cannot bind to the first's symbols); the `_`-prefixed entrypoint fallback; the empty path, which opens the main program; and lifetime read from `/proc/self/maps`: one copy is unmapped when its owner is collected, a second copy keeps the file mapped, and the file is unmapped once both are collected.

Recorded quirks, kept rather than fixed: `rmdir` returns `true` for a missing path, a missing parent and `""`. A plain entrypoint that pushes nothing returns the path argument.

## Results

| Check | Result |
| --- | --- |
| 18 suites, offscreen | all match (16 historical goldens plus `sysfs`, `sysloader`) |
| 16 suites, private Xvfb | byte-identical to the reference's Xvfb outputs. The 5 profile differences from Step 1 are still awaiting review. |
| Startup checks, dummy and Xvfb | 12/12 each |
| Hit-test | Rust grid matches the recorded C grid |
| Normal build | no hooks; 41 `system` exports |
| Unit tests | 17 pass, including the new command-buffer tests. Growth from 512 KiB by ×1.2 past 64 MiB keeps every byte and the 16-byte alignment, and teardown frees exactly once. A failed allocation keeps the old buffer. |
| Builds | No warnings in the default, `test_hooks` and `sdl_renderer` configurations. |

## Not covered

- No live desktop run. Default SDL windows and presentation were exercised only through the window suites on offscreen and private Xvfb.
- Allocations: the command buffer's growth policy and sizes are unchanged and covered by the unit tests. No heap profile was taken, and no cost change is claimed.
- `sysloader` assumes glibc's `dlerror` wording, and both permission cases in `sysfs` assume a non-root user.
