# Select the Rust-native platform stack

Parent: ../sdl3-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-platform-stack
Blocked by: 25, 26, 29, 30, 32, 33

## Question

Which crates replace which SDL areas, which SDL areas (if any) stay and why, and which gaps and differences does the user accept?

Context: [Survey Rust-native platform crates](25-survey-rust-platform-crates.md), [Define platform compatibility and acceptance criteria](26-define-platform-acceptance.md). The [window, scale and IME prototype](29-prototype-window-scale-ime.md) and the [loop, clipboard and dialogs prototype](30-prototype-loop-and-dialogs.md) supply the runtime evidence. The first prototype closed with qualified success, not a complete acceptance pass: Consult [Qualify Wayland CPU presentation without unsupported pixel padding](32-qualify-wayland-presentation.md) for presentation qualification and [Complete the outstanding window and input desktop checks](33-complete-platform-desktop-checks.md#resolution) for desktop-check evidence and approved validation dispositions; unresolved dependencies remain blocking. Carry their results and any explicitly approved gate revisions into the migration handoff; prototype closure alone accepts no workaround or missing check. The [presentation resolution](32-qualify-wayland-presentation.md#resolution) accepts an exact unreleased upstream pin and ARGB route for planning only; preserve its limits rather than treating the API upgrade as already validated for other backends, input or full-editor costs.

## Comments

### Live decisions — opacity and dialogs (initial round)

The user initially chose **“Preserve functional opacity”** and **“Use ashpd with that label limit.”** The opacity preference is superseded by **Opacity scope decision** below; the dialog choice stands. These answers did not select the whole stack:

- **Opacity must remain functional wherever the baseline supports it.** Preserve the Lua signature and its existing always-true return quirk; a successful no-op is not an accepted substitute for the actual translucency effect. Determine a feasible replacement before resolving stack selection. SDL 3.4.14's Wayland backend has a compositor-conditional alpha-modifier implementation, so the lack of a winit setter does not establish that there is no existing Wayland behavior to preserve.
- **Use ashpd for dialogs**, with the live-tested 0.13.13 API as the planning reference. Preserve distinct accept/cancel/error outcomes, glob filters and parent modality; use an async runtime that lives for the process. The user explicitly accepts the **portal-controlled Cancel label** rather than a custom cancel label; custom Accept text remains supported. No rfd/Zenity fallback is selected.

The lifetime runtime's integration and the remaining window/platform choices are not silently settled by these answers. No production dependency, implementation, feature loss beyond the stated Cancel-label difference, or acceptance-gate waiver is authorized. The ticket remains claimed while opacity feasibility and the remaining choices are investigated.

### Live decision — loop model; opacity requirement reopened

The user chose **“Keep poll/wait and pump.”** Preserve the existing Lua poll/wait model through winit's pump API, with `EventLoopProxy` wake-ups and a process-lifetime async runtime for ashpd. Runtime ownership, integration and cost validation remain to be planned; no callback-driven Lua-loop redesign is selected.

For opacity the user chose **“Revisit the opacity requirement”** and asked **“What is the mandatory case for functional opacity?”** The proposed blocking opacity prototype is therefore **not authorized or created**. Explain the distinction between preserving a public plugin API and a built-in editing requirement before asking for a final disposition. No bundled `data/` caller was found in the source inspection; this does not prove external plugins/user configuration never use the API. At this point the earlier preservation preference was under reconsideration, not silently reversed into an accepted no-op.

### Opacity scope decision

The user explicitly instructed: **“Drop the opacity config and set it as a no-op”**, rejected further investigation as a rabbit hole, and left possible recovery for a later effort. This **supersedes the initial functional-opacity requirement**.

For the planned replacement, drop functional **whole-window translucency** and do not carry forward configuration/UI support for that effect. Retain `system.set_window_opacity(window, opacity)` as an intentional compatibility **no-op**, preserving its existing argument handling and always-`true` return. Calls from external plugins or user configuration will no longer change the window's opacity; document this explicit compatibility loss. No new range validation or return-value correction is authorized.

Stop opacity adapter research and do not create an opacity prototype, protocol shim, SDL fallback or functional-opacity acceptance gate. Existing Lua argument/return contract coverage remains relevant. This decision does **not** drop internal renderer alpha blending, text antialiasing or the required ARGB8888/opaque-alpha presentation route. Recovery of whole-window opacity is **out of scope for this migration**, not unfinished fog, a blocker or a promised follow-up.

This is a planning decision; the current SDL implementation and configuration are not changed here. The user subsequently confirmed the complete proposal, recorded below.

### Resolution

The user chose **“Confirm planning stack”** after reviewing the crate choices, loop model, borderless behavior and the two explicit compatibility differences. This selects the platform direction for the migration plan; implementation and release acceptance remain separate.

| SDL responsibility | Selected replacement / disposition |
|---|---|
| Windows and OS input/events | **winit 0.30.13**, retaining Wayland and X11 support |
| CPU presentation | **softbuffer git `9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`**, the explicitly accepted unreleased pin; opaque-alpha ARGB8888 on Wayland |
| Clipboard and primary selection | **arboard 3.7.0** |
| Open/save/directory dialogs | **ashpd 0.13.13**, with a process-lifetime async runtime; no rfd/Zenity fallback |
| Poll/wait and worker/dialog wake-ups | Existing Lua poll/wait model through **winit's pump API**, using **EventLoopProxy** for completions |
| Borderless regions and controls | Retain the Lua hit-test interface/quirks and title controls; use application-side zones with winit's native drag/resize operations rather than requiring SDL's per-point callback |
| Whole-window opacity | **Compatibility no-op**, retaining `system.set_window_opacity` argument handling and its existing `true` result; no functioning opacity configuration |
| Plugin loading, timing/sleep, filesystem and allocations | Retain the map's charting decisions: `libloading`, `std`/Rust ownership, including existing Lua quirks and error wording |
| SDL surfaces and test-driver facilities | Must move off SDL; representation and test seams remain explicit follow-up decisions, not selected designs |

**Accepted compatibility differences in this decision:** whole-window opacity no longer changes translucency, and dialogs use the portal-controlled Cancel label instead of a custom Cancel label. The former supersedes the initial opacity-preservation preference. It does not remove internal drawing alpha or the required opaque-ARGB presentation path. Recovering window opacity is outside this effort; no adapter, prototype or feature-recovery task is authorized. Preserve custom Accept text, dialog outcome distinctions, glob filters and parent modality. Replacing the borderless callback with app-side zones is an approved mechanism change, not permission to lose region/coordinate quirks or title-control behavior; newly discovered behavioral differences still require review under the existing validation dispositions.

**SDL disposition:** no permanent SDL area or hidden SDL fallback is selected. SDL may coexist during the incremental migration until the unresolved pixel-store, runtime and test decisions are implemented and validated. The separately approved retirement of `sdl_renderer` remains unchanged; this selection does not add another rendering mode.

**Limits and runtime requirements:** these versions are planning references to the tested candidates, not a claim that the combined editor passes acceptance. The softbuffer pin is not released 0.4.8 despite its package version string. The pinned [X11 backend supports only Opaque/Ignored alpha modes](https://github.com/rust-windowing/softbuffer/blob/9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70/src/backends/x11.rs#L314-L316), not Premultiplied; revalidate its supported path under the existing dispositions rather than configuring the Wayland ARGB mode unconditionally. ashpd requires a working desktop portal/backend and a process-lifetime runtime; carry availability, failure reporting and packaging into the handoff rather than silently choosing another backend. The runtime's concrete ownership/integration remains a follow-up decision.

All remaining Lua-contract, drawing, real-OS-path and numerical cost gates retain their agreed dispositions. The user-approved opacity effect and Cancel-label differences are the exceptions, not a blanket waiver. Deferred/conditional checks and native X11's accepted-unverified status must not become passes or new planning blockers. The prototype's scratch copy is not a production pixel-store or performance choice.

**Remaining decisions:** [Choose SDL-free pixel storage and presentation conversion](34-choose-sdl-free-pixel-storage.md), [Define the pump-loop and dialog-runtime boundary](35-define-pump-and-dialog-runtime-boundary.md), and [Design SDL-free platform contract testing](36-design-sdl-free-contract-testing.md). Their answers feed [Define the SDL3 replacement handoff](28-plan-sdl3-migration.md), which owns incremental ordering and build changes. This resolution authorizes no production code, dependency installation, push, upstream post or desktop action.
