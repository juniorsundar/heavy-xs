-- utf8extra robustness: inputs that stressed the C build's pointer arithmetic. The port
-- panics (aborting the editor, since panic = "abort") on any out-of-bounds slice or integer
-- overflow, so every one of these must come back as a clean result or a Lua error. Status plus
-- results only.
local utf8 = require "utf8extra"

local function val(fn, ...)
  local r = table.pack(pcall(fn, ...))
  local out = { tostring(r[1]) }
  if r[1] then for i = 2, r.n do out[#out + 1] = tostring(r[i]) end end
  return table.concat(out, "|")
end
local function iter(gen)
  local t = {}
  for a, b in gen do t[#t + 1] = tostring(a) .. "=" .. tostring(b) end
  return table.concat(t, "|")
end

return function(rec)
  local function r(key, fn, ...) rec(key, val(fn, ...)) end

  -- malformed / truncated patterns
  local BAD = {
    "%", "[", "[a", "[a-", "[^", "[]", "[%", "(", ")", "(()", "%b", "%b(", "%f", "%fa",
    "%1", "%2", "(a)%2", "a**", "a+*", "%", "[a-]", "[-a]", "[a-z", "[%a-z]", "(a", "a)",
    "\xFF", "a\xFF", "[\xFF]", "%\xFF", "^", "$", "^$", "$$", "a$b", "()", "(())",
  }
  for i, p in ipairs(BAD) do
    r("bad find " .. i, utf8.find, "abc", p)
    r("bad match " .. i, utf8.match, "abc", p)
    r("bad gsub " .. i, utf8.gsub, "abc", p, "x")
    r("bad gmatch " .. i, function() return iter(utf8.gmatch("abc", p)) end)
  end

  -- patterns against empty / boundary subjects
  for _, s in ipairs({ "", "a", "\xFF", "日" }) do
    for _, p in ipairs({ "", ".", ".*", ".-", "^", "$", "()", "%f[%w]", "%b()", "[^a]", "a?" }) do
      r("empty find " .. s .. "/" .. p, utf8.find, s, p)
      r("empty gsub " .. s .. "/" .. p, utf8.gsub, s, p, "-")
      r("empty gmatch " .. s .. "/" .. p, function() return iter(utf8.gmatch(s, p)) end)
    end
  end

  -- init positions outside the string
  for _, init in ipairs({ -100, -5, -1, 0, 1, 2, 3, 4, 5, 100, math.maxinteger, math.mininteger }) do
    r("find init " .. tostring(init), utf8.find, "héllo", "l", init)
    r("find plain init " .. tostring(init), utf8.find, "héllo", "l", init, true)
    r("match init " .. tostring(init), utf8.match, "héllo", "l", init)
    r("sub init " .. tostring(init), utf8.sub, "héllo", init)
    r("sub init,init " .. tostring(init), utf8.sub, "héllo", init, init)
    r("len init " .. tostring(init), utf8.len, "héllo", init)
    r("codepoint init " .. tostring(init), utf8.codepoint, "héllo", init)
    r("byte init " .. tostring(init), utf8.byte, "héllo", init)
    r("offset init " .. tostring(init), utf8.offset, "héllo", 1, init)
    r("offset n " .. tostring(init), utf8.offset, "héllo", init)
    r("charpos n " .. tostring(init), utf8.charpos, "héllo", init)
    r("charpos off " .. tostring(init), utf8.charpos, "héllo", init, 1)
    r("next off " .. tostring(init), utf8.next, "héllo", init)
    r("insert at " .. tostring(init), utf8.insert, "héllo", init, "X")
    r("remove at " .. tostring(init), utf8.remove, "héllo", init)
    r("remove range " .. tostring(init), utf8.remove, "héllo", init, init)
    r("widthindex " .. tostring(init), utf8.widthindex, "héllo", init)
  end

  -- limits
  r("gsub max_s 0", utf8.gsub, "aaa", "a", "b", 0)
  r("gsub max_s -1", utf8.gsub, "aaa", "a", "b", -1)
  r("gsub max_s big", utf8.gsub, "aaa", "a", "b", math.maxinteger)
  r("gsub anchored", utf8.gsub, "aaa", "^a", "b")
  r("gsub anchored empty", utf8.gsub, "", "^", "b")
  r("deep pattern", utf8.find, string.rep("a", 500), string.rep("a?", 300) .. string.rep("a", 300))
  r("long subject", function() return #select(1, utf8.gsub(string.rep("ab", 50000), "a", "xx")) end)
  r("long find", utf8.find, string.rep("a", 100000) .. "b", "b")
  r("many captures", utf8.find, "a", string.rep("(a?)", 33))
  r("32 captures", utf8.find, "a", string.rep("(a?)", 32))
  r("nested unfinished", utf8.find, "abc", "((a)")
  r("backref past", utf8.find, "abcabc", "(abc)%2")

  -- replacements
  for _, rep in ipairs({ "%", "%%", "%0", "%1", "%2", "%a", "x%", "%9" }) do
    r("repl " .. rep, utf8.gsub, "abc", "(b)", rep)
  end
  r("repl number", utf8.gsub, "abc", "b", 42)
  r("repl table nil", utf8.gsub, "abc", "b", {})
  r("repl table false", utf8.gsub, "abc", "b", { b = false })
  r("repl table bad", utf8.gsub, "abc", "b", { b = {} })
  r("repl fn bad", utf8.gsub, "abc", "b", function() return {} end)
  r("repl fn num", utf8.gsub, "abc", "b", function() return 7 end)
  r("repl fn multi", utf8.gsub, "abc", "(a)(b)", function(a, b) return b .. a end)
  r("repl bool", utf8.gsub, "abc", "b", true)

  -- malformed subjects through every walker
  local BADSUBJ = { "\xFF", "\x80\x80", "\xC3", "\xE2\x82", "a\xFF", "\xF8\x88\x80\x80\x80", "\xFD\xBF\xBF\xBF\xBF\xBF" }
  for i, s in ipairs(BADSUBJ) do
    for _, fn in ipairs({ "len", "sub", "reverse", "byte", "codepoint", "lower", "upper", "width", "widthindex", "offset", "charpos", "next" }) do
      r("badsubj " .. fn .. i, utf8[fn], s, 1)
    end
    r("badsubj find " .. i, utf8.find, s, ".")
    r("badsubj gsub " .. i, utf8.gsub, s, ".", "x")
    r("badsubj escape " .. i, utf8.escape, s)
    r("badsubj insert " .. i, utf8.insert, s, 1, "x")
    r("badsubj remove " .. i, utf8.remove, s, 1, 1)
    r("badsubj codes " .. i, function() return iter(utf8.codes(s)) end)
    r("badsubj codes lax " .. i, function() return iter(utf8.codes(s, true)) end)
  end

  -- escape corner cases
  for _, s in ipairs({ "%", "%x", "%X", "%u", "%U", "%x4", "%xZZ", "%u{", "%u{41", "%u{4Z}", "%{", "%{41}", "%1", "%12", "%0", "%x{41}", "a%", "%%", "%%%" }) do
    r("escape " .. s, utf8.escape, s)
  end

  -- char / codepoint extremes
  for _, c in ipairs({ -1, 0, 127, 128, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, 0x7FFFFFFF, math.maxinteger }) do
    r("char " .. tostring(c), utf8.char, c)
    r("lower num " .. tostring(c), utf8.lower, c)
    r("width num " .. tostring(c), utf8.width, c)
  end
  r("char non-number", utf8.char, "x")
  r("char float", utf8.char, 65.0)
  r("char float frac", utf8.char, 65.5)
  r("width bad", utf8.width, true)

  -- huge numeric arguments
  r("sub huge", utf8.sub, "héllo", math.mininteger, math.maxinteger)
  r("codepoint huge", utf8.codepoint, "héllo", math.mininteger, math.maxinteger)
  r("byte huge", utf8.byte, "héllo", math.mininteger, math.maxinteger)
  r("len huge", utf8.len, "héllo", math.mininteger, math.maxinteger)
  r("offset huge", utf8.offset, "héllo", math.maxinteger)
  r("offset tiny", utf8.offset, "héllo", math.mininteger)
  r("widthindex huge", utf8.widthindex, "héllo", math.maxinteger)
  r("widthindex tiny", utf8.widthindex, "héllo", math.mininteger)
  r("ncasecmp bad", utf8.ncasecmp, "\xFF", "a")
  r("ncasecmp empty", utf8.ncasecmp, "", "")
  r("ncasecmp empty/a", utf8.ncasecmp, "", "a")
  r("ncasecmp a/empty", utf8.ncasecmp, "a", "")
end
