# Regex alternatives: split responsibilities, don't swap one engine for all

Status: advisory only. Architecture not approved. No production change.
Goal context: remove PCRE2 via a Rust-native implementation (Linux, Rust core).

## Verdict

Split the five consumers into independent responsibilities. No single tool
(LPeg, Tree-sitter, pest/winnow, Rust `regex`) covers all of them without a
compatibility break. Keep a real regex engine for user search/snippets;
consider grammars/parsers for highlighting as a separate upgrade. Fixed
metadata can use ordinary string parsing; it needs neither regex nor a new
parser dependency. Splitting responsibilities does not require separate
engines, abstractions or crates for every caller.

## What each candidate actually is (primary sources)

- **LPeg** is a pattern-matching library for Lua based on Parsing Expression
  Grammars; patterns are first-class Lua values (userdata) composed from
  Lua, plus an `re` module (~270 lines of Lua) offering regex-style syntax.
  Engine itself is C, Lua-integrated — not Rust-native. It could replace
  PCRE2 for selected internal patterns, but would substitute another C
  implementation rather than meet the Rust-native goal.
  PEG semantics (ordered choice, predicates) are not drop-in PCRE semantics.
  Source: <https://www.inf.puc-rio.br/~roberto/lpeg/>
- **Tree-sitter** is a parser generator plus incremental parsing library:
  concrete syntax tree per file, efficiently updated on edit; goals are
  general/fast/robust/dependency-free, and the runtime library is written in
  pure C11. Parsing is via its C API (`tree_sitter/api.h`); generated
  parsers are C. The official Rust binding (`lib/binding_rust`,
  <https://docs.rs/tree-sitter>) wraps that C runtime — a Rust *binding*,
  not a Rust-native runtime, so do not claim memory safety for the whole
  path; the unsafe C core remains. Sources:
  <https://tree-sitter.github.io/tree-sitter/>,
  <https://tree-sitter.github.io/tree-sitter/using-parsers/index.html>
- **Rust `regex` crate** guarantees worst-case `O(m*n)` search but explicitly
  lacks look-around and backreferences. JS-style syntax rules using
  lookahead/atomic/possessive groups therefore do not port 1:1. A hybrid
  (`fancy-regex`, delegates to `regex` when possible, backtracks otherwise)
  restores features at the cost of the linear-time guarantee.
  Sources: <https://docs.rs/regex/latest/regex/>, <https://docs.rs/fancy-regex/>
- **pest / winnow** are pure-Rust parsing tools (pest: PEG grammars +
  codegen; winnow: parser-combinator library, nom successor). Right shape
  for small fixed grammars, wrong shape for ad-hoc user regex.
  Sources: <https://pest.rs/>, <https://docs.rs/pest>, <https://docs.rs/winnow>

## Recommended split

1. **User search + capture replacement → keep regex.** Users expect regex
   syntax. Rust `regex` fits iff look-around/backrefs are dispensable;
   otherwise `fancy-regex` hybrid or a retained PCRE2 slice. Decision needed.
2. **Syntax highlighting (stateful, per-line Lua/PCRE rules) → retain the
   current interface initially.** A Rust-native regex engine with advanced
   features is one candidate; the basic `regex` crate alone cannot preserve
   the existing JS rules. Do not silently degrade them. Tree-sitter is a
   separate option for incremental, error-tolerant trees and highlight
   queries, requiring per-language grammar packaging, query files,
   edit-tracking integration and capture-to-color mapping. It is not a
   Rust-native implementation. `detectindent` currently consumes syntax
   rules directly, so replacing highlighting alone will not decouple it;
   using comment/string spans would require its own integration.
3. **Fixed plugin metadata (mod-version/priority) → simple string parsing.**
   This can stay in Lua or use ordinary Rust string operations if ownership
   moves later. Preserve current accepted inputs and defaults. Neither a
   PEG library nor a new Rust module is justified just for these fields.
4. **`lsp_snippets` regex transforms → treat as user-regex lane** (often
   needs backrefs/case transforms); confirm feature set before choosing
   `regex` vs hybrid vs PCRE2 slice.
5. **LPeg → not the Rust-native slice.** It is C + Lua-integrated. A PEG
   design could be useful for developer-authored grammars, but rules being
   migrated would need translation and compatibility checks. Existing Lua
   pattern rules need not all migrate; a full LPeg replacement would be a
   separate syntax-definition migration.

## Scope / compatibility decisions requiring approval

- Dropping PCRE2 entirely requires a replacement supporting the constructs
  actually used, or explicit approval to change those rules. Advanced
  Rust-native regex engines may support them; PCRE2 parity is not assumed.
- Tree-sitter adoption (even gradual) requires dropping or dual-supporting
  Lua syntax definitions and plugin highlighting APIs; per-language grammar
  sourcing/versioning becomes a new maintenance burden.
- Do not claim a whole-binary pure-Rust outcome from any single slice:
  Tree-sitter's Rust crate still links the C runtime.
