# Define the SDL3 replacement handoff

Parent: ../sdl3-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-migration-handoff
Blocked by: 26, 27, 31, 34, 35, 36, 37, 38

## Question

What is the incremental, separately shippable sequence of changes that removes SDL3, and what must each step pass before it is done?

For each step name the files and dependencies it changes, the build changes (Cargo.toml, flake.nix), the Lua-visible behaviour it touches, the test and real-window checks it must pass, and the stop conditions that send it back to the user. Write it for a separate implementation agent.

Carry forward [all agreed acceptance gates](26-define-platform-acceptance.md#resolution), as revised by [Complete the outstanding window and input desktop checks](33-complete-platform-desktop-checks.md#resolution), including full-editor drawing parity, actual OS event-path validation and the requirement to classify the existing process timing failure. Preserve the recorded distinctions between deferred validation, conditional requirements and accepted-unverified paths; do not turn them into passes or reinstate them as planning gates. Use the numerical tolerances from [Calibrate platform performance and memory gates](31-calibrate-platform-cost-gates.md); no all-passing claim is valid with unresolved test failures.

Carry the dependency, format/stride and integration-validation limits from [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md#resolution). Planning acceptance of the tested unreleased pin is not production adoption; the probe's scratch-buffer copy is not a production pixel-store decision. Apply the existing validation dispositions when revalidating affected paths, without silently waiving gates or reintroducing accepted-unverified checks as planning blockers.

Use the [selected platform stack and explicit compatibility differences](27-select-platform-stack.md#resolution). Carry the accepted opacity no-op and portal-controlled Cancel label into configuration/API documentation and test expectations; do not plan opacity adapters, a recovery prototype or a feature-recovery gate. Internal drawing alpha and the required ARGB presentation path are not dropped.

Incorporate [Choose SDL-free pixel storage and presentation conversion](34-choose-sdl-free-pixel-storage.md#resolution), [Define the pump-loop and dialog-runtime boundary](35-define-pump-and-dialog-runtime-boundary.md#resolution), and [Design SDL-free platform contract testing](36-design-sdl-free-contract-testing.md#resolution). Carry the storage/runtime/test resolutions' touch scopes, private-display isolation, required parity/lifecycle checks, pinned reference/baseline-review policy, normal-build exclusions and cost-accounting obligations into the relevant step. Document the runtime resolution's additional UTF-8-only clipboard exception and its explicit hidden-parent/Lua-generation completion policy; the earlier stack's two differences are not an exhaustive list of subsequent decisions. Also incorporate [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md#resolution): carry its earlier shared-runtime start, default-disposition-only ownership, process-level pending/wake rules, required failure handling and real SIGINT/SIGTERM evidence. Couple signal ownership to the native pump/queue cutover, preventing duplicate SDL/new-bridge handling; measure early-runtime costs rather than retaining the superseded always-lazy start. Incorporate [Preserve fatal-error reporting without SDL](38-choose-sdl-free-fatal-error-reporting.md#resolution): document its additional nonmodal best-effort notification exception, no-helper route, bounded IPC/diagnostic checks and lazy native-initialization/failure policy. The handoff owns concrete step/build ordering within these resolutions; do not silently replace their architectural choices while writing it.

## Comments

### Resolution

Prepared the [SDL3 replacement implementation handoff](../assets/sdl3-migration-handoff.md) from the settled decisions. This AFK task synthesizes the route; it makes no new compatibility choice and does not stand in for a human approval or authorize implementation.

The sequence is: pin/prepare the isolated reference and harness; port independent non-window facilities; retire the alternate SDL renderer build; ship one complete window/input/custom-queue/presentation/storage/service/signal cutover; then remove SDL build/package dependencies and prove the final installed artifact. The cutover retains the existing deep-module seam, not a transitional SDL-surface shim or generic platform adapter. File/build scopes, per-step interface effects, automated/live/cost checks, stop conditions and non-destructive recovery are recorded in the asset.

The handoff explicitly carries the early shared-runtime timing from [Choose process-signal quit and wake integration](37-choose-process-signal-quit-integration.md#resolution), lazy native display/bus initialization, process-owned pending quits, all four reviewed compatibility differences and the complete existing acceptance/disposition set. Native X11 stays accepted-unverified/reactive; touch stays unverified; sandbox checks stay conditional; isolated no-manager selection validation remains required. None becomes a pass or a new planning blocker.

A remaining inventory detail was traced rather than treated as permission to simplify: sandbox classification controls the system-file-picker default. Preserve the exact linked SDL 3.4.14 Linux detection priority, environment snapshot/public-setter update semantics and cached result with existing Rust/libc facilities; do not substitute a boolean or new container policy. Keep that port with setenv at the complete cutover so remaining SDL readers do not observe a broken intermediate environment model. The inventory's inverted picker explanation was corrected from the actual config expression; production behavior was not changed.

All sixteen historical suites, hit-test/startup checks, reference identities/baseline review, normal-build hook exclusions, raw-pixel assertions, isolated service tests, real child signals, actual OS-path review, numerical cost gates and dynamic no-SDL proof remain implementation obligations. The pinned reference binary is still unbuilt/uncertified here; the known process timing failure must be classified, not waived or reported as passing. Source/library inspection and the plan are not acceptance evidence.

No further planning question was exposed after these preservation/coupling checks. The map's destination is a concrete implementation/validation route, now recorded; implementation is a separate instruction. No production source, dependency, script, build configuration or baseline changed; no build, runtime test, GUI/bus/clipboard/signal action, installation, commit or push was performed by this task.
