# Define the GUI compatibility evidence and extension envelope

Parent: ../gui-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-gui-compatibility
Blocked by: 39

## Question

Beyond the frozen existing tests, what additional observable GUI and extension behaviours must a candidate demonstrate, and how will we establish that old Lua plugins still work?

Use grilling and domain-modeling with the human. The standing requirement is preservation, not a request to approve compatibility breaks. Existing functions and their behaviour cannot be removed or changed to fit a candidate. Additive Lua interfaces are allowed, but changed return shapes, callback order or object lifetimes are not automatically additive.

Use [Inventory the GUI and legacy Lua extension contract](39-inventory-gui-extension-contract.md) to discuss concrete plugin scenarios: subclassing a view, overriding a core method, adding drawing/layout, intercepting input, registering commands/configuration and composing independently authored plugins. Resolve what compatibility evidence must cover, including the extent and provenance of a bounded third-party plugin sample and how untested dependencies are handled. Do not silently limit the promise to native ABI or bundled plugins, or promise every possible monkeypatch works.

Specify the additional acceptance evidence and reference/provenance needed without implementing the tests here. Distinguish preservation of old interfaces from the future guarantees for newly exposed internals; detailed new interfaces wait for the design direction.

## Comments

### Live decisions — reference, compatibility layers and evidence

The user accepted all recommendations in the first discussion round:

- Use a pinned pre-redesign snapshot of this editor as the behavioural comparison reference, retaining existing Lite XL compatibility requirements and prior approved differences. Record pre-existing failures separately from redesign regressions; neither hide them nor automatically expand this GUI effort into fixing all historical incompatibilities.
- Compatibility layers are allowed: the implementation can change while old plugins keep working without modifications, preserving relevant arguments, callbacks/order, drawing behaviour and object identity. Their runtime and maintenance costs count against the eventual design criteria; no specific layer is selected.
- Require both focused GUI-contract evidence and a bounded, version-pinned sample of real third-party plugins, including combinations. Select by exercised extension patterns, not popularity; establish each configuration on the reference before comparing a candidate. Sampling is evidence, not certification of every plugin combination.

### Live decisions — visuals, missed dependencies and experimental exposure

The user accepted the second-round recommendations on reviewed visual/layout changes and confirmed redesign regressions, with a clarification overriding the proposed stability-first policy for new interfaces: this effort is experimental, so newly exposed internals may be changed or reverted freely, provided existing exposure is not degraded. No requirement to stabilise new internals before experimentation is imposed. Existing legacy exposure cannot be relabelled experimental to escape preservation.

### Resolution

The user confirmed the recommendations with the experimental-exposure clarification recorded above. The following is the settled compatibility policy; no architecture or candidate is selected:

1. **Reference and preservation.** Compare against a fixed pre-redesign editor snapshot, using source revision `e556d388892280d8343a82a625941137e3a6ac5f` from the inventory as the source reference. Pin the build, runtime data, environment and plugin configurations with the reference evidence. Retain existing Lite XL compatibility requirements, prior approved differences and every existing contract-test expectation. Record pre-existing failures separately; do not hide them, count them as passes or automatically expand this GUI effort into repairing all historical incompatibilities.
2. **Legacy behaviour, not literal implementation.** Preserve the working relationship of old plugins/configuration with the editor: relevant functions, arguments/returns, callbacks/order, drawing, subclassing/overrides, object/class identity, geometry and lifetimes. Compatibility layers are allowed if old plugins need no changes and observable legacy behaviour remains correct; their runtime/maintenance costs count against later design criteria. No particular layer or new internal architecture is chosen.
3. **Experimental additions.** Newly exposed internals are experimental and may be modified or removed freely, as long as existing exposure is not degraded. Identify additions as experimental so their users do not mistake them for the legacy compatibility promise. There is no blanket requirement to design permanent guarantees first, nor any permission to reclassify an old interface as experimental. A new experiment does not automatically enlarge the frozen legacy surface.
4. **Appearance and geometry.** Proposed default appearance/layout differences may be presented for explicit user review, rather than demanding pixel-identical composed frames. No unseen difference is preapproved. Existing tests and pixel expectations stay unchanged; text measurement, cursor placement, hit testing, clipping and plugin drawing must remain internally consistent and honour the legacy interface behaviour.
5. **Evidence beyond primitive tests.** Require focused GUI-contract tests plus a bounded, version-pinned sample of real third-party plugins and combinations. Select by extension patterns, including views/layout/drawing/input/commands/shared state and lifecycle interactions, rather than popularity. Establish selected configurations on the reference before comparing candidates. Evidence must distinguish native primitive/ABI coverage from normal composed-GUI and Lua-extension coverage; a finite sample is not certification of every combination.
6. **Missed dependencies and regression handling.** When a previously untested plugin or configuration reports a failure, reproduce reference and candidate behaviour and establish causation. A confirmed redesign regression in legacy extension behaviour blocks adoption until compatibility is restored, including undocumented dependencies outside the initial sample. Do not dismiss it merely as undocumented, promise every possible monkeypatch works, or classify an unavailable check as passing. Pre-existing failures remain visible and distinct from regressions.

The new prerequisite [Assemble the GUI and plugin compatibility reference evidence](45-assemble-gui-compatibility-reference.md) makes the now-sharp evidence collection question actionable. It will supply pinned configurations, observed reference behaviour and gaps before the philosophy selection; it does not implement the redesign or an exhaustive new test framework. Detailed new interface/prototype design and numerical performance criteria remain with their later decisions.

Validation: default-model review found no blocking fidelity, scope or dependency issues. Tracker metadata, links, resolution anchors and dependency cycles were checked; production code, existing tests and their expectations remain unchanged. No plugin sample was selected or executed, no compatibility test implementation was written and no new ticket was worked in this session.
