# Inventory the observable regex contract

Parent: ../map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-regex-inventory
Blocked by: none

## Question

What concrete behavior do the current regex implementation, Lua wrappers, callers and tests establish, so compatibility can be decided from evidence rather than assumptions?

Collect a bounded, source-cited inventory as an asset linked from the resolution:

- Functions, arguments, return shapes, compiled-object behavior, flags, byte offsets, Unicode/invalid-byte handling, errors and empty-match progress.
- Iteration and substitution semantics, including captures, replacement escapes and limits.
- Constructs used by bundled syntax definitions, startup metadata, document/project search and indentation detection.
- A clearly separated sample of installed third-party plugin use where available; it cannot establish compatibility with all plugins.
- Existing tests versus gaps. Distinguish observed behavior from documented or inferred behavior; label anything not run.

Start with `rust/src/regex.rs`, `rust/src/pcre.rs`, `data/core/regex.lua`, their callers and the contract suites. The existing regex inventory and advisory may be reused, but check claims against source. No engine research, implementation, baseline rewriting or compatibility decision belongs in this task.

## Comments

### Resolution

Completed by `pi-regex-inventory` against repository HEAD `4c02ffc8`.

- Evidence asset: [Observable regex contract inventory](../assets/regex-contract.md), with source citations, consumer/construct census, installed-plugin sample, test coverage and explicitly labeled gaps.
- Rebuilt and ran the existing debug regex and tokenizer suites: both passed; baselines unchanged.
- Added and ran a [bounded observational probe](../assets/regex-contract-probe.lua): 20 observations, 17 asserted and three error-message observations. [Recorded output](../assets/regex-contract-probe.txt).
- The inventory distinguishes observable quirks from requirements: suffix-relative matching, empty-iterator non-progress and positive-limit substitution behavior need explicit compatibility decisions, not automatic bug preservation. The tokenizer suite does not exercise bundled JavaScript/C/C++ regex definitions.
- Source-read safety/lifetime and application-budget gaps are recorded without executing unsafe cases. Runtime performance and arbitrary third-party compatibility remain unverified.
- Independent review approved the facts, evidence labels and bounded probe. No production code changed; no engine or ownership choice was made. Existing downstream tickets cover the surfaced decisions; no new ticket is needed merely to duplicate them.
