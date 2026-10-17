# Rust/Lua responsibilities and a Rust-native regex design

Labels: wayfinder:map
Status: resolved

## Destination

Agree on a Rust/Lua responsibility policy and a concrete design for replacing PCRE2 with a Rust-native regex implementation, ready for implementation with explicit compatibility and acceptance criteria.

## Notes

- Originally planning-only. The user subsequently authorized a lean fancy-regex implementation after the community audit; the selected native dialect supersedes the earlier PCRE2-preservation target. Approved changes are recorded in the resolved decision and the lean contract indexed below.
- The user confirmed these priorities: interactive responsiveness and bounded memory first, then simplicity and extensibility; minimizing Rust source lines or maximizing Lua code is not the objective.
- Reduce C-library and binding dependencies, starting with PCRE2. This does not require eliminating the Lua runtime or OS interfaces.
- Preserve the Lua-facing regex interface and bundled functionality subject to the resolved compatibility envelope. No additional syntax exclusions or behavior changes are implicit.
- The proposal to move `gmatch` and `gsub` into Lua is unapproved; ownership and crossing costs must be considered.
- Use Wayfinder for the map, grilling and domain-modeling for HITL decisions, codebase-design for the interface discussion, and research/prototype skills when tickets call for them.
- Tracker conventions and frontier queries are in [Local Wayfinder tracker](README.md). Nothing is claimed during charting.
- [Regex alternatives](../regex-alternatives.md) is advisory background, not a settled engine choice. [The retired port plan](../rust-port/PLAN.md) records historical contracts and deviations; it is not reopened.

## Decisions so far

- [Inventory the observable regex contract](issues/01-inventory-regex-contract.md): current API, consumers, bounded observations and coverage gaps are recorded as evidence for compatibility and ownership decisions.
- [Define the Rust–Lua ownership rule](issues/02-define-rust-lua-rule.md): mechanism/policy ownership, intended user override/composition, justified batching and workload evidence govern placement.
- [Choose the regex compatibility envelope](issues/03-choose-regex-compatibility.md): required dialect/Unicode/replacement semantics, retained quirks, diagnostic flexibility and explicit safety/iterator corrections are agreed; evidence gaps remain gated.
- [Assemble the regex compatibility acceptance corpus](issues/07-assemble-regex-compatibility-corpus.md): bounded fixtures, replacement audit and reference/correction evidence are recorded; candidate-only safety and semantic gaps remain explicit.
- [Collect the release regex workload baseline](issues/08-collect-release-regex-baseline.md): bounded headless release observations and unselected budget options are recorded for the criteria decision; real-window latency and enforceable native limits remain unmeasured.
- [Define regex performance acceptance criteria](issues/06-define-regex-performance-criteria.md): ordinary/stress classes, 33.3 ms ordinary blocking, 64 MiB/retention gates, 25% slowdown review and mandatory native work/output limits are agreed.
- [Assign ownership of regex execution and composition](issues/04-assign-regex-operation-ownership.md): Rust owns compilation/lifetime, single matches, `gsub`, `gmatch` and limit enforcement; Lua owns helpers, offset conversion, search/selection/syntax policy and limit configuration.
- [Spike fancy-regex against the regex acceptance evidence](issues/09-spike-fancy-regex.md): disposable candidate evidence exposes compatibility failures, blocked highlighting and remaining limit/performance questions; no engine or feature loss is approved.
- [Survey Rust-native regex engine options](issues/11-survey-rust-regex-engines.md): fancy-regex is the only released pure-Rust engine close to the needed dialect (16/25 probe cases vs 12 regress, 11 resharp); routes left are translation over fancy-regex, a fork, or a dedicated engine.

- [Lean decision and community plugin audit](assets/regex-plugin-audit.md): adopt native fancy-regex patterns/replacements, fix the bundled rule and remove PCRE2 emulation; no translator or custom replacement interpreter. Resource protection is per-execution backtracking plus capped output, not an aggregate budget.

## Follow-up

- The lean integration is implemented and [validated](assets/regex-lean-validation.md). The user accepted the four measured relative slowdowns; a separate [agent handoff](../regex-slowdown-handoff.md) scopes their investigation without reopening the dialect choice.

## Out of scope

- Broad Rust/Lua migration beyond the specifically authorized lean regex integration.
- Replacing SDL3, FreeType, Lua or libc; restoring non-Linux support.
- Adopting Tree-sitter or LPeg, redesigning syntax plugins, or adding structural language features.
- Reopening the completed Rust-port plan or optimizing unrelated editor subsystems.
