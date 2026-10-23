# Choose the GUI customisation philosophy

Parent: ../gui-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Blocked by: 40, 41, 43, 45

## Question

What should the default GUI own, what should Lua users/plugins be able to replace or compose, and what trade-offs best satisfy the compatibility and acceptance criteria?

Use grilling and domain-modeling with the human, plus codebase-design for interfaces/seams. Compare a strong structured model, a more open programmable substrate and a layered combination using the current editor inventory and [Compare editor GUI philosophies and extension costs](40-compare-editor-gui-philosophies.md). Respect [Define the GUI compatibility evidence and extension envelope](41-define-gui-compatibility-envelope.md) and [Define GUI performance and customisation acceptance criteria](43-define-gui-acceptance-criteria.md). Use [Assemble the GUI and plugin compatibility reference evidence](45-assemble-gui-compatibility-reference.md) for concrete legacy dependencies and reference gaps. These are discussion aids, not a prescribed menu or a choice of framework.

Stress-test the direction with concrete scenarios: replacing tab/pane arrangement, implementing a new view, changing input behaviour, composing two plugins that customise the same area, and opting out of customisation entirely. Explain implications for startup/responsiveness, default coherence, ease of trying changes, old method overrides and the experimental/reversible nature of newly exposed Lua internals, without degrading existing exposure.

Choose principles and ownership at a level justified by evidence; do not prematurely select widgets, a renderer, a framework or detailed interfaces. The conclusion may be targeted improvements rather than a rewrite. Resolve only through the user's live answers. Graduate now-sharp architecture, prototype and handoff questions from the map's fog after this decision.

## Comments
