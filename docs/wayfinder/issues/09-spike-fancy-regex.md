# Spike fancy-regex against the regex acceptance evidence

Parent: ../map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-fancy-regex-spike
Blocked by: none

## Question

How does fancy-regex measure against [the compatibility corpus](../assets/regex-acceptance.md), [the performance criteria](06-define-regex-performance-criteria.md) and the decisions recorded in [Select the Rust-native regex design](05-select-rust-native-regex-design.md)?

Time-boxed evidence spike on a disposable branch; never merged and no production change on the main line. Put fancy-regex behind the existing Lua-facing regex module, following [Assign ownership of regex execution and composition](04-assign-regex-operation-ownership.md), only as far as needed to run:

- The acceptance runtime in candidate mode (pattern, interface, tokenizer, replacement and iterator-correction cases) plus the agreed empty-match, unmatched-capture and out-of-range-offset outcomes. A minimal Rust template interpreter is in scope only to the extent needed to exercise replacement cases; record any case it cannot yet cover rather than guessing.
- The [release baseline](../assets/regex-release-baseline.md) workloads, same machine and protocol, compared against the reference.
- A small pathological-pattern set (catastrophic backtracking, nested quantifiers, large outputs), assembled here, checking that fancy-regex's backtrack limit and an output-byte limit stop work within 1 second with a distinct error, and that no ordinary or stress workload reaches the candidate default.

Check engine-default differences explicitly, including ASCII-only `\w`/class behavior, UTF-8 validation of invalid subjects, possessive-quantifier semantics, inline flags, and lookbehind forms.

Deliver a report under `docs/wayfinder/assets/`: pass/fail matrix, every failure as a concrete pattern with old versus new behavior and affected consumers, performance results against each criterion, and observed backtrack-limit values suitable as default options. Do not choose the engine, accept a loss or set the defaults; those return to the HITL design ticket.

## Comments

### Resolution

Completed the bounded disposable spike; the [report](../assets/fancy-regex-spike.md) links exact failing patterns, caller results, release measurements and limit options. Source is frozen on local branch `spike/fancy-regex`, commit `effe6189bdcfb05318bb1139982d2633edcfa49b`, with a [portable source patch](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/fancy-regex-spike.patch). Never merge this branch; it was not pushed. No production source/build input or existing contract baseline changed in the main worktree.

Findings:

- Candidate mode passes 155/160 checks: ASCII-class semantics differ, the bundled JavaScript start regex is rejected (three tokenizer failures), and `MARK` is unsupported. Additional dialect probes expose further class/escape differences and adapter absolute-anchor flag gaps. No pattern was silently rewritten and no loss is approved.
- All 17 candidate-only safety checks, 13 bounded resource assertions and eight replacement-interpreter unit tests pass. Unsafe-reference offset/handle/GC probes were not executed against PCRE2. Finite tests are not an exhaustive safety/dialect proof.
- Seven reachable workload cohorts finish five measured actions each. JavaScript full/highlight/edit cohorts are blocked, not timing passes. Prototype replacement and synthetic transforms cross the 25% median-slowdown review trigger; absolute segment/memory/retention observations fit the gates only for the exercised subset.
- Tested pathological cases stop within one second with distinct work/output errors. Sweep settings 100,000 and 1,000,000 plus a 1 MiB output cap let the seven reachable workloads finish. These are experiment options only: delegated scan work and aggregate VM accounting remain unproven, so this does not establish the required general native-work mechanism or certify defaults for every workload.

Validation includes release build, interpreter unit tests, Lua controls, matched reference runs, raw-statistic checks, portable-patch replay, PCRE2-symbol absence and one focused final review. The negative compatibility results are the assessment outcome, not an implemented/adopted replacement.

[Select the Rust-native regex design](05-select-rust-native-regex-design.md) is unblocked for HITL conflict/engine/default decisions; it is not resolved by these findings.
