-- Replacement-template fixtures for the existing PCRE2-backed `regex.gsub`.
--
-- Evidence for Wayfinder issue 07 (assemble the regex compatibility corpus).
-- These are observations of the current engine, not replacement acceptance
-- criteria and not a policy decision. They exercise interpretation of the
-- *replacement* string only; matching is the separate corpus case.
--
-- Dialect sources, distinguished per fixture group:
--   [10.44]  documented in repository-local subprojects/pcre2-10.44
--            (doc/pcre2api.3 + src/pcre2_substitute.c). The linked build is a
--            different PCRE2 release, so these are also confirmed at runtime.
--   [linked] observed on the linked PCRE2 10.46 build; these forms are not
--            established by the repository-local 10.44 documentation.
--            Observations do not establish which release introduced a form.
--
-- Contract: this file returns `function(check)`. Each call is
--   check(unique_id, table.pack(expected_values...), function() return actual... end)
-- and `check` (supplied by the caller) compares exact value counts, types and
-- values. Error fixtures return a boolean from `pcall`; they never assert an
-- engine code or message text.
require "core.regex"

return function(check)
  local function errored(pattern, subject, replacement, limit)
    return not pcall(regex.gsub, pattern, subject, replacement, limit)
  end

  -- ------------------------------------------------------------------
  -- Dollar forms always recognized (10.44 doc + linked agreement) [10.44]
  -- ------------------------------------------------------------------
  check("dollar_whole_match", table.pack("=+babcb+=", 1), function()
    return regex.gsub("a(b)c", "=abc=", "+$1$0$1+")
  end)
  check("dollar_numeric", table.pack("ba", 1), function()
    return regex.gsub("(a)(b)", "ab", "$2$1")
  end)
  check("dollar_braced_numeric", table.pack("a-b", 1), function()
    return regex.gsub("(a)(b)", "ab", "${1}-${2}")
  end)
  check("dollar_named", table.pack("<a>", 1), function()
    return regex.gsub("(?<word>a)", "a", "<$word>")
  end)
  check("dollar_named_braced", table.pack("<a>", 1), function()
    return regex.gsub("(?<word>a)", "a", "<${word}>")
  end)
  check("dollar_literal", table.pack("$", 1), function()
    return regex.gsub("a", "a", "$$")
  end)
  check("dollar_literal_then_capture", table.pack("$a", 1), function()
    return regex.gsub("(a)", "a", "$$$1")
  end)

  -- ------------------------------------------------------------------
  -- Extended conditional / default forms [10.44]
  -- ------------------------------------------------------------------
  check("ext_default_group_set", table.pack("b", 1), function()
    return regex.gsub("(a)(b)?", "ab", "${2:-D}")
  end)
  check("ext_default_group_unset", table.pack("D", 1), function()
    return regex.gsub("(a)(b)?", "a", "${2:-D}")
  end)
  check("ext_conditional_group_set", table.pack("S", 1), function()
    return regex.gsub("(a)(b)?", "ab", "${2:+S:U}")
  end)
  check("ext_conditional_group_unset", table.pack("U", 1), function()
    return regex.gsub("(a)(b)?", "a", "${2:+S:U}")
  end)
  check("ext_mark_named", table.pack("tag", 1), function()
    return regex.gsub("(*MARK:tag)(a)", "a", "${*MARK}")
  end)
  check("ext_mark_absent_is_empty", table.pack("", 1), function()
    return regex.gsub("(a)", "a", "${*MARK}")
  end)

  -- ------------------------------------------------------------------
  -- Backslash escapes in extended mode [10.44]
  -- ------------------------------------------------------------------
  check("bs_hex_plain", table.pack("A", 1), function()
    return regex.gsub("a", "a", [[\x41]])
  end)
  check("bs_hex_braced", table.pack("A", 1), function()
    return regex.gsub("a", "a", [[\x{41}]])
  end)
  check("bs_octal_braced", table.pack("A", 1), function()
    return regex.gsub("a", "a", [[\o{101}]])
  end)
  check("bs_control_c", table.pack("\1", 1), function()
    return regex.gsub("a", "a", [[\cA]])
  end)
  check("bs_newline", table.pack("\n", 1), function()
    return regex.gsub("a", "a", [[\n]])
  end)
  check("bs_tab", table.pack("\t", 1), function()
    return regex.gsub("a", "a", [[\t]])
  end)
  check("bs_literal_backslash", table.pack("\\", 1), function()
    return regex.gsub("a", "a", [[\\]])
  end)
  check("bs_quoted_slash", table.pack("/", 1), function()
    return regex.gsub("a", "a", [[\/]])
  end)
  check("bs_quoted_brace", table.pack("{", 1), function()
    return regex.gsub("a", "a", [[\{]])
  end)
  check("bs_Q_quotes_dollar", table.pack("$1", 1), function()
    return regex.gsub("(a)", "a", [[\Q$1\E]])
  end)
  check("bs_E_isolated_ignored", table.pack("ab", 1), function()
    return regex.gsub("a", "a", [[a\Eb]])
  end)

  -- ------------------------------------------------------------------
  -- Case conversion scope and resets (extended) [10.44]
  -- ------------------------------------------------------------------
  check("case_upper_rest", table.pack("A", 1), function()
    return regex.gsub("(a)", "a", [[\U$1\E]])
  end)
  check("case_lower_rest", table.pack("a", 1), function()
    return regex.gsub("(A)", "A", [[\L$1\E]])
  end)
  check("case_title_first", table.pack("A", 1), function()
    return regex.gsub("(a)", "a", [[\u$1]])
  end)
  check("case_lower_first", table.pack("a", 1), function()
    return regex.gsub("(A)", "A", [[\l$1]])
  end)
  check("case_scope_two_captures", table.pack("AB", 1), function()
    return regex.gsub("(a)(b)", "ab", [[\U$1$2]])
  end)
  check("case_reset_restores", table.pack("Aa", 1), function()
    return regex.gsub("(a)", "a", [[\U$1\E$1]])
  end)
  check("case_no_nesting_upper_then_lower", table.pack("AAbb", 1), function()
    return regex.gsub("x", "x", [[\Uaa\Lbb\E]])
  end)
  check("case_upper_literal_text", table.pack("XY", 1), function()
    return regex.gsub("a", "a", [[\Uxy]])
  end)

  -- ------------------------------------------------------------------
  -- Lua `%n` is literal replacement text, not capture interpolation [10.44]
  -- ------------------------------------------------------------------
  check("lua_percent_is_literal", table.pack("%1", 1), function()
    return regex.gsub("(a)", "a", "%1")
  end)
  check("lua_percent_then_capture", table.pack("%1a", 1), function()
    return regex.gsub("(a)", "a", "%1$1")
  end)

  -- ------------------------------------------------------------------
  -- Malformed replacement -> safe error, not silent text. Errors are
  -- reduced to a boolean; no engine code or wording is asserted. [10.44]
  -- ------------------------------------------------------------------
  check("bad_trailing_dollar", table.pack(true), function()
    return errored("a", "a", "x$")
  end)
  check("bad_missing_closing_brace", table.pack(true), function()
    return errored("a", "a", "${1")
  end)
  check("bad_unknown_group_number", table.pack(true), function()
    return errored("(a)", "a", "$9")
  end)
  check("bad_unknown_group_name", table.pack(true), function()
    return errored("(a)", "a", "$nope")
  end)
  check("bad_unset_group", table.pack(true), function()
    return errored("(a)(b)?", "a", "$2")
  end)
  check("bad_extended_operator", table.pack(true), function()
    return errored("(a)", "a", "${1:?x}")
  end)
  check("bad_invalid_escape", table.pack(true), function()
    return errored("a", "a", [[\Z]])
  end)

  -- ------------------------------------------------------------------
  -- Limit / global / positive-limit behavior [10.44]
  -- ------------------------------------------------------------------
  check("limit_one", table.pack("xa", 1), function()
    return regex.gsub("a", "aa", "x", 1)
  end)
  check("limit_two", table.pack("xxa", 2), function()
    return regex.gsub("a", "aaa", "x", 2)
  end)
  check("limit_negative_is_global", table.pack("xx", 2), function()
    return regex.gsub("a", "aa", "x", -1)
  end)
  check("limit_positive_empty_match_quirk", table.pack("--abc", 2), function()
    return regex.gsub("x*", "abc", "-", 2)
  end)
  check("global_empty_match", table.pack("-a-b-c-", 4), function()
    return regex.gsub("x*", "abc", "-")
  end)
  check("no_match_returns_input", table.pack("banana", 0), function()
    return regex.gsub("z", "banana", "o")
  end)

  -- ------------------------------------------------------------------
  -- Additional linked-build observations, not 10.44-doc-backed. They belong
  -- to the current-dialect preservation target; any incompatible candidate
  -- needs explicit approval, not an automatic exclusion of these forms.
  -- This is finite coverage, not proof of the complete 10.46 grammar. [linked]
  -- ------------------------------------------------------------------
  check("linked_perl_ampersand_whole", table.pack("x[ab]y", 1), function()
    return regex.gsub("(a)(b)", "xaby", "[$&]")
  end)
  check("linked_perl_prefix", table.pack("x[x]y", 1), function()
    return regex.gsub("(a)(b)", "xaby", "[$`]")
  end)
  check("linked_perl_suffix", table.pack("x[y]y", 1), function()
    return regex.gsub("(a)(b)", "xaby", "[$']")
  end)
  check("linked_perl_whole_input", table.pack("x[xaby]y", 1), function()
    return regex.gsub("(a)(b)", "xaby", "[$_]")
  end)
  check("linked_js_angle_named", table.pack("[a]", 1), function()
    return regex.gsub("(?<w>a)", "a", "[$<w>]")
  end)
  check("linked_backslash_group_number", table.pack("[ab]", 1), function()
    return regex.gsub("(a)(b)", "ab", [==[[\1\2]]==])
  end)
  check("linked_backslash_g_named", table.pack("[a]", 1), function()
    return regex.gsub("(?<w>a)", "a", [==[[\g<w>]]==])
  end)
  check("linked_backslash_g_number", table.pack("[a]", 1), function()
    return regex.gsub("(a)", "a", [==[[\g<1>]]==])
  end)
  check("linked_backslash_b_backspace", table.pack("[\8]", 1), function()
    return regex.gsub("a", "a", [==[[\b]]==])
  end)
  check("linked_backslash_v_vtab", table.pack("[\11]", 1), function()
    return regex.gsub("a", "a", [==[[\v]]==])
  end)
  check("linked_case_reverse_title", table.pack("aB", 1), function()
    return regex.gsub("x", "x", [[\l\Uab\E]])
  end)
  check("linked_case_title_rest", table.pack("Ab", 1), function()
    return regex.gsub("x", "x", [[\u\Lab\E]])
  end)
  check("linked_bad_g_brace_form", table.pack(true), function()
    return errored("(?<w>a)", "a", [[\g{w}]])
  end)
  check("linked_bad_js_angle_unclosed", table.pack(true), function()
    return errored("(?<w>a)", "a", "[$<w]")
  end)
end
