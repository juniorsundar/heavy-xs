# Define the Rust–Lua ownership rule

Parent: ../map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-rust-lua-rule
Blocked by: none

## Question

What operational rule determines whether a responsibility belongs in Rust or Lua, given the user's agreed priorities?

Use concrete examples to distinguish resource ownership, performance-sensitive mechanisms, editor policy and composition. Establish when batching, allocation cost, language crossings or trust constraints justify native work, and when configurability and locality justify Lua.

The priority order is already agreed; do not re-ask whether fewer Rust lines alone is success. Do not turn the provisional “mechanism in Rust, policy in Lua” heuristic into a blanket rule without testing its exceptions. Identify how uncertain cases should be measured or prototyped, rather than assuming all loops stay native or all wrappers move to Lua.

This ticket chooses the general rule. It does not choose a regex engine or settle ownership of individual regex operations.

## Comments

### Discussion in progress

The user accepted the default Rust mechanism/resource-safety versus Lua editor-policy rule, justified coarse-grained native operations, and representative-workload evidence for disputed performance claims. Those answers do not assign individual regex operations.

The user then added that behavior a user would want to override or compose should favor availability through Lua over a fixed Rust implementation. They requested a concrete discussion of the tradeoff before agreeing to the rule below.

### Resolution

Confirmed by the user after the initial three choices and the follow-up discussion of customization versus runtime cost.

1. **Default ownership:** Rust owns native-resource lifetimes, safety-critical invariants and performance-sensitive mechanisms. Lua owns editor policy and composition. Being technically possible in Lua is not by itself a reason to move an operation there.
2. **Intended user control:** users should be able to replace and compose intended customization points without rebuilding Rust. When both designs meet the agreed responsiveness and memory criteria, that control favors replaceable Lua behavior, even if a fixed native implementation could be faster. This does not promise replacement of every internal algorithm or speculative hooks for every imaginable use.
3. **Callability is not replaceability:** exposing a native operation to Lua can be sufficient for a fixed mechanism. It is not sufficient when users need to replace the editor's policy or composition. Conversely, implementation in Lua alone does not guarantee an accessible customization point; a hidden local function may still be inaccessible.
4. **Justified batching:** Rust may expose coarse-grained operations when significant crossing/allocation costs justify them, with editor choices controlled from Lua. Prefer an efficient interface over insisting on the smallest native primitive. If an intended Lua customization misses runtime criteria, investigate batching or a better interface rather than automatically removing user control; an unresolved tradeoff returns to the user.
5. **Evidence for disputed placement:** compare representative end-to-end workloads before committing performance-based moves. Responsiveness and memory come first; allocation/GC and crossing measurements can explain costs. Microbenchmarks alone, language stereotypes and Rust line counts do not settle placement. Leave unproven placement unchanged pending evidence. Resource/safety ownership does not require a speed benchmark to justify it.

The rule sets a decision process, not an operation-by-operation allocation. No engine, `gmatch`/`gsub` placement, compatibility reduction, latency/memory threshold or new customization feature is selected here. Native safety invariants remain protected.

[Define regex performance acceptance criteria](06-define-regex-performance-criteria.md) specifies the workloads, baseline and criteria needed to apply the evidence rule. [Assign ownership of regex execution and composition](04-assign-regex-operation-ownership.md) applies the agreed rule to individual operations.
