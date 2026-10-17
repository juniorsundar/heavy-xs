# Rust-native regex engine survey

Evidence for [Survey Rust-native regex engine options](../issues/11-survey-rust-regex-engines.md). It does not choose an engine.

**Finding:** fancy-regex is the closest pure-Rust engine by a wide margin. No other released crate runs the bundled JavaScript rule or the bundled backtracking constructs. Only two routes remain: adapt fancy-regex (by translating patterns, or by forking it) or write a dedicated engine.

## Candidates

| Candidate | Pure Rust | What it is | Verdict |
|---|---|---|---|
| fancy-regex 0.19.2 | Yes (MIT; regex-automata 0.4.18, regex-syntax 0.8.11, bit-set) | Hybrid: own parser and backtracking VM, delegates plain fragments to regex-automata. ~18,400 source lines. | Closest. Gaps listed below. |
| regress 0.12.0 | Yes (MIT/Apache-2.0; memchr, optional hashbrown) | ECMAScript 2018 engine, backtracking plus a PikeVM backend. ~45,000 lines including Unicode tables. | Out: rejects atomic groups, possessives, inline flags, `\A` and the JavaScript rule. |
| resharp 0.7.6 | Yes, by default (MIT; optional `pcre2` dependency) | Linear-time RE# engine with lookaround, intersection and complement. Leftmost-longest. ~15,200 lines. | Out: rejects backreferences, lazy quantifiers and atomic groups; possessive `a++` matches `aaa` with the wrong meaning. |
| regex / regex-automata / regex-lite | Yes | Linear-time finite automata; no lookaround or backreferences by design. | Building blocks only (fancy-regex already uses them). |
| onig, pcre2, hyperscan | No (C/C++ bindings) | — | Excluded by the Rust-native goal. |
| pomsky | Yes | Compiles its own language to other regex flavors; not an engine. | Excluded. |
| ecma-regex, sregex, Oniguruma ports | — | Stub crate, missing from docs.rs, or none found. | Not credible. |

Sources: crate sources as fetched by Cargo (versions above); [fancy-regex docs](https://docs.rs/fancy-regex/0.19.2/fancy_regex/), [syntax](https://github.com/fancy-regex/fancy-regex/blob/main/docs/syntax.md); [regress docs](https://docs.rs/regress/0.12.0/regress/); [resharp docs](https://docs.rs/resharp/0.7.6/resharp/), [syntax](https://github.com/ieviev/resharp/blob/main/docs/syntax.md).

**Correction to the research brief:** regress's `find_from_budgeted` step budget exists only on its unreleased main branch. The released 0.12.0 has no budget API, so released regress has no way to bound backtracking.

## Construct probe

[Probe source](regex-engine-survey-probe.rs) and [output](regex-engine-survey-probe.txt): 25 patterns, first match as a byte range on the subject, default options (regress with its `u` flag). The PCRE2 column is the target under UTF mode with UCP off. Fifteen of those values were observed on the reference build in the corpus or the [reference dialect run](fancy-regex-reference-dialect.txt). The `mark`, `branch_reset`, `conditional`, `posix` and lookbehind rows come from PCRE2 semantics and were not re-run here.

| Case | Pattern / subject | PCRE2 | fancy-regex | regress | resharp |
|---|---|---|---|---|---|
| atomic | `(?>a+)a` / `aaa` | none | none | error | error |
| possessive | `a++a` / `aaa` | none | none | error | **0..3 (wrong)** |
| lookahead, fixed/variable lookbehind | `a(?=b)`, `(?<=ab)c`, `(?<=a\|bc)d` | match | match | match | match |
| backref_num / backref_named | `(a)\1`, `(?<x>a)\k<x>` / `aa` | 0..2 | 0..2 | 0..2 | error |
| inline_flags | `(?i)ab` / `AB` | 0..2 | 0..2 | error | 0..2 |
| lazy | `a+?` / `aaa` | 0..1 | 0..1 | 0..1 | error |
| empty_capture | `a()b` / `ab` | 0..2 | 0..2 | 0..2 | 0..2 |
| word_ascii / digit_ascii / space_ascii | `\w` é, `\d` ٣, `\s` U+00A0 | none | **match ×3** | none, none, **match** | **match**, none, none |
| dot_utf8 / negclass_utf8 | `.`, `[^a]` / `é` | 0..2 | 0..2 | 0..2 | **0..1 (splits UTF-8)**, 0..2 |
| icase_utf8 | `(?i)é` / `É` | 0..2 | 0..2 | error | 0..2 |
| literal_bracket | `[^\\[/]+` / `x[y` | 0..1 | **error** | 0..1 | error |
| quote | `\Qa.b\E` | 0..3 | error | error | error |
| h_space | `\h` / space | 0..1 | **none** | error | error |
| abs_anchor | `\Aa` | 0..1 | 0..1 | error | 0..1 |
| posix | `[[:alpha:]]+` / `abé` | 0..2 | 0..2 | error | 0..2 |
| mark / branch_reset | `(*MARK:t)a`, `(?\|(a)\|(b))` | match | error | error | error |
| conditional | `(a)?(?(1)b\|c)` / `c` | 0..1 | 0..1 | error | error |
| **js_rule** | bundled [language_js.lua](../../../data/plugins/language_js.lua#L26) rule / `/ab+c/gi;` | 0..9 | **error (`[` in class)** | error | error |
| **Total matching PCRE2** | | 25 | **16** | 12 | 11 |

`\|` escapes the Markdown column separator; the real patterns use `|`. fancy-regex's `oniguruma_mode(true)` changes none of its failing rows. fancy-regex parses the group-number conditional `(?(1)…)`. Earlier raw probes showed it rejects the assertion conditional `(?(?=a)ab|c)`, so only that form, not conditionals generally, belongs with `(*MARK)` and branch reset as a possible named loss.

## Remaining routes

| Route | What closes the gaps | Work bound | Cost / risk |
|---|---|---|---|
| A. fancy-regex + translation layer ([ticket 10](../issues/10-spike-fancy-regex-pattern-translation.md)) | A Rust parser rewrites `\w \d \s \b` to ASCII forms, escapes `[` in classes, expands `\Q…\E` and `\h`/`\v`, maps NOTBOL/NOTEOL. No fork; the crate is upgraded as released. | Per-run backtrack limit plus the wrapper's run cap (agreed definition). No cancellation. | A second PCRE parser that must stay in step with fancy-regex's parser; double parsing at compile time (compilation is cached). |
| B. Fork fancy-regex | Same fixes in its parser, plus an operation-wide step counter or cancellation in `vm.rs`. | Can be truly shared across a whole operation. | Carry a patch set on ~18k lines and rebase it on each upstream release, or get the changes accepted upstream. |
| C. Dedicated engine | Anything, by design. | Anything, by design. | Comparable engines are 15–45k lines plus fuzzing: months of work for parity, against the "reduce dependencies" goal only marginally better than A/B. |

A and B are not exclusive: A can start now, and the parser changes could be offered upstream later; a fork is only needed if the agreed work-limit definition proves insufficient.

## Unknowns

- No engine was measured on the workloads here; only fancy-regex has workload data (first spike).
- Maintenance activity, release cadence and issue history were not verified (crates.io API blocked).
- Whether fancy-regex upstream would accept an ASCII-class mode, literal `[` in classes or `\Q…\E`.
