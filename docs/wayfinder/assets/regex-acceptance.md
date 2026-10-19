# Regex compatibility acceptance corpus

Evidence for [Assemble the regex compatibility acceptance corpus](../issues/07-assemble-regex-compatibility-corpus.md), against [the agreed compatibility envelope](../issues/03-choose-regex-compatibility.md). This is a bounded evidence corpus, not a replacement engine, performance result or proof of full PCRE2 parity.

## Assets and results

- [Runtime](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-acceptance.lua): small assertion runner using the existing `LITE_XL_RUNTIME` mechanism. Checks exact return count, Lua types and values; errors become failures. No third-party framework.
- [Pattern/interface/caller cases](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-acceptance-cases.lua): 94 assertions.
- [Replacement cases](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-replacement-cases.lua): 61 assertions, explained by [the replacement audit](regex-replacement-audit.md).
- [Reference results](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-acceptance-reference.txt): **157 cases, zero failures** (155 retained-behavior assertions plus two bounded observations of old iterator bugs).
- [Candidate-mode checks on the old reference](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-acceptance-old-candidate.txt): **160 cases, four expected failures**. The same 155 assertions pass; five correction checks replace the two observations. Four expose the still-unfixed iterator behavior. This is a negative control, not a passing candidate evaluation or a production fix.

Candidate mode does not automatically approve another engine's behavior. It replaces known-bug observations with approved corrections while keeping the same retained-behavior checks. The generic zero-width progress property passes on the old engine because it terminates prematurely at position 1 for that particular input; the other correction cases demonstrate non-progress/overlap. Property checks alone do not prove enumeration completeness.

## Reference identity and validation

Collected 2026-10-17. HEAD: `4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac`. The source checkout has no changes under `data/`, `rust/`, `.cargo/` or the existing contract baselines; the unrelated dirty retired-port document is not build input.

| Item | Identity |
|---|---|
| Binary | `rust/target/debug/lite-xl`, normal Cargo dev profile, rebuilt/verified with the project's Nix development environment |
| Binary SHA-256 | `3802ae8e0d56807b03966ca20e16a4bef7cd0ee301acf94976cd73ad666cd4b1` |
| `rust/Cargo.lock` SHA-256 | `af3836640e360db27e063f32ca76018d8cd42114de53845ecd4f1994ea1dfdc6` |
| `.cargo/config.toml` SHA-256 | `3c65cdb601fc68c37d91ca9f7b119e61d33acba519e358317dfeed92e7f5140f` |
| Native dependency | `pcre2-sys` 0.2.10; its build archives identify PCRE2 `10.46 2025-08-27`; static linking is forced by Cargo configuration |
| Runtime | Isolated temporary `LITE_USERDIR`/`LITE_OS_HOME`, dummy SDL video, staged current `data/`, no real dialogs or private user configuration |

The separate release-performance survey is not performed here. Existing `scripts/rust-port/verify.sh regex,tokenizer` passes after rebuilding; **no old baseline was rewritten**. Lua syntax checks pass for the three new runtime/case files. A temporary mutation adding an extra expected trailing `nil` to the first case produces exactly one failure and exit 1, confirming the runner checks return cardinality rather than Lua's unreliable `#` on sparse tables. The mutation is not in the assets.

## Coverage matrix

IDs identify executable cases, not tickets. Exact expressions, subjects, expected outputs and token strings live in the case files; results are linked above.

| Requirement | Cases / evidence | Limits |
|---|---|---|
| Native module name, compiled tables/metatable, function/method interface and return shapes | `IF_`; existing [regex suite](../../../tests/contract/cases/regex.lua) | Does not freeze a raw-pointer ownership mechanism; no forged handles. |
| Lookahead and fixed-length lookbehind, both polarities | `LA_` | Fixed examples; no variable-length lookbehind exclusion is approved. |
| Numeric/named backreferences | `BR_` | Synthetic expressions, not observed bundled use. |
| Atomic groups and possessive quantifiers | `AP_` | Semantic contrast checks backtracking versus failure, not merely compilation. |
| Compile and inline `i/m/s` | `FX_` | Other modifiers remain unclassified, not prohibited. |
| Ordinary operators, greedy/lazy repetition, classes, anchors/word boundaries, all capture types | `OP_` | Safe participating captures only; not an exhaustive syntax grammar. |
| UTF-8 byte spans, inclusive ends, default ASCII class behavior, invalid UTF-8 raises | `UT_` | No arbitrary-byte mode or Unicode-property default is introduced. |
| Suffix-relative `cmatch`, option values/ANCHORED/ENDANCHORED alias, ignored flag letters, limited/global substitution | `RQ_`; original [bounded probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-contract-probe.lua) | Existing negative-offset examples are retained by the old suite; unsafe upper offsets are not run. |
| Replacement captures, escapes, defaults/conditionals, case scope and additional linked-build forms | `RP_`, 61 replacement cases; [audit](regex-replacement-audit.md) | Current dialect is the preservation target; finite audit is not proof of the complete 10.46 grammar. |
| Failure conventions without frozen diagnostic text | `FL_`, replacement error cases | Compile returns nil plus a nonempty message; invalid arguments/replacements raise. Some old regression rows freeze PCRE2 messages and are historical evidence, not the replacement oracle. |
| Representative snippet compile/match/gsub flow | `SYN_`: filename transform, capture extraction, `i` flag, case-template transform | **Synthetic only.** No installed snippet-expression corpus was inspected or collected; no actual plugin parser/execution is claimed. |
| Bundled regex definitions in real tokenizer | `TOK_`: JavaScript regex/class/escaped slash and numeric division; C hex/float/octal; C++ binary suffix/class; Markdown math | Actual plugins loaded without suppressing require failures. Tokenization uses real `core.syntax`/`core.tokenizer`; only small representative inputs, not every rule/state. |
| Whole-match iterator progress | `OBS_iterator_last_capture_overlap` versus `FIX_iterator_whole_match_end` | Old `(a).*` on `aaa` yields three captures; corrected iteration must yield once. |
| Finite, non-repeating zero-width iteration and UTF-8 boundaries | `OBS_iterator_zero_width_repeats` versus four `FIX_iterator_*` checks | Every probe has a fixed call bound. Exact mixed empty/consuming retry policy is still a design obligation. |
| Existing regression coverage | Passing 49-row regex and 13-row tokenizer suites; [inventory](regex-contract.md) | Old tokenizer suite loads Lua syntax only. The new `TOK_` cases cover actual regex-bearing languages. |

The JavaScript class/escaped-slash case drives its inner regex syntax, while `AP_` establishes atomic/possessive semantics independently. The corpus bootstraps only the core thread/log containers needed for language-plugin warnings in a bare runtime; it does not call full `core.init` or measure real-window behavior. The reference Markdown math path can log a pre-existing malformed/empty-match pattern warning; exact token outcomes are captured, not a claim that all bundled rules are warning-free.

## Reproduce

From the repository root, enter the project's Nix development environment first (`nix develop`, or an equivalent freshly generated environment). Build/stage and verify the existing suites, then run the evidence runtime:

```sh
scripts/rust-port/verify.sh regex,tokenizer
root=$PWD
userdir=$(mktemp -d)
trap 'rm -rf "$userdir"' EXIT
cp docs/wayfinder/assets/regex-acceptance.lua \
   docs/wayfinder/assets/regex-acceptance-cases.lua \
   docs/wayfinder/assets/regex-replacement-cases.lua "$userdir/"
SDL_VIDEODRIVER=dummy \
DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/lxl-contract-bus \
SDL_FILE_DIALOG_DRIVER= \
LITE_USERDIR="$userdir" LITE_OS_HOME="$userdir" \
LITE_PREFIX="$root/rust/target/stage/usr/local" \
LITE_XL_RUNTIME=regex-acceptance \
timeout -k 2 30 "$root/rust/target/debug/lite-xl"
```

Default `REGEX_CORPUS_MODE=reference` must exit 0 with the linked reference results. Setting `REGEX_CORPUS_MODE=candidate` on that same old binary must exit 1 with the four documented `FIX_` failures. On a future candidate, candidate mode must pass **plus** the additional safety/design gates below. Neither output is written into old contract baselines. The external timeout is containment for this bounded survey, not evidence that a caller can interrupt native matching.

## Candidate-only safety and design gates — not executed on the old engine

The suite intentionally does **not** run any of these unsafe-reference scenarios. Before final design approval, specify observable outcomes and add isolated candidate checks after inspection establishes that they are safe to execute:

- Out-of-range positive offsets and malformed/forged compiled handles: validated rejection/safe failure, no invalid pointer access or crash. Preserve established valid negative/zero-offset normalization; do not silently redefine it as invalid.
- Iterator ownership: create an iterator from a compiled table, drop the caller's table and force GC, then consume it safely. Do not attempt this on the current raw-pointer implementation.
- Unmatched capture holes: choose/test a safe return convention with HITL approval before translating unset sentinels into Lua. Native `gsub` defaults/conditionals are separately safe and do not exercise the wrapper's offset conversion.
- Empty-match completeness/retry: decide cases mixing zero-width and consuming alternatives, Unicode advancement and EOF explicitly; passing generic progress properties is insufficient to settle those semantics.
- Work/output/memory budgets and failure handling: criteria remain with [Define regex performance acceptance criteria](../issues/06-define-regex-performance-criteria.md). No pathological unlimited work or engine resource-limit claim is made by these small probes.

These gates belong to [Assign ownership of regex execution and composition](../issues/04-assign-regex-operation-ownership.md) and [Select the Rust-native regex design](../issues/05-select-rust-native-regex-design.md). No missing fixture is permission to remove a feature, guess an unresolved return shape or silently reinterpret unsupported syntax. Candidate-specific differences still require concrete examples and explicit approval. The corpus is sufficient to start the bounded reference survey, not sufficient by itself to approve engine adoption.
