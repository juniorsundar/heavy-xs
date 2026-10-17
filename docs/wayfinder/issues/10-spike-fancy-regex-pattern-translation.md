# Spike a pattern-translation layer over fancy-regex

Parent: ../map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: none
Blocked by: 09, 11

## Question

Can a Rust layer that rewrites PCRE2-dialect patterns before fancy-regex compiles them bring the [fancy-regex spike](09-spike-fancy-regex.md) candidate to every corpus check except the named losses, without breaking the agreed limits or performance criteria?

Time-boxed, on the disposable `spike/fancy-regex` branch (commit `effe6189bdcfb05318bb1139982d2633edcfa49b`); never merged, no production change on the main line. Extend the existing spike adapter only:

- Translate patterns inside Rust, at compile time, in one place. Lua callers, bundled syntax files and third-party patterns are not edited. Cover at least: PCRE2 UCP-off classes (`\w \d \s \b` and negations, POSIX classes) as their ASCII forms while `.`, negated classes and case folding stay UTF-8 aware; an unescaped `[` inside a class as a literal; `\Q…\E` as escaped literals; `\h`/`\H`/`\v`/`\V` as their PCRE2 sets; and the NOTBOL/NOTEOL mapping that leaves `\A`/`\z` absolute.
- The translator must be a real parser of the PCRE2 surface it handles (escapes, classes, groups, comments, `(?x)` if supported), not ad-hoc substring replacement. Anything it cannot translate faithfully is rejected with a compile error naming the construct, never passed through with different meaning.
- Named losses, approved conditionally on this spike: `(*MARK)` and `${*MARK}`, branch reset `(?|…)` and assertion conditionals such as `(?(?=a)ab|c)` (group-number conditionals already work, see the [engine survey](../assets/regex-engine-survey.md)) raise a clear compile error. The `ext_mark_named` corpus check is the only expected failure; if any other construct cannot be supported, stop and report it for a decision instead of adding it to the losses.
- Parse replacement templates once per `gsub` call, not per match, then re-measure.
- Work limit as agreed: per execution backtrack limit plus a per-operation execution-count cap, documented as bounding roughly executions × (backtracks + input length), not time.

Measure, against the same reference and protocol as the previous spike:

- the acceptance corpus in candidate mode, and the dialect, safety and limits probes, with the JavaScript tokenizer checks proving token-for-token equality with the reference;
- all twelve release workloads (the five JavaScript highlighting ones now included), against the 33.3 ms segment, 64 MiB, retention and 25% slowdown-review criteria;
- the limit sweep across every workload, to give default options, including the stress classes;
- translator unit tests per construct, including rejection cases, and one bounded fuzz/differential check against PCRE2 on generated ASCII/UTF-8 patterns from the supported surface (safe inputs only, never the raw-pointer unsafe probes).

Deliver an updated report under `docs/wayfinder/assets/`: pass/fail matrix, every remaining difference as a concrete pattern with old versus new behavior and affected consumers, performance against each criterion, sweep-derived default options and a source patch. Do not choose the engine, add losses beyond the named ones, or set the defaults; those return to [Select the Rust-native regex design](05-select-rust-native-regex-design.md).

## Comments

### Resolution — superseded, not executed

After the bounded [community plugin audit](../assets/regex-plugin-audit.md), the user authorized the lean native-dialect integration. This translator spike is cancelled/superseded; none of its acceptance claims is marked as achieved. No translator or compatibility interpreter will ship.
