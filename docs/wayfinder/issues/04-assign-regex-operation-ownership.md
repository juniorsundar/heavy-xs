# Assign ownership of regex execution and composition

Parent: ../map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-regex-operation-ownership
Blocked by: 01, 02, 03, 06

## Question

Given [Define the Rust–Lua ownership rule](02-define-rust-lua-rule.md), [Choose the regex compatibility envelope](03-choose-regex-compatibility.md) and [Inventory the observable regex contract](01-inventory-regex-contract.md), which regex operations should execute in Rust and which should be composed in Lua?

Use [Define regex performance acceptance criteria](06-define-regex-performance-criteria.md) when evaluating disputed runtime costs.

Cover compilation and handle lifetime, single-match/capture-offset execution, match iteration, replacement-template interpretation, substitution loops, byte/character conversion and execution limits. Distinguish engine work from editor policy. Identify intended user override/composition points: a Lua-callable native mechanism is not the same as replaceable editor behavior. When alternatives meet the runtime criteria, favor user-controlled Lua behavior where customization is intended.

Keep the protected Lua-facing interface unless a separate explicit decision changes it. Do not assume that moving iteration or substitution to Lua is cheaper: evaluate crossing frequency, allocation/GC cost and coarse-grained native operations. Conversely, do not retain policy in Rust merely because it is there today.

Record the proposed internal interface and evidence needed to validate it. If a material ownership choice needs measurement or a prototype, create that decision ticket instead of inventing results. Do not implement the chosen interface in this session.

## Comments

### Resolution

The user accepted all six recommendations. Evidence: bundled code and the sampled plugins never call `gmatch`; `gsub` is used once per find-replace over the whole text and by the snippet plugin; the [release baseline](../assets/regex-release-baseline.md) shows whole-document replacement as one ~1.2 ms native call with no disputed crossing cost; and [the performance criteria](06-define-regex-performance-criteria.md) require native work/output limits that stop executing work.

| Operation | Owner | Notes |
|---|---|---|
| Compilation and compiled-pattern lifetime | Rust | Lua receives an opaque compiled-pattern object; the raw pointer is not required. Lua owns caching policy (e.g. tokenizer caches). |
| Single match from a byte offset, returning match/capture byte offsets | Rust | Keeps the agreed suffix-relative `cmatch` semantics. |
| `find`/`find_offsets`/`match` helpers, byte→character conversion, search direction/wrap, selection, syntax rules | Lua | Editor policy and intended user override/composition points. |
| `gsub`: match loop, replacement-template interpretation, output building | Rust | Limits must stop work mid-call; the template dialect is a fixed contract, not a customization point; one call is cheap. Custom replacement logic composes in Lua from single matches. |
| `gmatch` iteration | Rust | Native iterator owns the agreed UTF-8-safe progress guarantee and keeps its pattern alive. A Lua loop over suffix-relative `cmatch` would change `^`/lookbehind semantics; no bundled caller makes cost relevant. |
| Work and output-size limits | Rust enforcement, Lua configuration | Module-wide defaults set from Lua; existing call signatures unchanged. Default values chosen by the design ticket. |

The protected Lua-facing interface stays as is. No operation moves across the boundary, so no measurement or prototype ticket is needed. Moving `gsub` or `gmatch` to Lua would require a new measured decision. This ticket chooses no engine and implements nothing.
