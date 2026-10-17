# Replacement-template audit for the reference engine

Evidence for [Assemble the regex compatibility acceptance corpus](../issues/07-assemble-regex-compatibility-corpus.md). The [corpus report](regex-acceptance.md) records the build identity, runner, results and gaps; [replacement fixtures](regex-replacement-cases.lua) contain the concrete examples.

This is a finite audit, not an exhaustive grammar specification or engine choice. [Choose the regex compatibility envelope](../issues/03-choose-regex-compatibility.md) targets preservation of the **current** replacement dialect. No version-specific forms are implicitly excluded; any candidate incompatibility still needs explicit approval.

## Wrapper and version evidence

[The native gsub implementation](regex-contract.md#historical-source-reference) calls `pcre2_substitute_8` with `SUBSTITUTE_EXTENDED | SUBSTITUTE_OVERFLOW_LENGTH`, adding `GLOBAL` when the normalized limit is zero. It does not set `LITERAL`, `UNKNOWN_UNSET`, `UNSET_EMPTY`, `MATCHED` or `REPLACEMENT_ONLY`. Thus unknown/unset plain capture references raise, and Lua `%1` is literal. Replacement arguments go through string coercion; function/table replacements are rejected rather than invoked. Function rejection is covered by the acceptance cases; table rejection is source-read only.

Two different PCRE2 versions matter:

| Evidence | Finding | Limitation |
|---|---|---|
| Optional repository-local `subprojects/pcre2-10.44/doc/pcre2api.3` and `src/pcre2_substitute.c` | Old C-build source/manual, version 10.44 | This leftover tree is untracked and is not the current Cargo dependency; it is not needed to rerun fixtures. |
| [Cargo.lock](../../../rust/Cargo.lock#L225) | `pcre2-sys` 0.2.10 | Crate version alone is not a PCRE2 runtime-version query. |
| Build archives `rust/target/debug/build/pcre2-sys-*/out/libpcre2.a` | Embedded string `10.46 2025-08-27` | Build-artifact evidence, not a Lua-exposed runtime version API. |
| [Cargo configuration](../../../.cargo/config.toml#L4) | Forces static PCRE2 | Together with the fresh Cargo verification/build and binary hash in the report, identifies the reference build without relying on the old 10.44 tree. |

The local 10.44 manual/source supports the first groups below; their outcomes are also asserted against the rebuilt reference. Additional linked-build forms are **runtime observations**, not historical claims about which release introduced them. No external/current-version manual is needed to reproduce those observations, and no unverified source attribution is used as an oracle.

## Audited forms

### Local 10.44 documentation/source plus current runtime observations

The optional local `pcre2api.3` substitution section and `pcre2_substitute.c` describe:

- Dollar references: `$0` for the whole match, `$1`, `${1}`, `$name`, `${name}`; `$$` for a literal dollar; `$*MARK` / `${*MARK}` for the most recent MARK name (empty when absent). Here `name` is a placeholder, not angle-bracket syntax.
- Extended defaults/conditionals: `${2:-D}` and `${2:+S:U}`, selecting according to whether a capture participated. These run inside native substitution, not the wrapper's unsafe unmatched-capture offset conversion.
- Escapes: `\n`, `\t`, `\x41`, `\x{41}`, `\o{101}`, `\cA`; escaped non-alphanumeric literals such as `\/`, `\{` and `\\`; `\Q…\E` quoting.
- Case forcing: `\U`, `\L`, `\u`, `\l` and `\E`, including persistent scope, one-character scope and resetting. The cases cover ASCII, not complete Unicode casing.
- Errors for malformed `$`/brace syntax, unknown references, an unset capture used without a default, unsupported extended operators and invalid escapes. Error fixtures assert failure, not diagnostic text/codes.

Every concrete claim above has a fixture in the groups below; deeper nesting and grammar limits remain unverified. The local source hashes for this audit are:

```text
pcre2api.3         bc1e6c43a65b771146005c727356e6a17a254a23a62de5b6562360140f2aa916
pcre2_substitute.c 4ed0fc5a68a8e7370c0f69e34f4d3cfdf41168dbca61832b3d8b80d633165351
```

### Additional current-build observations

The `linked_` fixtures verify forms not established by that local 10.44 documentation:

- `$&` whole match; `` $` `` prefix; `$'` suffix; `$_` entire subject.
- `$<name>` named reference.
- `\1`, `\2`, `\g<name>` and `\g<1>` references; the tested `\g{name}` form raises.
- `\b` backspace and `\v` vertical tab.
- `\l\U…` and `\u\L…` combinations.

These observations extend the known replacement surface of the **current-dialect preservation target**. They do not establish a complete grammar, approve a new feature, or authorize dropping these forms because the old manual lacks them. In particular, preserving the old C-build 10.44 dialect alone would not demonstrate preservation of the current editor.

## Fixture index

All 61 replacement cases pass in reference mode. Each returns exact replacement text/count or an explicit error boolean; see [the runner](regex-acceptance.lua).

| Group | Count | Fixture IDs / prefixes |
|---|---:|---|
| Dollar forms | 7 | `dollar_` |
| Defaults/conditionals and MARK | 6 | `ext_` |
| Backslash escapes/quoting | 11 | `bs_` |
| Case scope/reset | 8 | `case_` |
| Literal Lua replacement notation | 2 | `lua_` |
| Malformed/unset references raise | 7 | `bad_` |
| Limit/global/no-match behavior | 6 | `limit_`, `global_empty_match`, `no_match_returns_input` |
| Additional linked-build observations | 14 | `linked_` |

The retained positive-limit quirk `gsub("x*", "abc", "-", 2)` yields `"--abc", 2`, separately from global substitution yielding `"-a-b-c-", 4`. The iterator progress correction does not normalize this path.

## Remaining gaps and approval gate

- No exhaustive version-matched 10.46 grammar audit: nested defaults, duplicate/nonnumeric name edge cases, Unicode case conversion/expansion, grammar bounds and additional context-sensitive substitution behavior are not fully covered. If a candidate comparison depends on one of these facts, obtain version-matched primary-source evidence or focused safe observations before accepting a difference.
- Positive-limit substitution changes the working subject between replacements. Only the bounded limit fixtures establish concrete outcomes; they do not prove compatibility for every anchor/lookbehind/context-sensitive expression.
- The wrapper does not expose configurable substitute callbacks; this audit does not propose adding the PCRE2 case-callout interface.
- None of the gaps is an approved exclusion. A candidate-specific incompatibility must return with a concrete example, old/new outcome and affected consumer for explicit approval. This audit is evidence for that review, not permission to invent missing semantics.
