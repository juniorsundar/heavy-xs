# Survey Rust-native regex engine options

Parent: ../map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-regex-engine-survey
Blocked by: 09

## Question

Which Rust-native regex engines, or engine-building routes, could meet the agreed requirements, and how do they compare with fancy-regex before more is invested in [Spike a pattern-translation layer over fancy-regex](10-spike-fancy-regex-pattern-translation.md)?

fancy-regex was selected for the [first spike](09-spike-fancy-regex.md) as the only *known* pure-Rust engine with atomic groups, possessive quantifiers, lookbehind and backreferences; no broader survey was recorded. Close that gap with primary sources (crate docs, source, issue trackers) and small host probes, not reputation.

Compare every credible candidate, including at least fancy-regex, regress, resharp, the `regex`/`regex-automata` crates as building blocks, forking or patching fancy-regex, and writing a dedicated backtracking engine, on:

- the bundled constructs: atomic groups, possessive quantifiers, lookahead, fixed and variable lookbehind, numbered and named backreferences, inline flags, empty captures for offsets, and the actual JavaScript rule in [language_js.lua](../../../data/plugins/language_js.lua#L26);
- the dialect gaps found by the first spike: PCRE2 UCP-off ASCII classes with UTF-8-aware `.`/negated classes/case folding, a literal `[` inside a class, `\Q…\E`, `\h`/`\v`/`\R`, POSIX classes, NOTBOL/NOTEOL with absolute `\A`/`\z`, and the conditionally named losses (`(*MARK)`, branch reset, conditionals);
- execution from a byte offset with byte-offset captures, unmatched-group representation and UTF-8 handling;
- bounding work: backtrack/step limits, cancellation or deadlines, worst-case complexity, memory limits;
- pure Rust (no C), dependencies, licence, maintenance activity and size of the change to reach the corpus.

Deliver a comparison table under `docs/wayfinder/assets/` with sources, probe results and unknowns. Do not choose the engine; that stays with [Select the Rust-native regex design](05-select-rust-native-regex-design.md).

## Comments

### Resolution

[The survey](../assets/regex-engine-survey.md) compared every credible released crate with sources and a 25-pattern host probe. fancy-regex matches PCRE2 on 16 of 25, regress on 12, resharp on 11. regress (no atomic groups, possessives, inline flags or `\A`, and no released step budget) and resharp (no backreferences or lazy quantifiers, wrong possessive meaning) are out. No released crate runs the bundled JavaScript rule. The remaining routes are fancy-regex with a translation layer, a fancy-regex fork, or a dedicated engine (15–45k lines by comparison). The survey also narrows the conditional named loss to assertion conditionals. The [translation spike](10-spike-fancy-regex-pattern-translation.md) is unblocked; the choice between routes stays with the design ticket.
