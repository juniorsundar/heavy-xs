-- W1: regex. Baseline recorded from the C implementation.
-- NOTE: `regex.cmatch` with a *string* pattern segfaults in the C core
-- (f_pcre_match calls lua_rawgeti on a string), so patterns are compiled first,
-- exactly as data/core/regex.lua does.
require("core.regex")

local function join(...) return table.concat({ ... }, "|") end
local function iter(gen) local t = {} for v in gen do t[#t+1] = tostring(v) end return table.concat(t, "|") end
-- mlua raises errors as userdata (tostring adds "runtime error: " + traceback);
-- the C core raises plain strings. Strip the prefix and traceback so the rest,
-- which is what a user sees, can be compared.
local function err_prefix(ok, msg)
  return (tostring(msg):gsub("\n.*", ""):gsub("^runtime error: ", ""))
end

return function(rec)
  local re = regex.compile("(b)(c)")
  rec("type(compiled)", type(re))
  rec("getmetatable(compiled) == regex", tostring(getmetatable(re) == regex))
  rec("metatable __name", tostring(getmetatable(re).__name))

  rec("compile failure returns", select("#", regex.compile("(")))
  local _, cmsg = regex.compile("(")
  rec("compile failure msg", err_prefix(false, cmsg))
  rec("compile options i", join(regex.find_offsets(regex.compile("A", "i"), "xay")))
  rec("compile options m", join(regex.find_offsets(regex.compile("^a", "m"), "x\na")))

  rec("cmatch no match count", select("#", regex.cmatch(regex.compile("z"), "banana")))
  rec("cmatch offsets", join(regex.cmatch(re, "abcd")))
  rec("cmatch whole", join(regex.cmatch(regex.compile("bc"), "abcd")))
  rec("find_offsets", join(regex.find_offsets(re, "abcd")))
  rec("find (plain)", join(regex.find(regex.compile("a"), "banana")))
  rec("match (plain)", tostring(regex.match(regex.compile("a(n)"), "banana")))
  rec("cmatch offset 3", join(regex.cmatch(regex.compile("a"), "banana", 3)))
  rec("cmatch negative offset", join(regex.cmatch(regex.compile("a"), "banana", -2)))
  rec("cmatch NOTBOL", join(regex.cmatch(regex.compile("^a"), "ba", 2, regex.NOTBOL)))
  rec("find empty capture", join(regex.find(regex.compile("a()"), "ban")))

  rec("gmatch no groups", iter(regex.gmatch(regex.compile("X"), "aXbXc")))
  rec("gmatch with groups", iter(regex.gmatch(regex.compile("(a)(n)"), "banana")))
  rec("gmatch empty capture", iter(regex.gmatch(regex.compile("a()"), "ban")))
  rec("gmatch offset", iter(regex.gmatch(regex.compile("a"), "banana", 4)))

  rec("gsub global", join(regex.gsub(regex.compile("a"), "banana", "o")))
  rec("gsub limit 1", join(regex.gsub(regex.compile("a"), "banana", "o", 1)))
  rec("gsub limit 2", join(regex.gsub(regex.compile("a"), "banana", "o", 2)))
  rec("gsub limit 9", join(regex.gsub(regex.compile("a"), "banana", "o", 9)))
  rec("gsub no match", join(regex.gsub(regex.compile("z"), "banana", "o")))
  rec("gsub captures", join(regex.gsub(regex.compile("(a)(n)"), "banana", "$2$1")))
  rec("gsub escape", join(regex.gsub(regex.compile("a"), "banana", "\\\\")))
  rec("gsub on newlines", join(regex.gsub(regex.compile("^", "m"), "a\nb", "-")))
  rec("gsub empty match", join(regex.gsub(regex.compile("x*"), "abc", "-")))
  rec("gsub grow", join(regex.gsub(regex.compile("a"), string.rep("a", 2000), "bb")))
  local big, n = regex.gsub(regex.compile("a"), string.rep("a", 2000), "bb")
  rec("gsub grow len/count", #big, n)
  rec("gsub limit grow", join(regex.gsub(regex.compile("a"), string.rep("a", 300), "bb", 5)))
  local lbig, ln = regex.gsub(regex.compile("a"), string.rep("a", 300), "bb", 5)
  rec("gsub limit grow len/count", #lbig, ln)

  -- Plugins use method syntax (data/core/regex.lua installs __index) and
  -- detectindent.lua inspects pattern.regex[1] to tell tables from strings.
  rec("re[1] type", type(re[1]))
  rec("method gmatch", iter(re:gmatch("bcbc")))
  rec("method gsub", join(re:gsub("abcd", "-")))
  rec("method cmatch", join(re:cmatch("abcd")))
  rec("method find_offsets", join(re:find_offsets("abcd")))

  rec("bad subject err", err_prefix(pcall(regex.cmatch, re, nil)))
  rec("bad pattern err", err_prefix(pcall(regex.cmatch, "(", "a")))

  rec("ANCHORED", regex.ANCHORED)
  rec("ENDANCHORED", regex.ENDANCHORED)
  rec("NOTBOL", regex.NOTBOL)
  rec("NOTEOL", regex.NOTEOL)
  rec("NOTEMPTY", regex.NOTEMPTY)
  rec("NOTEMPTY_ATSTART", regex.NOTEMPTY_ATSTART)
end
