# Assemble the GUI and plugin compatibility reference evidence

Parent: ../gui-map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-gui-reference
Blocked by: 41

## Question

Which reproducible reference behaviours and pinned plugin configurations provide concrete evidence for the GUI philosophy/design discussion under the policy in [Define the GUI compatibility evidence and extension envelope](41-define-gui-compatibility-envelope.md#resolution)?

Assemble bounded evidence, not a production test framework or a candidate implementation. Use the pinned pre-redesign source, build inputs and existing acceptance expectations from that decision and [Inventory the GUI and legacy Lua extension contract](39-inventory-gui-extension-contract.md); record environment, platform/backend, fonts, scale, plugin/configuration revisions, dependencies and enabled combinations. Separate source inspection, automated observations, live observations and unavailable checks. Pin executable/build provenance if running a reference; a source hash alone does not identify the executable or runtime data.

Select a bounded real third-party sample by exercised patterns, not popularity: view subclassing and insertion, layout manipulation, drawing/method overrides, input/commands, shared configuration/style and lifecycle/focus. Cover interactions between plugins that touch the same area. Name exact upstream revisions and supported configurations, investigate the current mod-version versus dated upstream branch guidance, and record selection rationale and uncovered patterns. A plugin that already fails on the reference is a recorded pre-existing failure, not a passing compatibility probe or a newly approved exclusion.

Pair the sample with concrete focused GUI-contract scenarios: arguments/returns and callback order; class/object identity; geometry/clipping/measurement/hit testing; view activation/close/quit; command/keymap precedence and text/IME consumption; prompts/status/menu integration; and existing deprecated call forms. Reuse existing checks and disposable probes where possible. Establish enough baseline evidence to expose design constraints; no comprehensive regression-suite implementation is required here, and future candidate validation remains a separate handoff. Report resource or evidence blockers rather than inventing results or declaring missing checks passed.

Do not weaken existing tests/goldens, fix production code, select an architecture or expand this task into repairing unrelated historical failures. New experimental Lua access is not retroactively part of the preserved reference. Reference appearance is evidence for later explicit human review, not a demand for pixel-identical future composed frames beyond existing frozen gates.

Third-party execution is security-sensitive: inspect the source and obtain advisory review before choosing the execution setup. Disposable configuration directories are not a security sandbox. Do not run untrusted plugins with access to personal configuration, credentials or user projects; if adequate isolation is unavailable, record the blocker and leave execution unverified. Delegate external-source investigation to an appropriate research helper. Do not silently install dependencies or launch unbounded builds.

Resolve with linked reference evidence/manifest under `docs/wayfinder/assets/`, provenance, observed outcomes, gaps and reproducible instructions. Detailed probe design is constrained by facts uncovered here, not preselected in this ticket.

## Comments

### Resolution

Assembled the bounded [GUI and plugin compatibility reference](../assets/gui-compatibility-reference.md), its [seven-file canonical manifest](../assets/gui-compatibility-reference/manifest.json), [observed results and provenance](../assets/gui-compatibility-reference/results.json), and disposable reproduction helpers. Default-model deep review found no blocking findings.

1. The executable is the byte-pinned clean release from [Establish GUI runtime and customisation iteration baselines](42-baseline-gui-and-development-costs.md#resolution), built from `e556d388892280d8343a82a625941137e3a6ac5f` without test hooks. All 90 staged runtime files, including fonts and bundled plugins, were checked against the pinned source/install substitution; source, executable, build inputs, backend/scale, fixture, configurations and helper hashes are recorded.
2. Primary-source investigation established an eligible mod-version 4 source at `adamharrison/lite-xl-plugins@ce2da6f190ceea5405ec211fb1d3d47e6c770179`. Canonical bytes matched Git blob IDs, sizes and SHA256s; nothing was ported or header-patched. The earlier research transcriptions are not execution inputs. Source inspection and isolation advisory reviews preceded execution; filesystem/network/environment/PID and negative-display gates passed before any third-party launch, on an owned private Xvfb with no personal files or host session services exposed.
3. Big Clock, Autosave on Focus Lost, Scale Status, Custom Caret and Smooth Caret passed individually; the caret pair and working five-plugin combination also passed. Their extension patterns and captured-method order are documented. The normal-core control and seven successful plugin configurations completed the focused nine-phase GUI probe, covering the bounded identity/layout/clipping, callback/draw, command/keymap, focus/save, prompt/status/menu, synthetic text/IME, custom-view and forced-quit assertions listed in the report.
4. Bracket Match and Indent Guide already fail on the untouched reference: their first update calls selection methods absent from this `DocView`. The original five-plugin and text-decoration pair configurations therefore also fail. These four failed observations remain visible as pre-existing failures, not passing compatibility probes, repairs or newly approved exclusions. Accepted mod-version headers alone do not establish API compatibility.
5. Evidence is one final run per configuration, not performance measurement or whole-plugin certification. No live human/visual observations were collected. Native input delivery, real IME, fractional/native-Wayland behaviour, broader text/geometry, multiple/off-screen carets, reverse wrapper order, other legacy overloads, full close/confirmation paths and heavy plugin integrations remain explicitly unverified. Existing tests/goldens and production code are unchanged; no architecture, numerical budget or candidate approval was selected.

This supplies concrete reference configurations to [Calibrate GUI comparison noise and desktop timing boundaries](46-calibrate-gui-comparison-noise.md). That human-coordinated prerequisite and [Define GUI performance and customisation acceptance criteria](43-define-gui-acceptance-criteria.md) remain open. Further desktop execution requires its own agreed protocol and safety review; private-Xvfb observations do not qualify desktop performance.
