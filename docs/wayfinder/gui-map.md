# A fast, deeply customisable GUI without breaking Lite XL plugins

Labels: wayfinder:map
Status: open

## Destination

Agree on a GUI philosophy and concrete design direction, with compatibility, performance and customisation acceptance criteria and a separate implementation handoff. The result may be targeted improvements, a redesign, or a combination; a rewrite is not presumed.

## Notes

- Planning only. Inventory, research, measurements and disposable prototypes support decisions; this map does not authorize production implementation.
- The user considers the Rust-core migration objective reached. Keep Lua as the scripting and plugin runtime; this is a new effort, not unfinished work on the regex, font or SDL3 maps.
- Preserve the existing Lite XL Lua interfaces and user/plugin-facing behaviour. Existing contract tests and their expectations are hard acceptance gates: do not weaken, remove or rebaseline them to accommodate a redesign. Adding coverage is allowed and necessary where evidence is missing.
- Additive Lua interfaces may expose more internals and enable deeper customisation, while preserving old interfaces and behaviour. Newly exposed internals are experimental and may be modified or reverted freely as long as existing exposure is not degraded; they do not automatically become permanent obligations. What to expose and how it works remain design questions.
- Passing the current tests is not proof of compatibility with every plugin. Use the resolved GUI inventory's evidence on legacy Lua usage, including subclassing, method overrides and the distinction between native checks and composed-GUI assertions, before defining additional gates. Undocumented use is not automatically disposable.
- Prioritise startup time and interactive responsiveness. Also evaluate idle CPU, memory/retention and how performance changes under substantial customisation. Set workloads and numerical criteria from evidence, not from unrelated previous maps.
- Keep live physical/visual checks exceptional, short and human-present. The user does not want a workflow requiring long AFK desktop benchmark sessions, now or for routine future iteration. Prefer unattended private-display automation; agree any live desktop check separately. Limited desktop evidence must be described honestly, not silently treated as a passed or waived requirement.
- Treat the cost of trying a change as a first-class criterion: Lua edit-to-result time, Rust incremental and clean build time, and build disk use. Do not presume live reload exists or select it in advance.
- Explore deep freedom: styling/layout, new view types, and replacing layout, input or view behaviour without a maintained fork. Keep a complete, coherent default editor; deep customisation must be optional. Neither a fixed toolkit nor an open substrate is selected.
- Study Zed, VS Code and Emacs as contrasting designs, not product templates or a predetermined ranking. The user's experience of speed, extensibility and Zed rebuild costs motivates questions, not measured comparative claims.
- Include the whole GUI path: Lua views/layout/commands/input dispatch, scheduling/event handling, rendering/caching and presentation. Keep unrelated editor subsystems out unless evidence establishes a GUI constraint. Retain the current Linux scope and existing platform/input behaviour; changes to backend implementation are not categorically excluded.
- Core library choices are not frozen. Replacing winit, softbuffer or rendering/rasterisation libraries such as tiny-skia is an eligible GUI-design option; earlier migration choices are not a veto. Compare alternatives against retaining/improving the current stack, including startup/responsiveness, customisation, compatibility, migration/maintenance overhead and build/dependency costs. Exploration does not authorize adoption.
- Use grilling and domain-modeling for live decisions, research for external facts, prototype for human-reviewed experiments, and codebase-design for interfaces/seams. Ask questions inline, explain terminology and trade-offs, and do not use structured questionnaire tools for this effort.
- Use the default model for reviews unless the user requests otherwise.
- Tracker conventions, claims and frontier queries are in [Local Wayfinder tracker](README.md). Resolve at most one non-research ticket per working session; charting hand-resolves none, with delegated research as the exception.

## Decisions so far

- [Compare editor GUI philosophies and extension costs](issues/40-compare-editor-gui-philosophies.md#resolution): primary-source evidence distinguishes extension freedom, core ownership and change-to-result mechanisms; no architecture or performance ranking is selected.
- [Inventory the GUI and legacy Lua extension contract](issues/39-inventory-gui-extension-contract.md#resolution): current GUI ownership and bundled extension dependencies are source-mapped; native goldens and a narrow normal-core shutdown check leave composed-GUI/plugin compatibility unestablished.
- [Define the GUI compatibility evidence and extension envelope](issues/41-define-gui-compatibility-envelope.md#resolution): preserve legacy behaviour against a pinned reference with focused/real-plugin evidence; new exposure stays reversible and experimental, and proposed visual changes need explicit review.
- [Establish GUI runtime and customisation iteration baselines](issues/42-baseline-gui-and-development-costs.md#resolution): bounded Xvfb GUI/resource, Lua restart and isolated Rust-build costs are recorded; cold-cache/physical-desktop latency and broader workloads remain unmeasured, with no acceptance gates selected.

- [Assemble the GUI and plugin compatibility reference evidence](issues/45-assemble-gui-compatibility-reference.md#resolution): byte-pinned normal-core and five-plugin scenarios run under isolated Xvfb; two initially selected plugins have recorded pre-existing failures, with desktop/input/visual limits left explicit.

## Not yet specified

- Which GUI responsibilities, state ownership and extension seams need to change after the contract inventory, evidence and philosophy decision. This includes which new experimental Lua access would be useful and how it can coexist with preserved legacy behaviour.
- Whether the current windowing, presentation and rendering/rasterisation libraries fit the chosen GUI direction, and which alternatives warrant bounded comparative evidence. A library migration is neither presumed nor excluded because of its overhead.
- Which concrete experiments would distinguish viable designs, and what needs live human review rather than automated evidence.
- How a selected direction can be introduced incrementally while keeping old plugins working, and what the final design and implementation handoff must specify. Details depend on whether the route is targeted improvements, redesign or both.

## Out of scope

- Implementing/adopting a production redesign during this planning map.
- Breaking existing Lite XL Lua interfaces or weakening existing user-facing contract tests to make a candidate pass.
- Replacing Lua, reopening prior efforts for reasons unrelated to GUI design, restoring non-Linux platforms, or redesigning unrelated editor subsystems.
- Cloning another editor's feature set, adding new user-facing features for their own sake, or selecting a GUI framework/rendering technology before the evidence and live decisions.
