# Regex dialect

This Rust/Linux branch uses **fancy-regex 0.19.2**, not PCRE2. This deliberately changes the pattern and replacement languages; it is not a PCRE2 compatibility layer. See the [native pattern syntax](https://docs.rs/fancy-regex/0.19.2/fancy_regex/) and [native replacement expander](https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.Expander.html).

## Patterns and replacements

Lookaround, backreferences, atomic groups and possessive quantifiers remain available. Default `\w`, `\d`, `\s` and word boundaries are Unicode-aware. POSIX classes retain the native engine's semantics. Escape literal `[` inside a character class (`\[`); the bundled JavaScript rule has been updated accordingly.

PCRE2-specific escapes are not translated. For example, `\Q…\E` is unsupported, and **`\h` means a hexadecimal digit**, not PCRE2 horizontal whitespace. Existing user search expressions, syntax plugins and snippet transforms may need adapting even when a pattern compiles successfully. Syntax rules that fail to compile are disabled with a warning, not silently rewritten.

Replacement syntax:

| Template | Meaning |
|---|---|
| `$0` | Whole match |
| `$1`, `${1}` | Numbered capture |
| `$name`, `${name}` | Named capture; braces disambiguate adjacent text |
| `$$` | Literal dollar |

Backslashes are literal. `\n` is two characters, not a newline; `\U$1\E` does not uppercase a capture. PCRE2 default/conditional templates, case conversion, `$&`, `$'`, `$\`` and other extended forms are not emulated. Unknown capture references expand to empty; malformed templates follow the native expander rather than raising PCRE2 template errors. Check important saved replacements before applying them to a document.

## Lua interface

`compile`, `cmatch`, `find_offsets`, `find`, `match`, `gmatch` and `gsub`, including compiled-pattern methods, keep their signatures. `require "core.regex"` installs the Lua helpers. Compiled patterns remain tables, but their handle is private owning userdata; do not inspect or forge its contents.

- `compile(pattern, flags)` returns a compiled table or `nil, message`. Compile flags `i`, `m`, `s` remain supported; unrelated flag letters are ignored.
- Offsets are 1-based **byte positions**, not Unicode character indices. `cmatch` returns exclusive ends; `find_offsets`/`find` return inclusive ends. Invalid UTF-8 and offsets inside a codepoint or past length+1 raise errors.
- `cmatch` rebases the subject at its offset (anchors/lookbehind see the suffix); `gmatch`/global `gsub` keep whole-subject context.
- `ANCHORED` remains available. `ENDANCHORED` remains its historical alias, not an end-of-subject constraint. `NOTBOL`, `NOTEOL`, `NOTEMPTY` and `NOTEMPTY_ATSTART` are removed; unsupported numeric option bits raise `REGEX_OPTIONS_ERROR`.
- An unmatched middle capture yields `nil` (two nil offsets from `cmatch`); trailing unmatched groups are omitted. A nil first capture also stops a Lua generic-for iterator; consume the iterator directly if that case matters.
- After an empty iterator/global-substitution match, retry there for a nonempty match, otherwise advance a complete UTF-8 character. EOF is yielded at most once. Positive-limit `gsub` keeps its historical repeated-substitution behavior: `gsub("x*", "abc", "-", 2)` produces `"--abc", 2`; global substitution produces `"-a-b-c-", 4`.

## Resource limits

`regex.set_limits(backtracks, output_bytes)` sets positive module-wide defaults for subsequent operations (including previously compiled patterns). Iterators snapshot the creation-time values. Defaults are **1,000,000 backtracks per native execution** and **8 MiB substitution output**. `REGEX_WORK_LIMIT` or `REGEX_OUTPUT_LIMIT` raises on exhaustion, never a no-match or partially returned replacement. Expansion streams into a capped writer instead of allocating an unlimited temporary string. The cap includes unchanged text: regex replace-all whose result exceeds 8 MiB raises even if there are no matches. Increase the configurable cap before replacing in large documents; this is not a file-opening limit. A failed whole-document replacement leaves its text unchanged. Replacement across several selections is not a transaction: earlier selections may already have changed if a later one fails (undo is available).

This is **not a deadline, total-operation budget or sandbox**. Delegated scans are outside the backtrack counter, compilation has separate approximate size settings, and substitutions may run many individually bounded matches. Positive-limit substitution can repeatedly rebuild its subject. Long inputs or patterns can still block the editor. The output cap bounds result bytes, not total process memory; Lua, input copies, caches and allocator overhead also consume memory.

The accepted cached-edit and synthetic-transform slowdowns have not been profiled; the requested follow-up handoff was never written.

Historical PCRE2 compatibility evidence is retained under [the architecture repository](https://github.com/juniorsundar/heavy-xs-architecture/blob/main/wayfinder/assets/); it is not the current dialect specification.
