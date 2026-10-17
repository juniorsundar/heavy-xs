# Choose the regex compatibility envelope

Parent: ../map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-regex-compatibility
Blocked by: 01

## Question

Beyond the protected Lua-facing interface and current bundled functionality, what regex-language and observable-behavior compatibility must the replacement preserve?

Use [Inventory the observable regex contract](01-inventory-regex-contract.md) to separate:

- Lua interface compatibility: names, arguments, compiled objects, methods, return shapes and capture/offset conventions.
- Regex-language compatibility: accepted pattern syntax, matching semantics, flags and replacement syntax.
- Operational compatibility: malformed-input behavior, execution limits, errors and cancellation/failure handling.

Decide how existing user search expressions, installed snippet transformations and unknown third-party plugins are covered. Distinguish error wording from successful match/replacement semantics.

The user's confirmation means full PCRE2 parity is not assumed; it does not approve a specific lost feature. Any exclusions or semantic changes need the user's explicit decision, with concrete examples and consequences. Unsupported expressions must not silently mean something different.

Use the inventory's observed quirks and source-read risks to distinguish intended compatibility from existing bugs. Decide explicitly how to handle suffix-relative matching, empty-match progress, limited substitution and invalid-input/lifetime safety; do not adopt the observational probe wholesale as an acceptance oracle.

This ticket defines requirements, not the engine.

## Comments

### Discussion in progress

The user accepted the following first-round constraints:

- Define an explicit tested dialect covering bundled functionality and the sampled installed snippet-plugin consumer, not universal compatibility with unknown PCRE2 expressions. No specific feature loss has been approved; the plugin's actual expression corpus still needs evidence.
- Preserve failure conventions (compile returns nil plus a message; match/substitution errors raise), while allowing actionable diagnostic wording and engine-specific codes to change.
- Correct unsafe input/lifetime handling and non-advancing empty-match iteration rather than reproduce them. Specify the corrected observable behavior explicitly.
- Otherwise preserve defined successful quirks for the first replacement slice: suffix-relative cmatch, ENDANCHORED aliasing ANCHORED, ignored compile-flag letters and the existing positive-limit substitution behavior.

The user subsequently accepted the feature floor, Unicode/class semantics, replacement-language target and whole-match iterator correction below.

### Resolution

Confirmed by the user across both interview rounds. These are requirements and approved exceptions, not a claim that an engine satisfies them.

#### Scope and retained interface

- Define an explicit, tested regex dialect supporting current bundled functionality and representative use of the sampled installed snippet-plugin consumer. Do not promise universal parity with every PCRE2 expression used by unknown plugins. Actual snippet-expression coverage remains an evidence gap.
- Preserve the existing Lua-facing function/method interface, compiled-pattern table behavior, arguments, capture/offset conventions and successful return shapes unless an exception below applies. The raw native pointer is not a required future ownership mechanism.
- Unlisted PCRE2 constructs are **unclassified**, not approved for removal. Before adoption, any discovered unsupported construct or semantic divergence must be presented with a concrete expression, old/new behavior and affected consumer class for explicit approval. Unsupported syntax must not silently acquire another meaning. No candidate-specific syntax exclusion is approved here.

#### Required pattern-language floor

Require literals and escapes, character classes/ranges, alternation, anchors/word boundaries, greedy/lazy quantifiers; capturing, named, noncapturing and empty groups; positive/negative lookahead and fixed-length lookbehind; numeric/named backreferences; atomic groups and possessive quantifiers; and compile flags `i`, `m`, `s` plus their inline equivalents.

This includes bundled syntax needs as well as useful user-search constructs. It is a floor, not a complete list of permitted syntax or proof that expressions match with the correct semantics.

#### UTF-8 and offsets

Retain byte-indexed results with UTF-8 matching. For example, `.` on `é` returns `cmatch` positions `1, 3` and inclusive `find_offsets` positions `1, 2`. Invalid UTF-8 fails safely; do not repair it silently or reinterpret it as an implicit byte mode. Preserve existing default character-class behavior: UTF decoding does not automatically enable Unicode-property classes (`\w` does not match `é` under current defaults). No change to character indexing or arbitrary-byte mode is approved.

#### Replacement language

Retain the current replacement-template dialect as the target, not another engine's default grammar. Known required examples include `$1`/`$2`, `${name}`, `$$`, escaped literal backslashes and literal `%1` rather than Lua capture interpolation. Audit the untested extended forms before final design approval; their exact coverage is not established by this resolution. Any concrete incompatibility needs explicit approval. This does not choose Rust versus Lua ownership or add function/table replacements.

#### Failures and approved corrections

- Preserve failure conventions: compilation failure returns `nil` plus an actionable message; match/substitution errors raise rather than masquerade as no match. Diagnostic wording and engine-specific numbers need not reproduce PCRE2's catalogue. Provide useful pattern locations where available; exact message text/positions are not a byte-for-byte oracle.
- Correct unsafe input/lifetime handling rather than reproduce undefined behavior: invalid handles/offsets must not crash or access invalid memory, and an iterator must not depend on the caller retaining an otherwise discardable compiled-pattern table.
- Correct `gmatch` advancement to use the **whole match's end**, not the last capture's end, for consuming matches. This is an explicitly approved exception to old successful behavior.
- Specify and test UTF-8-safe empty-match iteration that does not emit the same zero-width position repeatedly and eventually terminates. The exact retry/advancement algorithm remains a design obligation, not an arbitrary behavior change authorized by this ticket. Safety/progress checks do not require reproducing a crash or hang in the old engine.

#### Other defined quirks stay

Preserve suffix-relative `cmatch` semantics (including anchors/lookbehind seeing the sliced subject), the exported `ENDANCHORED == ANCHORED` alias, ignored compile-flag letters and the existing positive-limit substitution semantics. In particular, `gsub("x*", "abc", "-", 2)` still yields `"--abc", 2`; the iterator correction does not normalize that separate substitution path.

#### Handoff and remaining evidence

[Assemble the regex compatibility acceptance corpus](07-assemble-regex-compatibility-corpus.md) closes test-coverage and replacement/snippet evidence gaps without selecting an engine. Any remaining policy choice exposed by that task must return for HITL resolution. [Assign ownership of regex execution and composition](04-assign-regex-operation-ownership.md) and [Select the Rust-native regex design](05-select-rust-native-regex-design.md) must make the accepted corrections concrete and testable before implementation.

Numeric runtime budgets belong to [Define regex performance acceptance criteria](06-define-regex-performance-criteria.md). No engine, operation placement, benchmark result, new customization feature or implementation change is selected by this resolution.

### Later user-approved lean override

The subsequent [lean decision](05-select-rust-native-regex-design.md) and [current dialect](../../regex.md) supersede the PCRE2-specific syntax, ASCII-class, extended replacement and removed-flag requirements above. The original resolution remains as historical reasoning, not the current dialect contract. Safe ownership, checked byte offsets and approved iterator corrections remain required.
