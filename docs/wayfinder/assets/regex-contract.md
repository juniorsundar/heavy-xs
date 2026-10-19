# Observable regex contract inventory

Evidence for [Inventory the observable regex contract](../issues/01-inventory-regex-contract.md).

Snapshot: repository HEAD `4c02ffc8`, with the existing documentation-only working changes. This inventories that historical implementation; it does **not** decide replacement requirements or endorse preserving bugs. The later [native dialect](../../regex.md) supersedes PCRE2-specific observations as the current contract.

## Historical source reference

Rust `regex.rs`/`pcre.rs` line labels below and in the replacement audit refer to revision `4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac`, not today's native wrapper. Read the exact sources without restoring removed bindings:

```sh
git show 4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac:rust/src/regex.rs
git show 4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac:rust/src/pcre.rs
```

Other relative source links locate files in the current checkout; use the same revision with `git show <revision>:<path>` for snapshot-accurate content and line numbers.

## Evidence and validation

- **Source-read:** native implementation, Lua wrappers, bundled callers/pattern definitions, contract tests and a sample installed snippet plugin.
- **Observed:** rebuilt the current debug binary and ran `scripts/rust-port/verify.sh regex,tokenizer` in the existing Nix development environment: both suites passed. No baselines were rewritten.
- **Observed:** [bounded probe](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-contract-probe.lua) completed with 20 observations, 17 asserted and three error messages recorded without exact-text assertions. [Recorded output](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-contract-probe.txt) is a new inventory asset, not a replacement oracle.
- **Not measured:** release performance, memory under load, JIT availability/success, adversarial-input budgets or cancellation latency. No alternative engine was researched or tested here.

## Native and Lua surface

The native module is loaded before editor startup ([app.rs:71](https://github.com/juniorsundar/heavy-xs/blob/d9b3db31/rust/src/app.rs#L71)). `require "core.regex"` adds convenience functions and method lookup to that same global table ([core/regex.lua:1–82](../../../data/core/regex.lua#L1)).

| Operation | Current observable behavior | Evidence |
|---|---|---|
| `compile(pattern, flags?)` | Returns one compiled table on success; `nil, message` on pattern compilation failure. Invalid argument types raise. Strings and numbers are accepted through string coercion. | [regex.rs:42–51, 174–209](regex-contract.md#historical-source-reference); existing suite |
| Compile flags | UTF mode is always set. Characters `i`, `m`, `s` enable caseless, multiline and dotall. Other characters are ignored. An omitted flags argument works; explicit `nil` raises. This is not a strict flag-string validator. | [regex.rs:182–196](regex-contract.md#historical-source-reference); probe |
| Compiled object | Plain Lua table; field `[1]` is light userdata (Lua reports `userdata`), metatable is the regex module, `__name` is `regex`. Lua's `__index` enables `pattern:gsub(...)`, `:cmatch(...)`, etc. | [regex.rs:107–112](regex-contract.md#historical-source-reference), [finalizer:496–507](regex-contract.md#historical-source-reference), [wrapper:1–3](../../../data/core/regex.lua#L1); existing suite |
| `cmatch(pattern, subject, offset?, options?)` | Returns whole-match start/end followed by participating captures' start/end pairs. Indices are 1-based **bytes**, with exclusive ends. No match returns **zero values**, not one explicit `nil`. | [regex.rs:211–274](regex-contract.md#historical-source-reference); existing suite, probe |
| `find_offsets(...)` | Compiles non-table patterns, delegates to `cmatch`, subtracts one from every end position. Ends are therefore **1-based inclusive**; they are not 0-based positions. Empty matches have end < start. | [wrapper:19–29](../../../data/core/regex.lua#L19); existing suite, probe |
| `find(...)` | Whole-match start/inclusive end, then capture strings. An empty capture becomes its numeric byte position. No match returns zero values. | [wrapper:67–82](../../../data/core/regex.lua#L67); existing suite |
| `match(...)` | Captures only when present; otherwise the whole-match substring. Empty captures become positions through `find`; an empty whole match without captures is `""`. | [wrapper:43–49](../../../data/core/regex.lua#L43); source-read |
| `gmatch(pattern, subject, offset?)` | Returns a stateful iterator. No capture groups: yields the whole match. With groups: yields groups only; empty captures become positions. Subject bytes and match data survive in iterator state. There is no match-options argument. Important progress/lifetime gaps are below. | [regex.rs:152–168, 277–364](regex-contract.md#historical-source-reference); existing suite, probe |
| `gsub(pattern, subject, replacement, limit?)` | Returns new string and substitution count. No match returns original bytes and `0`. Replacement must be string/number, not a function/table. Omitted/zero/negative limits mean global; positive limits select a separate iterative path. | [regex.rs:366–493](regex-contract.md#historical-source-reference); existing suite, probe |

Native `cmatch`, `gmatch` and `gsub` accept string patterns as well as compiled tables ([pattern_arg:83–103](regex-contract.md#historical-source-reference)). Inline compilation uses UTF mode without the optional compile flags. The old C crash on a string passed to `cmatch` is noted in the test source; it is not a behavior to reproduce. Normal bundled callers compile first or go through the Lua wrapper.

### Match options

Exported integer values are observable ([regex.rs:510–523](regex-contract.md#historical-source-reference); existing suite):

| Name | Value |
|---|---:|
| `ANCHORED` | 2147483648 |
| `ENDANCHORED` | 2147483648 |
| `NOTBOL` | 1 |
| `NOTEOL` | 2 |
| `NOTEMPTY` | 4 |
| `NOTEMPTY_ATSTART` | 8 |

`ENDANCHORED` deliberately aliases `ANCHORED`; it does not mean end anchoring. In bundled executable call sites, only `ANCHORED` is passed. `NOTBOL`/`NOTEMPTY` also appear in wrapper comments; that is not runtime use. Tests exercise `NOTBOL`, but mostly record the other constants' values, not their semantics.

## Offsets, Unicode and failures

- **Source-read:** offset defaults to 1; 0 and overly negative offsets map to 1; other negatives count from the byte length's end ([regex.rs:115–125](regex-contract.md#historical-source-reference)). These are byte positions, not codepoint positions.
- **Observed:** matching `.` against `é` produces `cmatch` positions `1, 3` and inclusive `find_offsets` positions `1, 2`.
- **Observed:** UTF mode rejects subject byte `0xff`, raising `regex matching error -23: UTF-8 error: illegal byte (0xfe or 0xff)`. Do not infer arbitrary-byte acceptance merely because Lua strings hold bytes.
- **Observed/source-read:** `\w` does not match `é` under default flags. The wrapper enables `PCRE2_UTF`, not `PCRE2_UCP`; Unicode decoding does not imply Unicode-property semantics for every character class ([pcre.rs:20–35](regex-contract.md#historical-source-reference)).
- **Observed:** `cmatch` passes a suffix as a new subject, whereas `gmatch` passes the full subject and a start offset. Thus `cmatch("^a", "ba", 2)` finds positions `2, 3`, but `gmatch("^a", "ba", 2)` finds nothing. Lookbehind `(?<=b)a` at the same offset sees no prefix in `cmatch`, but does in `gmatch`. The probe uses a compiled pattern for `cmatch`.
- **Observed:** unmatched *trailing* capture `(b)?` is omitted from `cmatch` results for `(a)(b)?` on `"a"`. Handling of unmatched groups before a participating later group is not established by this probe.
- **Existing suite:** compilation failure for `(` returns `nil` plus `regex compilation failed at offset 1: missing closing parenthesis`. Inline pattern errors raise instead ([regex.rs:94–103](regex-contract.md#historical-source-reference)). Match and substitute failures raise engine-derived messages ([matching_error](regex-contract.md#historical-source-reference), [substitution error](regex-contract.md#historical-source-reference)).
- **Existing suite limitation:** error comparisons strip mlua's `runtime error: ` prefix and traceback ([tests/regex.lua:9–14](../../../tests/contract/cases/regex.lua#L9)). Passing that suite does not prove exact Lua error-object/type/traceback compatibility.

## Iteration and replacement semantics

**Source-read:** `gmatch` allocates match data once and uses the pattern's total capture count, not just the current match's participating captures. It advances to the end of the **last yielded capture**, not necessarily the end of the whole match ([regex.rs:332–360](regex-contract.md#historical-source-reference)). There is no general empty-match character-advancement step.

**Observed:** calling `regex.gmatch("(?=a)", "ba")` twice yields position `2` both times. A generic iteration over this expression would not make progress. This was tested with exactly two calls, not an unbounded loop. This observation is a compatibility/safety question, not a proposed requirement to keep the bug.

**Source-read:** substitution delegates replacement interpretation to `pcre2_substitute_8` with `SUBSTITUTE_EXTENDED | SUBSTITUTE_OVERFLOW_LENGTH`. The global path adds `SUBSTITUTE_GLOBAL`; it is not a Lua replacement parser ([regex.rs:381–493](regex-contract.md#historical-source-reference)).

- **Existing suite:** `$2$1` capture substitution; escaped literal backslashes; multiline `^`; global empty matches (`x*` on `abc` → `-a-b-c-`, count 4); growing output and positive limits.
- **Observed:** `${letter}` named capture substitution works; `$$$1` produces a literal dollar followed by capture 1; Lua `%1` is literal replacement text, not capture interpolation.
- **Observed:** negative limits mean global. A function replacement raises `string expected, got function`.
- **Observed quirk:** `gsub("x*", "abc", "-", 2)` produces `--abc`, count 2, unlike simply truncating the global path's `-a-b-c-` after its first two original-input positions. The positive-limit path substitutes into a newly changed subject and recomputes offsets.
- **Not exhaustively tested:** replacement escapes such as case conversion, unmatched/named/missing capture errors, embedded NULs, positive limits with context-sensitive patterns, and all combinations of zero-width/Unicode matching.

## Bundled consumers and constructs

Most syntax definitions use Lua patterns (`pattern =`), not PCRE2 (`regex =`). A textual search for `*+` inside a Lua pattern can mean a repeated literal `*`, not a possessive regex quantifier. Pattern dialect must be retained in the inventory.

| Consumer | Responsibility and concrete dependency | Evidence |
|---|---|---|
| Syntax validation | Compile regex entries and reject ones matching the empty subject. This is only a check against `""`, not proof that a pattern can never match an empty region of other text. | [syntax.lua:10–29](../../../data/core/syntax.lua#L10) |
| Tokenizer | Repeated per-token matching; scalar regex patterns cached in `p._regex`. Start/end-table patterns take a different path and can reach `regex.find` as strings, so caching is not universal. `ANCHORED`, empty offset captures and byte-to-character conversion matter. | [tokenizer.lua:202–247](../../../data/core/tokenizer.lua#L202) |
| C/C++ syntax | Noncapturing groups, `\d`, `\b`, alternation, numeric suffix/float rules and empty `()` boundaries. | [language_c.lua:4–24](../../../data/plugins/language_c.lua#L4), [language_cpp.lua:4–29](../../../data/plugins/language_cpp.lua#L4) |
| JavaScript syntax | Positive/negative lookahead, noncapturing groups, **atomic groups `(?>...)` and possessive `++`/`*+`**, escapes/classes and empty `()` boundaries. Comments explicitly associate atomicity with avoiding catastrophic backtracking. | [language_js.lua:4–38](../../../data/plugins/language_js.lua#L4), [numeric rules:67–69](../../../data/plugins/language_js.lua#L67) |
| Markdown syntax | Inline math end rule uses lookahead for an escaped-newline boundary. | [language_md.lua:94](../../../data/plugins/language_md.lua#L94) |
| Document search | User-entered expressions; optional `i`. Compiles at search entry, scans lines, converts exclusive end offsets to document positions. Direction, wrapping and iteration are Lua policy. | [doc/search.lua:25–86](../../../data/core/doc/search.lua#L25) |
| Find/replace | User-entered pattern/replacement; compiles with `m`, calls `regex.gsub`. Regex replacement is not Lua `string.gsub` replacement syntax. | [findreplace.lua:208–214](../../../data/core/commands/findreplace.lua#L208) |
| Project search | User expressions, optional `i`, one compile per search; `cmatch` in per-line callback. | [projectsearch.lua:283–295](../../../data/plugins/projectsearch.lua#L283) |
| Indentation detection | Derives comment/string boundaries from syntax rules; compiles regex starts/ends, prefixes comment patterns with `^\s*`, uses anchored line matching. It is coupled to the syntax-rule dialect. | [detectindent.lua:95–131](../../../data/plugins/detectindent.lua#L95), [matching:218–242](../../../data/plugins/detectindent.lua#L218) |
| Plugin headers | Two fixed compiled expressions for mod-version and priority; captures/noncapturing optional groups, `\d`, `\s`, alternation. Compiled once; matched per header line via `pcall`. | [init.lua:514–564](../../../data/core/init.lua#L514) |

Bundled patterns establish a required feature floor for current functionality, not the complete set of expressions users and plugins can supply. No bundled backreference or lookbehind requirement was found in these regex definitions; their absence is not permission to remove support from user search.

## Installed third-party sample (not bundled)

Source-read only: `~/.config/lite-xl/plugins/lsp_snippets.lua:11, 141–176` requires `regex`, compiles snippet-provided patterns/flags, invokes `reg:gsub(str, replacement)` or `reg:match(str)`, and treats string captures differently from numeric empty-capture positions.

This establishes a real plugin-side consumer of methods, replacement syntax and capture shapes. No actual snippet corpus was executed, no claim is made about all installed plugins, and the sample file/version is not a repository fixture. Unknown third-party expressions remain an explicit compatibility-envelope question.

## Coverage and unresolved risks

The [regex suite](../../../tests/contract/cases/regex.lua) has 47 `rec` calls: compiled-table shape/methods, compile errors, `i`/`m`, offsets/negative offsets, `NOTBOL`, ordinary captures, some iteration, substitution limits/growth and exported constants. Its output file has 49 physical lines (including header and a multiline value); that is not 49 independent checks.

The [tokenizer suite](../../../tests/contract/cases/tokenizer.lua#L5) loads **Lua syntax only**. Its 13-line baseline covers tokenizer output/state for Lua strings/comments/numbers, not JavaScript regex literals or C/C++ numeric regex rules. Its “regex heavy” introductory comment is misleading. No production test or comment was changed by this inventory.

The probe adds bounded examples for Unicode, dotall, unknown flags, suffix-versus-start-offset behavior, repeated empty matches, named/dollar replacements and atomic/possessive syntax. It is not a complete PCRE2 dialect corpus, and its observations of quirks are not an approved replacement spec.

Source-read gaps to carry into compatibility and acceptance decisions:

- **Limits:** compilation requests JIT but ignores its return value; matching calls `pcre2_match_8`. Compile/match contexts are null; this wrapper sets no application-specific work/depth/heap budget or cancellation hook. This does not mean PCRE2 has no internal defaults ([regex.rs:62–79, 242–250](regex-contract.md#historical-source-reference)).
- **Invalid inputs:** offsets beyond the subject, offsets into multibyte sequences, malformed compiled tables and every invalid UTF case are not a validated safe-input matrix. In particular, `cmatch` forms a pointer at the requested offset without a corresponding upper-bound check before FFI; out-of-range cases were not executed.
- **Unmatched groups:** offset construction does not explicitly translate PCRE2's unset-capture sentinel; `gmatch` traverses all pattern captures. Interior unmatched captures need isolated safety tests, not an assumption that they become empty strings.
- **Lifetime:** iterator state retains subject bytes and a raw pattern pointer; when passed a compiled table, the iterator does not visibly retain that table. GC/finalizer ownership and iterator survival after the caller drops its pattern were not exercised. Do not treat the raw-pointer representation as a required future implementation.
- **Performance:** source call frequency is not measured latency or allocation cost. No evidence here settles whether iteration/substitution should move to Lua.

These are facts/gaps for the existing compatibility and design tickets, not additional decisions resolved by this task.

## Reproduce the bounded observations

From the repository root, after building/staging with the verification script:

```sh
scripts/rust-port/verify.sh regex,tokenizer
userdir=$(mktemp -d)
cp docs/wayfinder/assets/regex-contract-probe.lua "$userdir/regex-contract-probe.lua"
SDL_VIDEODRIVER=dummy LITE_USERDIR="$userdir" \
  LITE_PREFIX="$PWD/rust/target/stage/usr/local" \
  LITE_XL_RUNTIME=regex-contract-probe \
  timeout -k 2 15 rust/target/debug/lite-xl
rc=$?
rm -rf "$userdir"
test "$rc" = 0
```

Use the project's Nix development shell for the build prerequisites. The executed verification run sourced the already-generated `/tmp/devenv.sh`. Startup replaces Lua's package paths, so copying the runtime into a temporary user directory is intentional; setting `LUA_PATH` alone did not load it. Neither the user's real configuration nor the contract baselines are modified.
