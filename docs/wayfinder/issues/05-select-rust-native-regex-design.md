# Select the Rust-native regex design

Parent: ../map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: none
Blocked by: 03, 04, 07, 09, 10, 11

## Question

Which Rust-native regex implementation and internal design should replace PCRE2 after [Choose the regex compatibility envelope](03-choose-regex-compatibility.md) and [Assign ownership of regex execution and composition](04-assign-regex-operation-ownership.md) are resolved?

Use [Assemble the regex compatibility acceptance corpus](07-assemble-regex-compatibility-corpus.md) as evidence for the actual constructs, offsets, flags and replacements; also require evidence for the failure limits the design needs. Use the resulting [coverage matrix and candidate-only safety gates](../assets/regex-acceptance.md), including its [replacement audit](../assets/regex-replacement-audit.md). Resolve unmatched-capture return conventions and exact mixed empty/consuming iteration semantics through HITL decisions, and validate handle/offset/lifetime safety before enabling those checks; the old engine must not be used as an unsafe oracle. The replacement target is the current linked dialect, not merely the older local PCRE2 manual. Meet the criteria in [Define regex performance acceptance criteria](06-define-regex-performance-criteria.md): assemble its pathological-pattern set, and choose default work/output limits and their enforcement mechanism. Resolve any remaining observable edge-case choices before calling the design implementation-ready. Unlisted constructs are not pre-approved exclusions: present concrete old/new behavior and affected consumers for explicit approval before accepting any candidate-specific incompatibility. Use existing primary-source comparisons as leads; create focused research or prototype tickets for any remaining factual blockers before resolving this choice.

The decision must name the engine, explain how it fits the chosen interface, define observable acceptance checks and a comparison baseline, and describe how PCRE2 and its bindings leave the final build. Any requested fallback, compatibility layer or behavior change needs justification and explicit approval; a permanent two-engine design is not presumed.

If no candidate meets the constraints, return that conflict to the user rather than silently weakening compatibility, bounded execution or the Rust-native goal. Resolution should make the design ready for implementation, not implement it.

## Comments

### Discussion in progress — engine not chosen

The user accepted these engine-independent decisions:

1. **Evidence before choosing.** The engine is chosen only after [Spike fancy-regex against the regex acceptance evidence](09-spike-fancy-regex.md). If the candidate fails, the conflict returns to the user (custom engine versus explicitly accepted, named losses); compatibility is never silently weakened.
2. **Replacement templates.** Rust implements its own interpreter for the current extended template dialect on top of match offsets, regardless of engine, verified against the 61 replacement cases.
3. **Empty-match progress.** `gmatch` and global `gsub` use the Perl/PCRE2 rule: after an empty match at p, retry at p for a non-empty match anchored there; if that fails, advance one whole UTF-8 character. Expected: global `gsub("x*","abc","-")` = `"-a-b-c-"`, 4; `(?=a)` on `"ba"` yields once; iteration over `"éa"` stops only on character boundaries. The agreed limited-`gsub` quirk remains separate.
4. **Unmatched captures.** An unmatched non-trailing group returns `nil` in its slot (`nil, nil` offsets from `cmatch`; `nil` from `match`, `find` and `gmatch`). Omission of unmatched trailing groups is retained.
5. **Out-of-range offsets.** An offset beyond length+1 raises an explicit error, not no-match. Valid, zero and negative offset normalization is unchanged.

Still open for the HITL resumption: engine choice, default work/output limit values and enforcement mechanism (informed by the spike), and any concrete incompatibility the spike surfaces. The claim was released so the spike follows the evidence frontier.

### Spike evidence available — still not resolved

[The completed fancy-regex assessment](../assets/fancy-regex-spike.md) records 155/160 candidate corpus passes, blocked JavaScript workloads, concrete class/escape/MARK/flag incompatibilities, prototype slowdown review triggers and incomplete general work enforcement. Its tested work/output values are options, not approved defaults. Resume the HITL choice from these concrete findings; do not infer an engine choice, permission to drop a feature, or a production implementation handoff. Further compatibility-layer/engine-change investigations, if chosen, need bounded tickets rather than silently expanding the disposable spike.

### Decisions after the fancy-regex spike — engine still not chosen

The user accepted:

1. **Conflict path.** Run a time-boxed AFK spike of a Rust pattern-translation layer over fancy-regex: [Spike a pattern-translation layer over fancy-regex](10-spike-fancy-regex-pattern-translation.md). Not a custom engine and not accepted losses at this point. If translation cannot reach every corpus check apart from the named losses, the conflict returns to the user.
2. **Named losses, conditional.** `(*MARK)` and `${*MARK}`, branch reset `(?|…)` and assertion conditionals such as `(?(?=a)ab|c)` (the survey found group-number conditionals already work) become documented, explicitly named losses (clear compile error) only if the translation spike confirms they are the only remaining incompatibilities. No bundled code uses them.
3. **Work-limit definition.** The native work limit is a per-execution backtrack limit plus a per-operation execution-count cap, bounding roughly executions × (backtracks + input length), and is documented as exactly that, not as a time limit. Default values are chosen after the translation spike, starting from the sweep (100,000 executions, 1 MiB output let the seven reachable workloads finish).
4. **Slowdowns.** The prototype's replacement and synthetic-transform slowdowns are a re-check item, not grounds to reject the engine: parse templates once per call and re-measure against the 25% review trigger.

Still open: engine choice, default limit values, and any difference the translation spike finds.

Before investing in that spike, the user asked for the other options to be checked: [Survey Rust-native regex engine options](11-survey-rust-regex-engines.md) blocks it.

### Resolution — lean native dialect after community audit

The user authorized dropping PCRE2 emulation if community damage appeared negligible. The bounded [audit and selected lean contract](../assets/regex-plugin-audit.md) supports proceeding: no removed flags found, extracted complete static patterns compile, and 167 EditorConfig tests pass on both engines. Unknown/dynamic user patterns and replacements are not certified.

Choose fancy-regex 0.19.2 with its native pattern and replacement dialects, private owning compiled handles, byte-offset API and corrected iterator behavior. No translator, custom replacement interpreter, engine fork or fallback. Preserve ANCHORED and the ENDANCHORED alias; reject other match option bits. Retain per-execution backtracking and output caps but abandon aggregate-work accounting; there is no deadline guarantee. Details/defaults and approved behavior changes are in the selected contract.

This explicitly supersedes earlier PCRE2 pattern/class/template preservation and aggregate-work requirements. Rust/Lua ownership remains unchanged. Remove pcre2-sys, the compiled binding module and static-build setting; fix the bundled JS class; validate current regression behavior, syntax failure handling, safe ownership/offsets and all reachable release workloads. The historical corpus remains evidence, not a requirement to emulate PCRE2. The translation ticket is superseded, not passed. Implementation is now user-authorized, rather than inferred from the original planning map.
