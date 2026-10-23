# Compare editor GUI philosophies and extension costs

Parent: ../gui-map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-gui-philosophies-research
Blocked by: none

## Question

What design choices in Zed, VS Code and Emacs explain their different forms of GUI customisation, runtime responsiveness and change-to-result costs, and which trade-offs are relevant to a Lua-extensible Rust-core editor?

Use the research skill and primary sources: official documentation and versioned source. Compare the same dimensions across all three: what extensions can own or replace; what remains core-only; GUI composition/layout/state/input; scheduling and rendering responsibilities; extension startup/loading and execution; isolation versus direct internal access; and the path from a small extension or core change to trying it.

Distinguish extension development from rebuilding the editor. Document mechanisms and published evidence, not unsupported claims that an editor is inherently slow or fast. Attribute performance/build observations to their version, workload and environment; do not invent head-to-head numbers or perform expensive builds. Emacs is an editor with an embedded extension language; investigate its actual extension/redisplay choices rather than relying on slogans.

Explain transferable principles and constraints in plain language. Separate source facts, inference and unknowns. Do not select a philosophy, framework, GPU route, new interface or compatibility concession for this project.

Capture one cited Markdown asset on an isolated throwaway `research/gui-philosophies` branch (use a unique suffix if occupied). Push that branch without opening a PR; it is not to be merged. Record its branch, commit and asset URL in this ticket; keep a context pointer so later sessions can retrieve it without checking out or merging it. Claim before work and do not alter production files or unrelated working-tree changes.

## Comments

### Research claim

Claimed for the delegated research session. The canonical tracker stays in the main checkout; the research asset lives on an isolated throwaway branch, linked below. No PR or merge is intended.

### Resolution

The delegated primary-source research is captured in [Editor GUI philosophies and extension costs](https://github.com/juniorsundar/heavy-xs/blob/ba700e9fc02740b379456cb15288ddfabdfac721/docs/wayfinder/assets/gui-philosophies.md). It distinguishes extension capabilities from core GUI implementation and extension iteration from core rebuilding: Zed's reviewed capability-specific extension model, VS Code's contribution/hosted-view model, and Emacs's live Lisp programmability within its editor/display model. Facts, inference and unknowns are separated; no project philosophy or architecture is selected.

Context pointer: branch `research/gui-philosophies`, commit `ba700e9fc02740b379456cb15288ddfabdfac721`, asset `docs/wayfinder/assets/gui-philosophies.md`. The branch is pushed, with no PR and no intended merge. Retrieve without switching the main checkout using `git show research/gui-philosophies:docs/wayfinder/assets/gui-philosophies.md` (fetch the branch first if absent).

Limits: current docs and tagged source contexts are distinct, GNU HTML fetches were unreliable, and source-surface checks are not exhaustive implementation audits. No startup, latency, memory, build-time or disk benchmarks were run. The evidence supports the live philosophy discussion, not performance rankings or a promise that every editor offers arbitrary UI replacement.

Validation: primary-source research and a focused documentation review completed; `git diff --cached --check` passed before the evidence-only commit, and the research branch push succeeded. Existing production files, tests and unrelated working-tree changes were not included.
