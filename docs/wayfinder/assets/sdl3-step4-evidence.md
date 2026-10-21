# SDL3 migration Step 4 evidence

**Status: in progress. Only the dependency/build checkpoint is complete.** Windows, drawing, input and services still use SDL. This is not evidence of a completed native cutover or permission to waive its acceptance gates.

## Dependency/build checkpoint

Based on `0e4247cc70e51a8c09bf0eb012f446bf500b1db4` on `sdl3/step4-native-cutover`, following the [implementation handoff](sdl3-migration-handoff.md#step-4--complete-native-platform-storage-and-service-cutover).

- Added the selected exact winit, arboard, ashpd and Tokio versions/features. Softbuffer uses upstream git revision `9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`, not released 0.4.8. Tokio includes the selected signal/sync features; no runtime or listener is initialized by these dependency declarations.
- Cargo.lock contains 277 packages (previously 64); every previous name/version pair remains. The added packages include target-specific dependencies which are not all compiled on Linux. Lock SHA-256: `16c7a91895650e7d38c4302457580a6fb79fe8659b89f499f50d08fc5e4c830f`.
- Computed the git-source hash with `fetchgit` from the flake's pinned nixpkgs, using the same URL/revision/defaults as its `importCargoLock`: `sha256-X8gdXyRoFeGo1RhYarenBmW+c11xgu9NuQVo/UmpOmw=`. The subsequent package build successfully consumed it through `cargoLock.outputHashes."softbuffer-0.4.8"`.
- Kept SDL and all existing SDL build inputs. Explicitly named native dlopen providers alongside them; the installed ELF gains their RPATH after Nix's shrink-rpath phase. No wrapper or executable rename. Private D-Bus tooling is in the dev shell only.

### Checks performed

| Check | Result |
| --- | --- |
| `env -u LD_LIBRARY_PATH cargo check --offline` (lock resolution), followed by locked builds below | Pass |
| `nix build --no-link .#lite-xl` | Pass: x86_64 release, ordinary configuration, git dependency vendored through pinned nixpkgs |
| `PROFILE=xvfb scripts/rust-port/verify.sh` | All 18 suites pass; windowed suites report X11 |
| `PROFILE=offscreen scripts/rust-port/verify.sh` | All 18 suites pass; windowed suites report offscreen |
| `scripts/rust-port/startup-check.sh`, dummy and private Xvfb separately | 12/12 each, including isolated child SIGINT, restart and missing-display failure diagnostics |
| `scripts/rust-port/normal-build-check.sh` | 4/4: no hook global/module or strings, retired switch inert, 41 system exports |
| `scripts/rust-port/hit-test-check.sh` | 1,239,848-point recorded C grid agrees |
| `env -u LD_LIBRARY_PATH cargo test --offline --locked --lib` | 17 pass |
| `nix eval --offline --raw .#packages.aarch64-linux.lite-xl.pname` | Evaluates; **not** an aarch64 build |
| Installed ELF RPATH load probe, `env -i` | All nine sonames below load with `RTLD_NOW` |
| `git diff --check` | Pass |
| Read-only `zai/glm-5.3-flash` review of Cargo/Nix diff | No actionable findings |

The isolated C load probe used precisely the installed ELF's RPATH and no environment, opening `libwayland-client.so.0`, `libwayland-cursor.so.0`, `libxkbcommon.so.0`, `libxkbcommon-x11.so.0`, `libX11.so.6`, `libX11-xcb.so.1`, `libxcb.so.1`, `libXcursor.so.1` and `libXi.so.6`. It establishes library resolution only: it creates no display connection, clipboard or native event loop. The SDL contract passes do not establish native-crate runtime behavior.

Built artifact: `/nix/store/aiws994dry2x3rnxkc6zdr6ch8bdn6q5-lite-xl-2.1.7/bin/lite-xl`, SHA-256 `435c3dc23db818e558a99a042cec108dffea4182e8551e5f44b975d7512548db`. Hook-enabled contract binary SHA-256 `4f6a51ecfae4444a8af0386c38c90322791fb1bc5d1fc5281d34c28320270429`. These identify this uncommitted-checkpoint build, not a later rebuilt binary after a commit changes build metadata.

Local, noncanonical logs/provenance: `target/step4-deps/`, `target/step4-deps-*.log`, `target/step4-nix-build.log`, `target/step4-softbuffer-hash.log`. The installed closure contains no Xvfb/xauth/compiler test tools. It still contains SDL's transitive `dbus-1.16.2-lib`; that is distinct from adding private-bus test executables to runtime dependencies.

### Remaining work and limits

The coupled main-thread platform owner, persistent draw target and coverage atlases, native event conversion/waits, clipboard, generation-fenced dialogs, process-signal integration, sandbox/environment preservation and fatal notification remain to implement and validate. The planner's proposed temporary SDL-surface bridge is rejected by the existing pixel-storage resolution. Signal ownership must move with the pump, not as an independently operating or duplicate signal producer.

No live desktop, native Wayland/XWayland presentation/input, IME, selection, chooser, notification or performance check was performed here. Native X11 retains its accepted-unverified disposition; aarch64 was evaluated only. Library resolution and passing SDL tests do not qualify the replacement. Preserve all existing stop conditions, including an observed non-excepted IME/input difference. SDL removal and the final installed-artifact dynamic proof belong to Step 5, not this checkpoint.
