# Community plugin audit and lean decision

User decision: check the community plugin repository, then use a lean fancy-regex integration if the likely damage is negligible. The bounded checks below support proceeding, **not** a promise of universal plugin compatibility.

## Scope and observations

- Community repository: https://github.com/lite-xl/lite-xl-plugins at `6e4aa640d679decd67d2c0b677ede52ef70a7bf9` (2026-10-12); 190 Lua files.
- Its manifest contains 63 remote entries, resolving to 56 distinct repositories. All 56 were fetched at their default-branch HEADs (not the manifest-pinned commits), totaling another 286 Lua files. This is a snapshot of available sources, not a package-version compatibility certification.
- The [static extractor](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-plugin-audit-scan.lua) found 56 pattern literals/fragments: 54 compile with unmodified fancy-regex 0.19.2, while two PHP SQL prefixes are concatenated expressions, not complete patterns. Reconstructing both with their actual eight SQL keywords makes them compile too. The scanner includes comments, does not interpret arbitrary Lua, and is not exhaustive for aliases or dynamically assembled patterns.
- No uses of `NOTBOL`, `NOTEOL`, `NOTEMPTY`, `NOTEMPTY_ATSTART` or `ENDANCHORED` were found in this source scan.
- EditorConfig's generated patterns were exercised through its own 167 tests: PCRE2 reference **167 passed / 0 failed**; the existing fancy-regex prototype **167 passed / 0 failed**. These use no replacement templates, so the prototype's custom replacement interpreter is irrelevant to that result.
- `search_ui` forwards user-entered replacement text, so it inherits the intentional replacement-dialect change. `regexreplacepreview` implements its own backslash-number replacements. `indent_convert`'s native `gsub` example is only a comment, not an active dependency.
- `lsp_snippets` forwards plain text to `reg:gsub` and implements structured snippet transforms/case handling itself. This does not prove every user-authored transform remains compatible. User-entered patterns, saved snippets, Tree-sitter query predicates and other dynamically supplied expressions remain unbounded evidence gaps.

The [extracted patterns](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-plugin-audit-patterns.tsv) are supporting scanner output, not an executed compatibility corpus. Compilation alone does not establish identical matches: Unicode character classes, native escape meanings and replacement expansion intentionally change. No all-plugin runtime or private-snippet audit was performed.

## Approved lean contract

This supersedes the earlier PCRE2-fidelity requirements and the unexecuted pattern-translation spike:

- Native **fancy-regex 0.19.2 pattern syntax**, including Unicode-aware default classes. No translator, fork or PCRE2 fallback. Escape the literal `[` in the bundled JavaScript rule.
- Native dollar replacement expansion (`$0`, `$1`, `${name}`, `$name`, `$$`). Backslashes are literal; PCRE2 case conversion, replacement escapes and conditional/default templates are not emulated. Unknown references follow the native expander (usually empty), not PCRE2 errors.
- Keep Lua function/method signatures, compiled tables backed by private owning userdata, byte offsets, invalid-input validation, middle capture holes/trailing omission and UTF-8-safe empty-match iteration. Keep the approved positive-limit substitution quirk separately.
- Only `ANCHORED` (and its historical `ENDANCHORED` alias) remains supported; removed flag constants are absent and unsupported option bits raise an explicit error.
- Native per-execution backtrack limit: 1,000,000 by default. Substitution output cap: 8 MiB by default. `regex.set_limits(backtrack, output_bytes)` changes defaults for subsequent operations. Iterators retain their creation-time setting. No aggregate execution counter, deadline or universal linear-time guarantee is claimed. Delegated scans and repeated matches still cost time proportional to their inputs and execution counts; pathological operations may block the editor.
- Retain existing syntax-rule disable/warning handling; test that a failing component of a two-part syntax rule is not accidentally accepted.

Tradeoff: small integration and fewer C bindings in exchange for a documented dialect change for user search/replacement and potentially third-party patterns. Historical PCRE2 evidence remains historical; update only current regression expectations and add tests for the new contract. Workload/performance and native safety validation are still required before calling implementation complete.
