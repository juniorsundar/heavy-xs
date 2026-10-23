-- Native fancy-regex contract. Historical PCRE2 evidence is in docs/wayfinder/assets/.
require("core.regex")

local function join(...) return table.concat({ ... }, "|") end
local function iter(gen) local t = {} for v in gen do t[#t+1] = tostring(v) end return table.concat(t, "|") end
-- mlua raises errors as userdata (tostring adds "runtime error: " + traceback). Strip the
-- prefix and traceback so the rest, which is what a user sees, is recorded.
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
  rec("cmatch ANCHORED", join(regex.cmatch(regex.compile("^a"), "ba", 2, regex.ANCHORED)))
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
  assert(regex.NOTBOL == nil and regex.NOTEOL == nil)
  assert(regex.NOTEMPTY == nil and regex.NOTEMPTY_ATSTART == nil)
  local ok, msg = pcall(regex.cmatch, "a", "a", 1, 1)
  assert(not ok and tostring(msg):find("REGEX_OPTIONS_ERROR", 1, true))

  -- Lua helpers must retain nil capture holes and the original compile error.
  local captures = table.pack(regex.match("(a)?(b)", "b"))
  assert(captures.n == 2 and captures[1] == nil and captures[2] == "b")
  local offsets = table.pack(regex.find_offsets("(a)?(b)", "b"))
  assert(offsets.n == 6 and offsets[3] == nil and offsets[4] == nil)
  ok, msg = pcall(regex.find_offsets, "(", "a")
  assert(not ok and tostring(msg):find("regex compilation failed", 1, true))

  local syntax, tokenizer = require "core.syntax", require "core.tokenizer"
  require "plugins.language_js"
  local js = syntax.get("contract.js", "")
  local function tokens(text)
    local out = {}
    for _, kind, part in tokenizer.each_token(tokenizer.tokenize(js, text, "")) do
      out[#out + 1] = kind .. ":" .. part
    end
    return table.concat(out, " ")
  end
  assert(tokens("let r = /ab+c/gi;") ==
    "keyword:let symbol: r operator: = string: /ab+c/gi normal:;")
  assert(tokens([=[let r = /[a-z]+\//g;]=]) ==
    [=[keyword:let symbol: r operator: = string: /[a-z]+\//g normal:;]=])
  assert(tokens("let n = 1.5/x;") ==
    "keyword:let symbol: n operator: = number: 1.5 operator:/ symbol:x normal:;")

  -- Warming every exercised rule should leave no compilation in the hot path.
  local compile, recompiles = regex.compile, 0
  regex.compile = function(...)
    recompiles = recompiles + 1
    return compile(...)
  end
  ok, msg = pcall(function()
    tokens("let r = /ab+c/gi;")
    tokens([=[let r = /[a-z]+\//g;]=])
    tokens("let n = 1.5/x;")
  end)
  regex.compile = compile
  assert(ok, tostring(msg))
  assert(recompiles == 0, "warmed syntax rules recompiled " .. recompiles .. " times")

  -- A valid closing expression must not hide a failed opening expression.
  local core = require "core"
  local warn, warnings = core.warn, 0
  core.warn = function() warnings = warnings + 1 end
  local rule = { regex = { "(", "x" }, type = "string" }
  ok, msg = pcall(function()
    syntax.add { name = "Invalid regex contract", patterns = { rule } }
  end)
  core.warn = warn
  assert(ok, tostring(msg))
  assert(rule.disabled and warnings == 1)
  assert(tokenizer.tokenize({ patterns = { rule } }, "x", ""))

  -- Replace-all computes its result before modifying the actual document.
  core.threads = core.threads or {}
  local doc = require("core.doc")()
  doc:insert(1, 1, "unchanged")
  local before = table.concat(doc.lines)
  regex.set_limits(1000000, 4)
  ok, msg = pcall(doc.replace, doc, function(text)
    return regex.gsub("absent", text, "")
  end)
  regex.set_limits(1000000, 8 * 1024 * 1024)
  assert(not ok and tostring(msg):find("REGEX_OUTPUT_LIMIT", 1, true))
  assert(table.concat(doc.lines) == before)
end
