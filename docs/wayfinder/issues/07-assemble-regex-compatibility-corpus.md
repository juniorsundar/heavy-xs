# Assemble the regex compatibility acceptance corpus

Parent: ../map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-regex-compatibility-corpus
Blocked by: 03

## Question

Which concrete fixtures, expected outcomes and source-backed coverage gaps will let candidate designs be checked against [Choose the regex compatibility envelope](03-choose-regex-compatibility.md), using [Inventory the observable regex contract](01-inventory-regex-contract.md) as the starting evidence?

Assemble bounded evidence assets, not a replacement implementation. Reuse the existing contract suites and inventory instead of building a new test framework. Record the exact reference build/version and distinguish old behavior to retain from explicitly approved corrections.

Cover:

- The approved pattern feature floor, including constructs not exercised by the old suites: lookbehind/backreferences, atomic/possessive semantics, inline flags and actual bundled regex definitions.
- Byte offsets, UTF-8 validation, default class behavior, captures and method/return conventions. Do not silently assign new meanings to undefined or unsafe capture cases.
- The known replacement forms and a source-backed audit of additional extended replacement syntax, with minimal safe examples and expected outputs/errors. Distinguish verified forms from remaining gaps; do not infer that another engine's template grammar is compatible.
- Representative snippet transformations using the sampled plugin's compile/match/gsub call flow. Separate synthetic fixtures from expressions actually found in installed snippets. Do not copy private snippet bodies or configuration into the repository; record absence of a usable corpus rather than inventing real-world coverage.
- Retained quirks: suffix-relative cmatch, ENDANCHORED aliasing, ignored compile-flag letters and positive-limit substitution.
- Approved iterator/safety corrections, clearly separated from old behavior. Include whole-match advancement and bounded UTF-8-safe empty-match progress cases, without assuming a particular implementation algorithm.

Do not execute known-unsafe old-engine offset/handle/lifetime cases merely to capture a crash, or use an unbounded iterator to prove non-progress. Describe safe candidate acceptance checks for those cases instead. No production code or existing baselines may be rewritten.

Use repository-local source/tests/documentation where available. If an external library fact is needed, surface a focused research blocker; do not guess. If an observable semantic choice remains unresolved, surface it for HITL decision rather than turning an AFK evidence task into a policy decision.

Return a coverage matrix linking fixtures/results and remaining gaps. A passing reference corpus is evidence about the current implementation, not proof that a Rust-native candidate meets it. Candidate-specific unsupported constructs or semantic differences still require explicit approval before adoption.

## Comments

### Resolution

Assembled the bounded [acceptance corpus and coverage matrix](../assets/regex-acceptance.md), with [pattern/interface/caller cases](../assets/regex-acceptance-cases.lua), [replacement cases](../assets/regex-replacement-cases.lua), the [replacement audit](../assets/regex-replacement-audit.md) and the [runtime](../assets/regex-acceptance.lua). The report records reference revision/build hashes, exact reproduction commands, source evidence and limitations.

Validation:

- Fresh debug build and existing regex/tokenizer contract suites pass; no production code or old baseline was rewritten.
- [Reference results](../assets/regex-acceptance-reference.txt): 157 checks, zero failures (155 retained-behavior assertions and two bounded old-iterator observations).
- [Candidate-mode negative control on the unchanged reference](../assets/regex-acceptance-old-candidate.txt): exactly four expected iterator failures among 160 checks. This demonstrates the correction checks, not passing candidate behavior; correcting the implementation is outside this task.
- All three new Lua assets pass syntax checks. A temporary expected-return-count mutation produces the intended failure/nonzero exit.
- Independent deep review confirmed fixture/result agreement, provenance, bounded execution, actual bundled tokenizer coverage, and separation of synthetic snippets, retained behavior and approved corrections.

The current build's PCRE2 archives identify 10.46, while the old optional local manual/source is 10.44. The audit distinguishes documentation-backed forms from additional current-build observations; none are implicitly excluded from the current-dialect preservation target. It does not claim exhaustive grammar coverage.

No real installed snippet-expression corpus was collected; the transform fixtures are explicitly synthetic and follow the sampled consumer's method flow. No unsafe offset/handle/GC-lifetime or wrapper unset-capture probe ran. Those safety gates, unmatched-capture return conventions and exact mixed empty/consuming retry semantics are called out for HITL/design follow-up in [Select the Rust-native regex design](05-select-rust-native-regex-design.md). A coverage gap is not permission to choose its semantics or remove a feature.

The evidence is sufficient to unblock [Collect the release regex workload baseline](08-collect-release-regex-baseline.md), not to approve an engine, operation placement, numeric resource budget or compatibility loss.
