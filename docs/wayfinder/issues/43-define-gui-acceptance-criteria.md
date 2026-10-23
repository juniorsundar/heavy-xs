# Define GUI performance and customisation acceptance criteria

Parent: ../gui-map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Blocked by: 41, 42, 46

## Question

What evidence must a GUI design satisfy for startup, responsiveness, resource use and ease of customisation, while preserving the old Lua contract and a complete default editor?

Use grilling and domain-modeling with the human, informed by [Define the GUI compatibility evidence and extension envelope](41-define-gui-compatibility-envelope.md) and [Establish GUI runtime and customisation iteration baselines](42-baseline-gui-and-development-costs.md). Prioritise startup-to-ready and interactive latency. Define representative ordinary/stress workloads, default and customised configurations, noise-aware numerical limits, and how regressions or unavailable hardware checks are treated. Include idle CPU, memory/retention and change-to-result costs, with clean/incremental builds and disk use kept distinct.

Translate customisation goals into observable scenarios across styling/layout, adding a view and replacing view/layout/input behaviour without maintaining a fork. Specify the default-editor parity and keyboard/input/IME/focus interactions needing review, retaining existing behaviour rather than assuming a new feature set. Separate easy-to-customise from merely technically possible; include how much core rebuilding an ordinary Lua customisation needs.

Preserve every existing contract gate and the additional compatibility evidence. Do not set arbitrary budgets before measurements or choose an architecture to satisfy them. Resolve with testable criteria, priorities, evidence gaps and candidate stop conditions; test/benchmark implementation remains a later handoff unless a separate prerequisite ticket is needed.

## Comments

### Agreed so far — priorities, evidence and customisation demonstrations

The user accepted the first round's recommendations:

- Preserve ordinary startup and interactive responsiveness, and require a worthwhile demonstrated improvement in startup, responsiveness or ease/depth of GUI customisation. An implementation change alone is not a benefit. Any meaningful regression requires explicit user review of its measured cost and the benefit it buys; compatibility remains non-negotiable. Existing costs are not automatically satisfactory merely because they are the reference.
- Design exploration may continue with the bounded Xvfb evidence, but controlled real-desktop comparisons are required before claiming a candidate's startup and interactive performance are qualified. Record desktop, display scale, refresh rate and measurement boundary; do not claim physical input-to-pixel precision the method cannot establish. Missing checks remain unverified, not passed. Which desktop checks are mandatory and the corresponding follow-up work remain to be decided.
- Demonstrate GUI customisation through small, documented Lua examples without a Rust rebuild or maintained fork: restyle/rearrange the interface, add a view/panel participating in layout/focus, change keyboard/pointer behaviour, and replace a view behaviour or layout policy. Judge required knowledge/work, interaction with existing plugins and runtime cost rather than an arbitrary line count. Native windowing/rendering backend changes may still require Rust work. New interfaces remain experimental and reversible under the settled legacy-preservation policy.

### Agreed so far — workloads, resources and native development costs

The user accepted the second round's recommendations:

- Ordinary-use comparisons cover the complete default editor (open/edit/save, tabs, splits, search and command interface), a small real project alongside the repeatable generated-file fixture, the agreed Lua customisation demonstrations, and a bounded real-plugin combination from the compatibility reference. Customisation is not automatically classified as stress. Separate stress workloads cover larger projects/files, long or complex lines, more views and heavier but bounded plugin work. Fix files, configurations and actions before comparing candidates. Ordinary use must meet the main responsiveness criteria; stress evidence identifies deterioration without promising responsiveness under arbitrary monopolising Lua work.
- Compare longer settled focused/unfocused idle CPU, typical/peak memory under ordinary/customised use, and repeated document/view open/close cycles without forced GC, allowing normal collection and settling. Reject reproducible continuing accumulation attributable to a candidate; meaningful CPU/memory increases require explicit user trade-off review. Lower idle cost is desirable, but improving every resource metric simultaneously is not required. Small allocator fluctuations alone do not establish a leak.
- Treat native development cost as a separate acceptance category: comparable no-op, small core-edit and empty-artifact builds; debug/release separately; build-cache disk, executable size and runtime memory separately; additional dependency-download/setup burden recorded. Meaningful increases require explicit user approval against a concrete benefit. Library migration effort is a cost to evaluate, not a categorical veto. No universal build-time cap is inferred from this machine's small sample.

### Agreed so far — calibration, default/input review and per-scenario qualification

The user accepted the third round's recommendations:

- Calibrate numerical comparisons with bounded, matched reference-against-reference measurements before agreeing limits. Establish ordinary variation and usable desktop timing boundaries, then distinguish measurement uncertainty from practical importance. Compare candidates under matched workloads/conditions, keeping typical and slower-tail behaviour separate. An inconclusive result does not automatically pass. This acceptance ticket stays open until calibration is available and the human has agreed numerical limits; no percentage or millisecond budget is invented from the exploratory sample.
- Require an explicit default/input review checklist: a complete readable default editor; keyboard-only navigation/commands; pointer selection, scrolling and resizing; focus between documents, custom views, prompts and menus; IME composition placement, completion and cancellation; and consistent text/cursor/selection/clipping/hit-testing geometry at tested scales. Retain existing supported Linux paths/checks. Unavailable required checks remain unverified, not permission to drop existing behaviour. Proposed appearance/layout changes still require human review under the settled compatibility policy.
- Evaluate each candidate with a scenario-by-scenario evidence table identifying demonstrations/reference comparisons, provisional/unverified/inconclusive results, failed criteria and explicit user-approved trade-offs. Do not use an overall score to let a strong result elsewhere cancel a failed requirement. Pause qualification at unresolved failures or missing required evidence, while exploration may continue. Confirmed legacy compatibility regressions remain blockers, including those discovered outside the selected sample; pre-existing reference failures remain separately recorded under the compatibility policy.

### Paused for calibration — not a resolution

Created [Calibrate GUI comparison noise and desktop timing boundaries](46-calibrate-gui-comparison-noise.md) as the bounded HITL prerequisite, blocked by the existing runtime baseline and plugin-reference evidence. Released this ticket's claim while it is blocked so it can be reclaimed after that evidence is available. The approved policy above is retained; it is not a completed acceptance specification or performance qualification.

After calibration, resume the live discussion to agree practical/noise-aware numerical limits for ordinary and stress workloads, resource/developer costs, and the required backend/scale/IME coverage and treatment of any remaining evidence blockers. Do not resolve this ticket or unblock [Choose the GUI customisation philosophy](44-choose-gui-customisation-philosophy.md) merely because policy has been agreed. No benchmark, desktop operation, candidate implementation or library selection was performed in this discussion.
