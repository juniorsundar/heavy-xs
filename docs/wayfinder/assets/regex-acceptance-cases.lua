-- Regex compatibility acceptance corpus (evidence for Wayfinder issue 07).
--
-- This is an evidence asset, not production code and not a replacement engine.
-- It returns function(check); the harness supplies check and compares the exact
-- number, types and values returned by the case function against expected.
--
--   check(unique_id, table.pack(expected_values...), function() return actual_values... end)
--
-- Coverage: the approved pattern feature floor with semantic contrasts
-- (lookaround, backreferences, atomic/possessive, inline flags, operators and
-- every capture type), UTF-8 byte offsets and default classes, the compiled-table
-- function/method interface, retained quirks (suffix-relative cmatch,
-- ENDANCHORED aliasing, ignored flag letters, positive-limit substitution),
-- the replacement dialect, failure-convention shapes, synthetic snippet
-- method-chain examples and the bundled JavaScript/C/C++/Markdown syntax driven
-- through the real tokenizer.
--
-- Deliberate exclusions (safe-input policy): no malformed compiled tables, no
-- out-of-range or mid-sequence offsets, no GC/iterator-lifetime exercise, no
-- unmatched-capture ("unset hole") cases and no gmatch iteration. Whole-match
-- and empty-match iterator progress checks belong to the harness, not here.
-- Error cases convert pcall results to booleans and never assert exact message
-- text or engine codes.
--
-- A passing run is evidence about the implementation under test; it is not proof
-- that an alternative engine meets the requirements in issue 03.

return function(check)
  require "core.regex"

  -- The bundled language plugins route tokenizer warnings through core.warn in a
  -- headless process. Give that call somewhere to go when this corpus is loaded
  -- outside a fully initialized editor; harmless no-ops in a normal one.
  local core = require "core"
  if core.threads == nil then core.threads = setmetatable({}, { __mode = "k" }) end
  if core.log_items == nil then core.log_items = {} end

  -- ---------------------------------------------------------------------------
  -- Compiled-object function/method interface (data/core/regex.lua, regex.rs).
  -- ---------------------------------------------------------------------------

  check("IF_compile_returns_table", table.pack("table"), function()
    return type(regex.compile("a"))
  end)

  check("IF_native_module_name", table.pack(true), function()
    return require("regex") == regex
  end)

  check("IF_metatable_is_regex_module", table.pack(true), function()
    return getmetatable(regex.compile("a")) == regex
  end)

  check("IF_metatable_name", table.pack("regex"), function()
    return getmetatable(regex.compile("a")).__name
  end)

  check("IF_method_gsub", table.pack("baanaanaa", 3), function()
    return regex.compile("(a)"):gsub("banana", "$1$1")
  end)

  check("IF_method_cmatch", table.pack(2, 3), function()
    return regex.compile("a"):cmatch("banana")
  end)

  check("IF_method_match_returns_captures", table.pack("a", "n"), function()
    return regex.compile("(a)(n)"):match("banana")
  end)

  check("IF_method_find_offsets_inclusive", table.pack(2, 2), function()
    return regex.compile("a"):find_offsets("banana")
  end)

  check("IF_find_offsets_accepts_string_pattern", table.pack(2, 2), function()
    return regex.find_offsets("a", "banana")
  end)

  check("IF_find_returns_capture_strings", table.pack(2, 3, "n"), function()
    return regex.find(regex.compile("a(n)"), "banana")
  end)

  check("IF_match_plain_substring", table.pack("an"), function()
    return regex.match(regex.compile("an"), "banana")
  end)

  check("IF_match_empty_capture_becomes_position", table.pack(3), function()
    return regex.match(regex.compile("a()"), "ban")
  end)

  check("IF_find_empty_capture_becomes_position", table.pack(2, 2, 3), function()
    return regex.find(regex.compile("a()"), "ban")
  end)

  -- ---------------------------------------------------------------------------
  -- Lookaround: positive/negative lookahead and fixed-length lookbehind.
  -- ---------------------------------------------------------------------------

  check("LA_positive_lookahead", table.pack(2, 3), function()
    return regex.cmatch(regex.compile("a(?=n)"), "banana")
  end)

  check("LA_negative_lookahead", table.pack(6, 7), function()
    return regex.cmatch(regex.compile("a(?!n)"), "banana")
  end)

  check("LA_lookahead_does_not_consume", table.pack(3, 3), function()
    return regex.cmatch(regex.compile("(?=na)"), "banana")
  end)

  check("LA_fixed_length_lookbehind", table.pack(4, 5), function()
    return regex.cmatch(regex.compile("(?<=n)a"), "banana")
  end)

  check("LA_negative_lookbehind", table.pack(2, 3), function()
    return regex.cmatch(regex.compile("(?<!n)a"), "banana")
  end)

  check("LA_consuming_contrast", table.pack(3, 5), function()
    return regex.cmatch(regex.compile("na"), "banana")
  end)

  -- ---------------------------------------------------------------------------
  -- Numeric and named backreferences.
  -- ---------------------------------------------------------------------------

  check("BR_numeric_backreference", table.pack(1, 5, 1, 2, 2, 3), function()
    return regex.cmatch(regex.compile("(a)(n)\\2\\1"), "anna")
  end)

  check("BR_named_backreference", table.pack(1, 5, 1, 2, 2, 3), function()
    return regex.cmatch(regex.compile("(?<x>a)(?<y>n)\\k<y>\\k<x>"), "anna")
  end)

  check("BR_numeric_backreference_mismatch", table.pack(), function()
    return regex.cmatch(regex.compile("(a)(n)\\2\\2"), "anna")
  end)

  -- ---------------------------------------------------------------------------
  -- Atomic groups and possessive quantifiers prevent backtracking.
  -- ---------------------------------------------------------------------------

  check("AP_atomic_blocks_backtrack", table.pack(), function()
    return regex.cmatch(regex.compile("(?>a+)a"), "aab")
  end)

  check("AP_plain_quantifier_backtracks", table.pack(1, 3), function()
    return regex.cmatch(regex.compile("a+a"), "aab")
  end)

  check("AP_possessive_blocks_backtrack", table.pack(), function()
    return regex.cmatch(regex.compile("a++a"), "aab")
  end)

  check("AP_possessive_quantifier_alone", table.pack("aaa"), function()
    return regex.match(regex.compile("a++"), "aaa")
  end)

  check("AP_atomic_group_alone", table.pack("aaa"), function()
    return regex.match(regex.compile("(?>a+)"), "aaa")
  end)

  -- ---------------------------------------------------------------------------
  -- Inline compile flags i/m/s plus compile-time flag contrasts.
  -- ---------------------------------------------------------------------------

  check("FX_inline_i", table.pack("A"), function()
    return regex.match(regex.compile("(?i)a"), "A")
  end)

  check("FX_inline_m", table.pack(3, 3), function()
    return regex.find_offsets(regex.compile("(?m)^b"), "a\nb")
  end)

  check("FX_inline_s_matches_newline", table.pack(10), function()
    return regex.match(regex.compile("(?s)."), "\n"):byte(1)
  end)

  check("FX_anchor_without_m_misses", table.pack(), function()
    return regex.cmatch(regex.compile("^a"), "x\na")
  end)

  check("FX_compile_m_hits", table.pack(3, 4), function()
    return regex.cmatch(regex.compile("^a", "m"), "x\na")
  end)

  check("FX_compile_i_hits", table.pack(2, 2), function()
    return regex.find_offsets(regex.compile("A", "i"), "xay")
  end)

  check("FX_compile_s_hits_newline", table.pack(10), function()
    return regex.match(regex.compile(".", "s"), "\n"):byte(1)
  end)

  -- ---------------------------------------------------------------------------
  -- Ordinary operators and all capture types.
  -- ---------------------------------------------------------------------------

  check("OP_alternation", table.pack("dog"), function()
    return regex.match(regex.compile("cat|dog"), "hotdog")
  end)

  check("OP_greedy", table.pack("<a><b>"), function()
    return regex.match(regex.compile("<.+>"), "<a><b>")
  end)

  check("OP_lazy", table.pack("<a>"), function()
    return regex.match(regex.compile("<.+?>"), "<a><b>")
  end)

  check("OP_star_zero_repetition", table.pack("ac"), function()
    return regex.match(regex.compile("ab*c"), "ac")
  end)

  check("OP_question_zero_or_one", table.pack("ac"), function()
    return regex.match(regex.compile("ab?c"), "ac")
  end)

  check("OP_bounded_repetition", table.pack("123"), function()
    return regex.match(regex.compile("\\d{2,3}"), "12345")
  end)

  check("OP_class_range", table.pack("abc"), function()
    return regex.match(regex.compile("[a-c]+"), "xxabcxx")
  end)

  check("OP_word_boundary", table.pack(3, 5), function()
    return regex.find_offsets(regex.compile("\\bcat\\b"), "a cat.")
  end)

  check("OP_line_anchors", table.pack("banana"), function()
    return regex.match(regex.compile("^b.*a$"), "banana")
  end)

  check("OP_escaped_dot", table.pack("."), function()
    return regex.match(regex.compile("\\."), "a.b")
  end)

  check("OP_capture_numeric", table.pack(2, 4, 2, 3, 3, 4), function()
    return regex.cmatch(regex.compile("(a)(n)"), "banana")
  end)

  check("OP_capture_named", table.pack(2, 4, 2, 3, 3, 4), function()
    return regex.cmatch(regex.compile("(?<x>a)(?<y>n)"), "banana")
  end)

  check("OP_capture_noncapturing_omitted", table.pack(4), function()
    return select("#", regex.cmatch(regex.compile("(?:a)(n)"), "banana"))
  end)

  check("OP_capture_empty_group", table.pack(2, 3, 3, 3), function()
    return regex.cmatch(regex.compile("a()"), "ban")
  end)

  -- ---------------------------------------------------------------------------
  -- UTF-8 byte offsets, default class behavior, safe invalid-UTF failure.
  -- ---------------------------------------------------------------------------

  check("UT_dot_multibyte_byte_span", table.pack(1, 3), function()
    return regex.cmatch(regex.compile("."), "é")
  end)

  check("UT_find_offsets_inclusive_end", table.pack(1, 2), function()
    return regex.find_offsets(".", "é")
  end)

  check("UT_capture_multibyte_span", table.pack(1, 3, 1, 3), function()
    return regex.cmatch(regex.compile("(é)"), "é")
  end)

  check("UT_default_word_class_ascii_only", table.pack(), function()
    return regex.cmatch(regex.compile("\\w"), "é")
  end)

  check("UT_default_digit_class_ascii", table.pack("42"), function()
    return regex.match(regex.compile("\\d+"), "x42")
  end)

  check("UT_invalid_utf_fails_safely", table.pack(false), function()
    return (pcall(regex.cmatch, regex.compile("."), "\255"))
  end)

  -- ---------------------------------------------------------------------------
  -- Retained quirks: suffix-relative cmatch, ENDANCHORED alias, ignored flag
  -- letters, positive-limit substitution and zero-value no-match returns.
  -- ---------------------------------------------------------------------------

  check("RQ_suffix_cmatch_anchor", table.pack(2, 3), function()
    return regex.cmatch(regex.compile("^a"), "ba", 2)
  end)

  check("RQ_suffix_lookbehind_loses_prefix", table.pack(), function()
    return regex.cmatch(regex.compile("(?<=b)a"), "ba", 2)
  end)

  check("RQ_endanchored_aliases_anchored", table.pack(true), function()
    return regex.ENDANCHORED == regex.ANCHORED
  end)

  check("RQ_match_constant_values",
        table.pack(2147483648, 1, 2, 4, 8), function()
    return regex.ANCHORED, regex.NOTBOL, regex.NOTEOL,
           regex.NOTEMPTY, regex.NOTEMPTY_ATSTART
  end)

  check("RQ_anchored_option_requires_start", table.pack(), function()
    return regex.cmatch(regex.compile("a"), "ba", 1, regex.ANCHORED)
  end)

  check("RQ_endanchored_option_acts_anchored", table.pack(), function()
    return regex.cmatch(regex.compile("a"), "ba", 1, regex.ENDANCHORED)
  end)

  check("RQ_unanchored_option_finds_later_match", table.pack(2, 3), function()
    return regex.cmatch(regex.compile("a"), "ba")
  end)

  check("RQ_unknown_flag_letter_ignored", table.pack(1, 2), function()
    return regex.cmatch(regex.compile("a", "z"), "a")
  end)

  check("RQ_positive_limit_empty_substitution", table.pack("--abc", 2), function()
    return regex.gsub(regex.compile("x*"), "abc", "-", 2)
  end)

  check("RQ_global_empty_substitution", table.pack("-a-b-c-", 4), function()
    return regex.gsub(regex.compile("x*"), "abc", "-")
  end)

  check("RQ_negative_limit_means_global", table.pack("xx", 2), function()
    return regex.gsub(regex.compile("a"), "aa", "x", -1)
  end)

  check("RQ_zero_limit_means_global", table.pack("xx", 2), function()
    return regex.gsub(regex.compile("a"), "aa", "x", 0)
  end)

  check("RQ_gsub_no_match", table.pack("abc", 0), function()
    return regex.gsub(regex.compile("z"), "abc", "-")
  end)

  check("RQ_notbol_suppresses_anchor", table.pack(), function()
    return regex.cmatch(regex.compile("^a"), "ba", 2, regex.NOTBOL)
  end)

  check("RQ_cmatch_no_match_zero_values", table.pack(), function()
    return regex.cmatch(regex.compile("z"), "banana")
  end)

  check("RQ_find_no_match_zero_values", table.pack(), function()
    return regex.find(regex.compile("z"), "banana")
  end)

  -- ---------------------------------------------------------------------------
  -- Replacement-template dialect (PCRE2 extended, not Lua %n interpolation).
  -- ---------------------------------------------------------------------------

  check("RP_numeric_capture_swap", table.pack("bnanaa", 2), function()
    return regex.gsub(regex.compile("(a)(n)"), "banana", "$2$1")
  end)

  check("RP_named_brace_substitution", table.pack("a", 1), function()
    return regex.gsub(regex.compile("(?<letter>a)"), "a", "${letter}")
  end)

  check("RP_double_dollar_then_capture", table.pack("$a", 1), function()
    return regex.gsub(regex.compile("(a)"), "a", "$$$1")
  end)

  check("RP_percent_is_literal", table.pack("%1", 1), function()
    return regex.gsub(regex.compile("(a)"), "a", "%1")
  end)

  check("RP_escaped_backslash_is_literal", table.pack("\\\\", 2), function()
    return regex.gsub(regex.compile("(a)"), "aa", "\\\\")
  end)

  -- ---------------------------------------------------------------------------
  -- Failure conventions: shape only, never exact wording or engine codes.
  -- ---------------------------------------------------------------------------

  check("FL_compile_failure_shape", table.pack(true, "string", true), function()
    local compiled, message = regex.compile("(")
    return compiled == nil, type(message), type(message) == "string" and #message > 0
  end)

  check("FL_explicit_nil_flag_raises", table.pack(false), function()
    return (pcall(regex.compile, "a", nil))
  end)

  check("FL_non_string_subject_raises", table.pack(false), function()
    return (pcall(regex.cmatch, regex.compile("a"), nil))
  end)

  check("FL_function_replacement_rejected", table.pack(false), function()
    return (pcall(regex.gsub, regex.compile("a"), "a", function() return "b" end))
  end)

  -- ---------------------------------------------------------------------------
  -- SYNTHETIC snippet-plugin method chains (compile -> reg:gsub / reg:match).
  -- Modeled on the compile/match/gsub call flow of a snippet consumer; these are
  -- synthetic and are NOT copied from any installed or private plugin file.
  -- ---------------------------------------------------------------------------

  check("SYN_transform_filename_gsub", table.pack("Widget", 1), function()
    return regex.compile("^(.*)\\.([^.]+)$"):gsub("Widget.TS", "$1")
  end)

  check("SYN_transform_match_captures", table.pack("alpha", "beta"), function()
    return regex.compile("([a-z]+)-([a-z]+)"):match("alpha-beta")
  end)

  check("SYN_transform_flag_i", table.pack("VALUE", 1), function()
    return regex.compile("name", "i"):gsub("Name", "VALUE")
  end)

  check("SYN_transform_case_template", table.pack("Alpha-Beta", 2), function()
    return regex.compile("([a-z]+)", "g"):gsub("alpha-beta", [[\u$1]])
  end)

  -- ---------------------------------------------------------------------------
  -- Bundled JavaScript/C/C++/Markdown regex definitions through the real
  -- tokenizer. The old tokenizer suite loads Lua syntax only, so this is the
  -- integration evidence for the bundled regex patterns (including the JS
  -- atomic group and possessive quantifiers).
  -- ---------------------------------------------------------------------------

  local syntax = require "core.syntax"
  local tokenizer = require "core.tokenizer"
  for _, module in ipairs({ "plugins.language_js", "plugins.language_c",
                            "plugins.language_cpp", "plugins.language_md" }) do
    require(module)
  end

  local function tokens(file, text)
    local syn = syntax.get(file, "")
    local out = {}
    for _, token_type, token_text in tokenizer.each_token(tokenizer.tokenize(syn, text, "")) do
      out[#out + 1] = token_type .. ":" .. token_text:gsub("\n", "\\n")
    end
    return table.concat(out, " ")
  end

  check("TOK_bundled_syntax_names",
        table.pack("JavaScript", "C", "C++", "Markdown"), function()
    return (syntax.get("x.js", "") or {}).name,
           (syntax.get("x.c", "") or {}).name,
           (syntax.get("x.cpp", "") or {}).name,
           (syntax.get("x.md", "") or {}).name
  end)

  check("TOK_js_regex_literal",
        table.pack("keyword:let symbol: r operator: = string: /ab+c/gi normal:;"),
        function()
    return tokens("x.js", "let r = /ab+c/gi;")
  end)

  check("TOK_js_regex_class_and_escaped_slash",
        table.pack([=[keyword:let symbol: r operator: = string: /[a-z]+\//g normal:;]=]),
        function()
    return tokens("x.js", [=[let r = /[a-z]+\//g;]=])
  end)

  check("TOK_js_number_div_not_comment",
        table.pack("keyword:let symbol: n operator: = number: 1.5 operator:/ symbol:x normal:;"),
        function()
    return tokens("x.js", "let n = 1.5/x;")
  end)

  check("TOK_c_integer_suffix",
        table.pack("keyword2:int normal: x  operator:= number: 0x1fUL normal:;"),
        function()
    return tokens("x.c", "int x = 0x1fUL;")
  end)

  check("TOK_c_float_suffix",
        table.pack("keyword2:double normal: d  operator:= number: 1.5e3f normal:;"),
        function()
    return tokens("x.c", "double d = 1.5e3f;")
  end)

  check("TOK_c_octal_capture",
        table.pack("keyword2:int normal: o  operator:= keyword: 0 number:77 normal:;"),
        function()
    return tokens("x.c", "int o = 077;")
  end)

  check("TOK_cpp_binary_suffix",
        table.pack("keyword:auto normal: n  operator:= number: 0b1010u normal:;"),
        function()
    return tokens("x.cpp", "auto n = 0b1010u;")
  end)

  check("TOK_cpp_class_declaration",
        table.pack("keyword:class  keyword2:Foo normal:;"), function()
    return tokens("x.cpp", "class Foo;")
  end)

  check("TOK_markdown_inline_math",
        table.pack("normal:a  string:$ normal:x+1 string:$ normal: b"), function()
    return tokens("x.md", "a $x+1$ b")
  end)

  check("TOK_markdown_math_block",
        table.pack("string:$$ normal:y string:$$"), function()
    return tokens("x.md", "$$y$$")
  end)
end
