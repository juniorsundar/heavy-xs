-- W4: utf8extra. Baseline recorded from the C implementation.
--
-- Two kinds of observation:
--   * curated cases covering every exported function;
--   * digests over thousands of codepoints, which catch drift in the Unicode
--     tables (generated into src/unidata.rs from upstream's C unidata.h).
--
-- Every case goes through `val`, which records the pcall status and, on success,
-- the results. Error messages are deliberately not recorded: the C core raises
-- strings while mlua raises userdata, so message text is a known deviation
-- (see PLAN.md). Using pcall everywhere also means one unexpected raise cannot
-- abort the suite and silently truncate the baseline.

local utf8 = require "utf8extra"

local function val(fn, ...)
  local r = table.pack(pcall(fn, ...))
  local out = { tostring(r[1]) }
  if r[1] then
    for i = 2, r.n do out[#out + 1] = tostring(r[i]) end
  end
  return table.concat(out, "|")
end

local function iter(gen)
  local t = {}
  for a, b in gen do t[#t + 1] = tostring(a) .. "=" .. tostring(b) end
  return table.concat(t, "|")
end

-- FNV-1a over the results of fn for a spread of codepoints.
local SAMPLE = {}
for cp = 0, 0x2FFF do
  if not (cp >= 0xD800 and cp <= 0xDFFF) then SAMPLE[#SAMPLE + 1] = cp end
end
for cp = 0x3000, 0x10FFFF, 97 do
  if not (cp >= 0xD800 and cp <= 0xDFFF) then SAMPLE[#SAMPLE + 1] = cp end
end

local function digest(fn)
  local h = 2166136261
  local function mix(x)
    h = (h ~ (x & 0xFFFFFFFF)) & 0xFFFFFFFF
    h = (h * 16777619) & 0xFFFFFFFF
  end
  for _, cp in ipairs(SAMPLE) do
    local ok, s = pcall(utf8.char, cp)
    if ok then mix(fn(s)) end
  end
  return h
end

local STRINGS = {
  "", "a", "abc", "héllo", "日本語", "𝄞𝄞", "a\u{0301}",
  "\xFF", "\x80", "\xC3", "\xC0\x80", "\xE2\x82", "\xED\xA0\x80",
  "a\0b", "line\nbreak", "\t x ", "1234", " \u{3000} ",
}

return function(rec)
  local function r1(key, fn, ...) rec(key, val(fn, ...)) end

  r1("charpattern len", function() return #utf8.charpattern end)
  r1("charpattern bytes", function()
    return (utf8.charpattern:gsub(".", function(c) return string.format("%02X", c:byte()) end))
  end)

  -- Every accessor against every string, good and malformed.
  for i, s in ipairs(STRINGS) do
    r1("len[" .. i .. "]", utf8.len, s)
    r1("len lax[" .. i .. "]", utf8.len, s, 1, -1, true)
    r1("sub[" .. i .. "]", utf8.sub, s, 2, 3)
    r1("reverse[" .. i .. "]", utf8.reverse, s)
    r1("byte[" .. i .. "]", utf8.byte, s, 1, 3)
    r1("codepoint[" .. i .. "]", utf8.codepoint, s, 1, 3)
    r1("codepoint lax[" .. i .. "]", utf8.codepoint, s, 1, 3, true)
    r1("lower[" .. i .. "]", utf8.lower, s)
    r1("upper[" .. i .. "]", utf8.upper, s)
    r1("title[" .. i .. "]", utf8.title, s)
    r1("fold[" .. i .. "]", utf8.fold, s)
    r1("width[" .. i .. "]", utf8.width, s)
    r1("widthindex[" .. i .. "]", utf8.widthindex, s, 1)
    r1("offset[" .. i .. "]", utf8.offset, s, 1)
    r1("charpos[" .. i .. "]", utf8.charpos, s, 1)
    r1("next[" .. i .. "]", utf8.next, s)
    r1("codes[" .. i .. "]", function() return iter(utf8.codes(s)) end)
    r1("codes lax[" .. i .. "]", function() return iter(utf8.codes(s, true)) end)
    r1("find[" .. i .. "]", utf8.find, s, "%a")
    r1("find magic[" .. i .. "]", utf8.find, s, ".")
    r1("match[" .. i .. "]", utf8.match, s, "(.)")
    r1("gmatch[" .. i .. "]", function() return iter(utf8.gmatch(s, ".")) end)
    r1("gsub[" .. i .. "]", utf8.gsub, s, "a", "X")
    r1("ncasecmp[" .. i .. "]", utf8.ncasecmp, s, s)
  end

  -- codepoint / sub / offset / charpos argument variations
  r1("codepoint mixed", utf8.codepoint, "héllo", 1, -1)
  r1("codepoint neg", utf8.codepoint, "abc", -2)
  r1("codepoint empty", utf8.codepoint, "abc", 2, 1)
  r1("codepoint oor", utf8.codepoint, "abc", 0)
  r1("codepoint surrogate lax", utf8.codepoint, "\xED\xA0\x80", 1, 3, true)
  r1("codepoint surrogate strict", utf8.codepoint, "\xED\xA0\x80", 1, 3)
  for _, r in ipairs({ { 2, 4 }, { -3, -2 }, { 3 }, { 0, 2 }, { 0 }, { -1 }, { -20, 20 }, { 4, 2 } }) do
    r1("sub " .. r[1] .. "," .. tostring(r[2]), utf8.sub, "héllo", r[1], r[2])
  end
  for _, n in ipairs({ 1, 2, 3, -1, -2, 0, 9, -9 }) do
    r1("offset n=" .. n, utf8.offset, "héllo", n)
  end
  r1("offset from", utf8.offset, "héllo", 2, 3)
  r1("offset n=0 from", utf8.offset, "héllo", 0, 3)
  for _, n in ipairs({ 1, 2, -1, -9, 9 }) do
    r1("charpos n=" .. n, utf8.charpos, "héllo", n)
  end
  r1("charpos idx", utf8.charpos, "héllo", 1, 2)
  r1("next i=3", utf8.next, "héllo", 3)
  r1("next i=-1", utf8.next, "héllo", -1)
  r1("next i=99", utf8.next, "héllo", 99)
  r1("next lax", utf8.next, "\xFFa", 0, true)
  r1("next strict", utf8.next, "\xFFa", 0)

  -- char
  r1("char 1", utf8.char, 0x68)
  r1("char multi", utf8.char, 0x68, 0xE9, 0x65)
  r1("char max", utf8.char, 0x10FFFF)
  r1("char nul", utf8.char, 0)
  r1("char invalid", utf8.char, 0x110000)
  r1("char surrogate", utf8.char, 0xD800)
  r1("char neg", utf8.char, -1)

  -- case conversion
  for _, s in ipairs({ "HeLLo", "ÉÀ", "éà", "İ", "ß", "ΣΑΣ", "σας", "ǅǆ", "ЖУК", "ǰ", "ᾼ" }) do
    r1("lower " .. s, utf8.lower, s)
    r1("upper " .. s, utf8.upper, s)
    r1("title " .. s, utf8.title, s)
    r1("fold " .. s, utf8.fold, s)
  end

  -- escape / insert / remove
  for _, s in ipairs({ "a\\tb", "a\\65b", "a\\x41b", "\\u{48}", "\\65\\066\\9", "\\q", "\\u{1F600}", "\\u{}", "trailing\\" }) do
    r1("escape " .. s, utf8.escape, s)
    r1("escape nohex " .. s, utf8.escape, s, false)
  end
  for _, a in ipairs({ { 2, "X" }, { -1, "X" }, { 6, "X" }, { 3 }, { 0, "X" }, { 99, "X" } }) do
    r1("insert " .. a[1], utf8.insert, "héllo", a[1], a[2])
  end
  for _, a in ipairs({ { 2, 3 }, { 3 }, { -2 }, { 2, 99 }, { 0 } }) do
    r1("remove " .. a[1] .. "," .. tostring(a[2]), utf8.remove, "héllo", a[1], a[2])
  end

  -- width / ncasecmp
  r1("width ascii", utf8.width, "abc")
  r1("width cjk", utf8.width, "日本")
  r1("width mixed", utf8.width, "a日b")
  r1("width ambi", utf8.width, "±")
  for _, a in ipairs({ { "a日b", 2 }, { "±±", 1 }, { "héllo", 3 }, { "日本", 2 } }) do
    r1("widthindex " .. a[1] .. "," .. a[2], utf8.widthindex, a[1], a[2])
  end
  for _, a in ipairs({ { "Hello", "hello" }, { "Hello", "world" }, { "ÉÀ", "éà" }, { "abc", "ab" } }) do
    r1("ncasecmp " .. a[1] .. "," .. a[2], utf8.ncasecmp, a[1], a[2])
  end

  -- find / match / gmatch / gsub
  local PATTERNS = {
    "abc abc|%a+", "abc abc|(%a+)", "a1b2c3|%d", "  x  |%s", "a.b,c|%p",
    "0x1F|%w+", "x|%z", "hello world|(%a+) (%a+)", "abcabc|(abc)%1",
    "abcabc|()b", "abc|^a", "abc|c$", "ab(bc)cd|%b()", "hello world|%f[%a]%a+",
    "aXbXc|X", "日本語|.", "aaa|a-", "aaa|a*", "abc|[^b]+", "aBc|[%a]",
    "a\nb|.", "a\nb|%s", "x=1|(%a)=(%d)", "abc|b?", "text--more|--",
    "高|%a", "３４|%d", "abc|%u", "abc|%l", "abc|%c", "abc|%x", "abc|%g",
    "abc|%A", "abc|%D", "abc|%W", "%|%%", "abcde|%a%a", "aaa|^a$",
    "abc|[]]", "abc|[%^]", "-|%-",
  }
  for i, p in ipairs(PATTERNS) do
    local s, pat = p:match("^(.-)|(.*)$")
    r1("find[" .. i .. "]", utf8.find, s, pat)
    r1("match[" .. i .. "]", utf8.match, s, pat)
    r1("gmatch[" .. i .. "]", function() return iter(utf8.gmatch(s, pat)) end)
    r1("gsub[" .. i .. "]", utf8.gsub, s, pat, "X")
  end
  r1("find init", utf8.find, "abc abc", "%a+", 3)
  r1("find init neg", utf8.find, "abc abc", "%a+", -3)
  r1("find init plain", utf8.find, "abc abc", "abc", 1, true)
  r1("find init plain 2", utf8.find, "a.b a.b", "a.b", 1, true)
  r1("find anchor init", utf8.find, "abc", "^b", 2)
  r1("gmatch cap", function() return iter(utf8.gmatch("k=v;l=w", "(%a+)=(%a+)")) end)

  local GSUBS = {
    { "hello world", "%a+", "X" }, { "hello", "(l+)", "[%1]" },
    { "abc", "%a", "" }, { "hello", "l", "L", 1 }, { "hello", "l", "L", 2 },
    { "abc", "x", "y" }, { "hello", "l", "%%" }, { "abc", ".", "%0%0" },
    { "aaa", "", "-" }, { "abc", "()", "|" },
  }
  for i, g in ipairs(GSUBS) do
    r1("gsub-case[" .. i .. "]", utf8.gsub, g[1], g[2], g[3], g[4])
  end
  r1("gsub fn repl", utf8.gsub, "a1b2", "%d", function() return "#" end)
  r1("gsub table repl", utf8.gsub, "a1b2", "%d", { ["1"] = "one", ["2"] = "two" })
  r1("gsub bad repl", utf8.gsub, "a", "a", "%2")
  r1("gsub bad fn", utf8.gsub, "a", "a", function() error("boom") end)
  r1("gsub no string", utf8.gsub, "abc", "a", true)

  -- argument type errors: status only, but through val so the suite cannot abort
  r1("type len", utf8.len, 42)
  r1("type sub", utf8.sub, "abc", nil)
  r1("type find", utf8.find, "abc", nil)
  r1("type match", utf8.match, "abc", "(")
  r1("type byte", utf8.byte, 42)
  r1("type width", utf8.width, nil)
  r1("type reverse", utf8.reverse, {})
  r1("type lower", utf8.lower, 42)

  -- Unicode table digests
  r1("digest lower", function()
    return digest(function(s) return utf8.codepoint(utf8.lower(s), 1, -1, true) or 0 end)
  end)
  r1("digest upper", function()
    return digest(function(s) return utf8.codepoint(utf8.upper(s), 1, -1, true) or 0 end)
  end)
  r1("digest title", function()
    return digest(function(s) return utf8.codepoint(utf8.title(s), 1, -1, true) or 0 end)
  end)
  r1("digest fold", function()
    return digest(function(s) return utf8.codepoint(utf8.fold(s), 1, -1, true) or 0 end)
  end)
  r1("digest width", function() return digest(function(s) return utf8.width(s) end) end)
  for _, cls in ipairs({ "%a", "%d", "%l", "%u", "%s", "%p", "%c", "%x", "%w", "%g" }) do
    r1("digest " .. cls, function()
      return digest(function(s) return utf8.find(s, cls) and 1 or 0 end)
    end)
  end
end
