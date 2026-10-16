-- W7 contract suite: fonts and text rendering (src/renderer.c, src/api/renderer.c).
--
-- Two halves. The API half records return values and error behaviour of the `renderer.font`
-- functions. The pixel half renders text into a real offscreen window and records an FNV-1a hash
-- of the resulting surface (colour only), so a glyph that is one pixel off, hinted differently,
-- blended differently or positioned differently changes the baseline.
--
-- Pixel hashes depend on the FreeType build. The baseline was recorded with the C core's
-- FreeType; see docs/rust-port/PLAN.md (W7) for what that means for the Rust port.

local renderer = require "renderer"
local renwindow = require "renwindow"

local W, H = 520, 140

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  if not r[1] then return "ERR " .. tostring(r[2]) end
  local out = {}
  for i = 2, r.n do
    local v = r[i]
    out[#out + 1] = type(v) == "number" and (math.type(v) .. ":" .. tostring(v)) or type(v) == "table" and "table" or type(v) == "userdata" and "userdata" or tostring(v)
  end
  return "ok " .. table.concat(out, ",")
end

return function(rec)
  assert(lxl_test and lxl_test.surface_hash, "needs the test_hooks cargo feature")
  local base = (os.getenv("LITE_PREFIX") or "") .. "/share/lite-xl/fonts/"
  local MONO, FIRA, ICONS = base .. "JetBrainsMono-Regular.ttf", base .. "FiraSans-Regular.ttf", base .. "icons.ttf"
  local load = renderer.font.load

  -- ------------------------------------------------------------------ API half
  local mono = load(MONO, 14)
  local fira = load(FIRA, 14)
  local icons = load(ICONS, 14)
  rec("mono size/height", mono:get_size(), mono:get_height())
  rec("fira size/height", fira:get_size(), fira:get_height())
  rec("icons size/height", icons:get_size(), icons:get_height())
  rec("mono path is the path", tostring(mono:get_path() == MONO))
  rec("mono widths A/AB/empty", mono:get_width("A"), mono:get_width("AB"), mono:get_width(""))
  rec("fira widths A/AV/space", fira:get_width("A"), fira:get_width("AV"), fira:get_width(" "))
  rec("icons width any", icons:get_width("A"), icons:get_width("\u{E000}"), icons:get_width(" "))
  rec("fira missing glyphs", fira:get_width("\u{25A1}"), fira:get_width("\u{4E2D}"), fira:get_width("\u{1F600}"))
  rec("width of NUL / controls", fira:get_width("\0"), fira:get_width("\1\2\3"))
  rec("width, invalid utf8", fira:get_width("\xff\xfe"), fira:get_width("a\xc3"), fira:get_width("\xe4\xb8"))
  rec("width, overlong/surrogate", fira:get_width("\xc0\x80"), fira:get_width("\xed\xa0\x80"))
  rec("width, max codepoint", fira:get_width("\xf4\x8f\xbf\xbf"), fira:get_width("\xf7\xbf\xbf\xbf"))

  -- Tabs: default stops, tab_offset, nan default, a tab straddling the space advance.
  rec("tab alone", mono:get_width("\t"), mono:get_width("\t", {tab_offset = 0}), mono:get_width("\t", {tab_offset = 3.5}))
  rec("tab after char", mono:get_width("A\tB"), mono:get_width("A\tB", {tab_offset = 0}), mono:get_width("A\tB", {tab_offset = 3.5}))
  rec("tab offsets", mono:get_width("\t", {tab_offset = 1}), mono:get_width("\t", {tab_offset = 15}), mono:get_width("\t", {tab_offset = -2}))
  rec("tab nearly at stop", mono:get_width("\t", {tab_offset = 16.8}), mono:get_width("\t", {tab_offset = 15.9}), mono:get_width("\t", {tab_offset = 8.4}))
  rec("tab, fira (proportional)", fira:get_width("\t"), fira:get_width("a\t", {tab_offset = 0}), fira:get_width("\t\t"))
  mono:set_tab_size(4)
  rec("tab size 4", mono:get_width("\t"), mono:get_width("A\tB", {tab_offset = 0}))
  mono:set_tab_size(0)
  -- A zero tab size makes the stop arithmetic fmod(x, 0), which is NaN. NaN's sign bit is an
  -- accident of the C library, so only "is it NaN" is part of the contract.
  local z1, z2 = mono:get_width("\t"), mono:get_width("A\tB", {tab_offset = 0})
  rec("tab size 0", z1, tostring(z2 ~= z2))
  mono:set_tab_size(8)
  rec("tab size 8", mono:get_width("\t"), mono:get_width("\t", {tab_offset = 3}))
  rec("tab_offset bad type", try(mono.get_width, mono, "\t", {tab_offset = "x"}))
  rec("tab arg bad type", try(mono.get_width, mono, "\t", 5))
  rec("tab arg nil", try(mono.get_width, mono, "\t", nil))
  mono:set_size(14)
  rec("set_size resets tab size", mono:get_width("\t"))
  mono:set_tab_size(3.9)
  rec("tab size float truncates", mono:get_width("\t"))
  mono:set_tab_size(2)

  -- set_size re-derives the metrics.
  local sized = load(MONO, 14)
  sized:set_size(30)
  rec("after set_size 30", sized:get_size(), sized:get_height(), sized:get_width("A"))
  sized:set_size(7.5)
  rec("after set_size 7.5", sized:get_size(), sized:get_height(), sized:get_width("A"))
  sized:set_size(1)
  rec("after set_size 1", sized:get_size(), sized:get_height(), sized:get_width("A"))
  rec("set_size 0", try(sized.set_size, sized, 0), sized:get_size(), sized:get_height())
  rec("set_size negative", try(sized.set_size, sized, -4))
  rec("set_size missing", try(sized.set_size, sized))
  rec("set_size huge", try(sized.set_size, sized, 400), sized:get_height())

  -- load: sizes and errors
  rec("load fractional size", load(MONO, 13.6):get_size(), load(MONO, 13.6):get_height(), load(MONO, 13.6):get_width("A"))
  rec("load tiny size", try(load, MONO, 1))
  rec("load size 0", try(load, MONO, 0))
  rec("load negative size", try(load, MONO, -5))
  rec("load size huge", try(load, MONO, 1000))
  rec("load no such file", try(load, "/nonexistent/font.ttf", 14))
  rec("load empty path", try(load, "", 14))
  rec("load a directory", (try(load, base, 14):gsub(base:gsub("(%W)", "%%%1"), "<fonts>/")))
  rec("load not a font", try(load, os.getenv("LITE_PREFIX") .. "/share/lite-xl/core/start.lua", 14))
  rec("load no args", try(load))
  rec("load missing size", try(load, MONO))
  rec("load size not a number", try(load, MONO, "big"))
  rec("load number path", try(load, 5, 14))

  -- options
  for _, opts in ipairs{
    {antialiasing = "none"}, {antialiasing = "grayscale"}, {antialiasing = "subpixel"},
    {hinting = "none"}, {hinting = "slight"}, {hinting = "full"},
    {italic = true}, {bold = true}, {underline = true}, {smoothing = true}, {strikethrough = true},
    {italic = true, bold = true}, {italic = false}, {bold = 1}, {},
    {antialiasing = 5}, {hinting = false}, {antialiasing = "none", hinting = "none", bold = true},
  } do
    local keys = {}
    for k, v in pairs(opts) do keys[#keys + 1] = k .. "=" .. tostring(v) end
    table.sort(keys)
    local ok, f = pcall(load, MONO, 14, opts)
    rec("options " .. (#keys > 0 and table.concat(keys, ",") or "{}"), ok and f:get_width("Wg") or "ERR", ok and f:get_height() or "")
  end
  rec("bad antialiasing", (select(2, pcall(load, MONO, 14, {antialiasing = "fancy"}))))
  -- NOTE: the C raises this message with a mis-typed printf argument (it formats the address of
  -- the enum, not the string), so only the fact that it raises is part of the contract.
  rec("bad hinting raises", tostring(not pcall(load, MONO, 14, {hinting = "fancy"})))
  rec("options not a table", try(load, MONO, 14, "x"))
  rec("options nil", try(load, MONO, 14, nil))

  -- copy
  local c1 = mono:copy(20)
  rec("copy size 20", c1:get_size(), c1:get_height(), c1:get_width("A"), tostring(c1 ~= mono))
  local c2 = mono:copy()
  rec("copy default size = height", c2:get_size(), c2:get_height())
  local c3 = fira:copy(14, {italic = true})
  rec("copy keeps path", c3:get_path() == FIRA, c3:get_width("Wg"))
  rec("copy keeps options (-1 sentinels)", mono:copy(14):get_width("Wg"), load(MONO, 14, {antialiasing = "none"}):copy(14):get_width("Wg"))
  rec("copy bad size", try(mono.copy, mono, "x"))
  rec("copy bad options", try(mono.copy, mono, 14, {antialiasing = "x"}))

  -- groups
  local group = renderer.font.group({mono, fira})
  rec("group type", type(group), tostring(getmetatable(group) == getmetatable(mono)))
  rec("group size/height/width", group:get_size(), group:get_height(), group:get_width("A"))
  rec("group width falls back", group:get_width("\u{E9}"), group:get_width("\u{4E2D}"), group:get_width("\u{25A1}"))
  rec("group path", try(function() local p = group:get_path(); return #p, p[1] == MONO, p[2] == FIRA end))
  rec("group copy", try(function() local g = group:copy(20); return type(g), #g, g[1]:get_size(), g[2]:get_size() end))
  rec("group tab size", try(function() group:set_tab_size(5); return group:get_width("\t") end))
  rec("group set_size", try(function() group:set_size(22); return group:get_size(), group:get_height(), group[1]:get_size(), group[2]:get_size() end))
  rec("group of one", try(function() return renderer.font.group({mono}):get_height() end))
  rec("group empty", try(renderer.font.group, {}))
  rec("group non-table", try(renderer.font.group, mono))
  rec("group with non-font", try(renderer.font.group, {mono, 5}))
  rec("group with nested group", try(renderer.font.group, {mono, group}))
  local ten, eleven = {}, {}
  for i = 1, 11 do eleven[i] = mono; if i <= 10 then ten[i] = mono end end
  rec("group of 10", try(function() return renderer.font.group(ten):get_height() end))
  rec("group of 11", try(renderer.font.group, eleven))
  rec("group of 11 as a plain table", try(function() return setmetatable({table.unpack(eleven)}, getmetatable(mono)):get_height() end))
  rec("groups gc cleanly", try(function() for _ = 1, 50 do renderer.font.group({load(MONO, 9), load(FIRA, 9)}) end collectgarbage() return "ok" end))

  -- Colours and argument checking on the drawing calls (outside a frame there is no target).
  rec("draw_text outside a frame", try(renderer.draw_text, mono, "abc", 5, 5, {255, 255, 255}))
  rec("draw_text returns x", try(renderer.draw_text, mono, "abc", 5.5, 5, {255, 255, 255}))
  rec("draw_text group", try(renderer.draw_text, group, "a\u{4E2D}", 0, 0))
  rec("draw_text no color", try(renderer.draw_text, mono, "abc", 0, 0))
  rec("draw_text bad color value", try(renderer.draw_text, mono, "abc", 0, 0, {255, "x", 255}))
  rec("draw_text color short", try(renderer.draw_text, mono, "abc", 0, 0, {255, 255}))
  rec("draw_text color not table", try(renderer.draw_text, mono, "abc", 0, 0, 5))
  rec("draw_text color userdata value", try(renderer.draw_text, mono, "abc", 0, 0, {255, mono, 255}))
  rec("draw_text color boolean value", try(renderer.draw_text, mono, "abc", 0, 0, {255, 255, 255, true}))
  rec("draw_text color floats", try(renderer.draw_text, mono, "abc", 0, 0, {255.9, 0.5, -1, 300}))
  rec("draw_text font missing", try(renderer.draw_text))
  rec("draw_text font wrong type", try(renderer.draw_text, 5, "abc", 0, 0))
  rec("draw_text text not string", try(renderer.draw_text, mono, {}, 0, 0))
  rec("draw_text number text", try(renderer.draw_text, mono, 12, 0, 0))
  rec("draw_text y float", try(renderer.draw_text, mono, "x", 0, 3.9))
  rec("draw_text empty string", try(renderer.draw_text, mono, "", 7, 0))
  rec("draw_rect bad color", try(renderer.draw_rect, 0, 0, 1, 1, {1, 2, "z"}))
  rec("set_clip_rect bad arg", try(renderer.set_clip_rect, 0, 0, 1, "x"))
  rec("show_debug no arg", try(renderer.show_debug))
  rec("show_debug false", try(renderer.show_debug, false))
  rec("get_size outside frame", try(renderer.get_size))

  -- ----------------------------------------------------------------- pixel half
  local win = renwindow.create("renderer-contract", W, H)
  local function shot(name, fn)
    renderer.begin_frame(win)
    renderer.draw_rect(0, 0, W, H, {0, 0, 0, 255})
    local ok, err = pcall(fn)
    renderer.end_frame()
    rec(name, ok and lxl_test.surface_hash(win, 0, 0, W, H) or ("ERR " .. tostring(err)))
  end
  local WHITE = {255, 255, 255, 255}
  local TEXT = "Hello, w\u{F8}rld! gjpqy {}[] 0123"

  -- Font x antialiasing x hinting at one size: the rasteriser's main matrix.
  for _, face in ipairs{{"mono", MONO}, {"fira", FIRA}} do
    for _, aa in ipairs{"none", "grayscale", "subpixel"} do
      for _, hint in ipairs{"none", "slight", "full"} do
        local f = load(face[2], 14, {antialiasing = aa, hinting = hint})
        shot(("%s %s %s"):format(face[1], aa, hint), function()
          renderer.draw_text(f, TEXT, 8, 10, WHITE)
        end)
      end
    end
  end

  -- Sizes (grayscale, slight) and subpixel positioning: the pen's fractional part picks one of
  -- three cached bitmaps in subpixel mode.
  for _, size in ipairs{6, 8, 11, 13.5, 18, 24, 36, 48} do
    local f = load(FIRA, size, {antialiasing = "grayscale"})
    shot("size " .. size, function() renderer.draw_text(f, "Rag Qy@", 6, 4, WHITE) end)
  end
  local sub = load(MONO, 16, {antialiasing = "subpixel", hinting = "slight"})
  for _, x in ipairs{10, 10.2, 10.34, 10.5, 10.67, 10.99} do
    shot("subpixel x=" .. x, function() renderer.draw_text(sub, "iIl1|.:", x, 10, WHITE) end)
  end
  shot("subpixel string drift", function() renderer.draw_text(load(FIRA, 15, {antialiasing = "subpixel"}), "The quick brown fox jumps over the lazy dog", 5.3, 10, WHITE) end)

  -- Styles.
  for _, style in ipairs{
    {"bold", {bold = true}}, {"italic", {italic = true}}, {"bold italic", {bold = true, italic = true}},
    {"smoothing", {smoothing = true}}, {"underline", {underline = true}}, {"strikethrough", {strikethrough = true}},
    {"underline+strike", {underline = true, strikethrough = true}},
    {"all", {bold = true, italic = true, smoothing = true, underline = true, strikethrough = true}},
  } do
    for _, aa in ipairs{"subpixel", "none"} do
      local opts = {antialiasing = aa}
      for k, v in pairs(style[2]) do opts[k] = v end
      local f = load(FIRA, 16, opts)
      shot(("style %s (%s)"):format(style[1], aa), function() renderer.draw_text(f, "Styled Wgq fi", 6, 10, WHITE) end)
    end
  end

  -- Colour and alpha: the blend is (c*s*a + d*(65025 - s*a) + 32767) / 65025, per channel.
  local g14 = load(FIRA, 20, {antialiasing = "grayscale"})
  local s14 = load(FIRA, 20, {antialiasing = "subpixel"})
  for _, c in ipairs{
    {255, 0, 0, 255}, {10, 200, 90, 255}, {255, 255, 255, 128}, {255, 255, 255, 1}, {255, 255, 255, 254},
    {200, 100, 50, 77}, {255, 255, 255, 0}, {0, 0, 0, 255},
  } do
    shot("color " .. table.concat(c, ","), function()
      renderer.draw_rect(0, 0, W, H, {90, 90, 90, 255})
      renderer.draw_text(g14, "Blend Qg", 6, 10, c)
      renderer.draw_text(s14, "Blend Qg", 6, 60, c)
    end)
  end
  shot("rect alpha over rect", function()
    renderer.draw_rect(10, 10, 100, 60, {200, 0, 0, 255})
    renderer.draw_rect(60, 30, 100, 60, {0, 0, 200, 128})
    renderer.draw_rect(5, 5, 300, 20, {0, 255, 0, 1})
    renderer.draw_rect(5, 100, 300, 20, {255, 255, 255, 254})
  end)
  shot("rect alpha, odd sizes", function()
    for i = 0, 6 do renderer.draw_rect(10 + i * 30, 10 + i * 3, 17 + i, 9 + i, {255, 128, 0, 30 + i * 30}) end
  end)

  -- Clipping: partial glyphs at each edge, and the "start_x < clip.x" adjustment.
  local cf = load(MONO, 24, {antialiasing = "grayscale"})
  shot("clip left", function() renderer.set_clip_rect(40, 0, W, H); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip right", function() renderer.set_clip_rect(0, 0, 70, H); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip top", function() renderer.set_clip_rect(0, 20, W, H); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip bottom", function() renderer.set_clip_rect(0, 0, W, 25); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip window", function() renderer.set_clip_rect(30, 12, 60, 14); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip empty", function() renderer.set_clip_rect(0, 0, 0, 0); renderer.draw_text(cf, "ABCDEFGH", 10, 10, WHITE) end)
  shot("clip subpixel", function() renderer.set_clip_rect(33, 0, 41, H); renderer.draw_text(sub, "wwwwwwwwwwww", 10.4, 10, WHITE) end)
  shot("text starting off the left edge", function() renderer.draw_text(cf, "ABCDEFGH", -30, 10, WHITE) end)
  shot("text off the right edge", function() renderer.draw_text(cf, "ABCDEFGH", W - 20, 10, WHITE) end)
  shot("text above/below", function() renderer.draw_text(cf, "AB", 5, -10, WHITE); renderer.draw_text(cf, "AB", 50, H - 8, WHITE) end)

  -- Tabs in rendered text and underline across fallbacks.
  shot("tabs rendered", function()
    local f = load(MONO, 14, {antialiasing = "grayscale"})
    renderer.draw_text(f, "a\tb\tc", 5, 5, WHITE)
    renderer.draw_text(f, "a\tb\tc", 5, 30, WHITE, {tab_offset = 3})
    renderer.draw_text(f, "\t\tx", 5, 55, WHITE, {tab_offset = 0})
    f:set_tab_size(5)
    renderer.draw_text(f, "a\tb\tc", 5, 80, WHITE)
  end)

  -- Fallback groups: first font with the glyph wins, whitespace always uses the first font, and
  -- a missing glyph falls back to the box U+25A1 (or, if nothing has it, draws a filled box).
  local gm = renderer.font.group({load(MONO, 16, {antialiasing = "grayscale"}), load(FIRA, 16, {antialiasing = "grayscale"})})
  local gi = renderer.font.group({load(FIRA, 16, {antialiasing = "grayscale"}), load(ICONS, 16, {antialiasing = "grayscale"})})
  shot("group mono+fira", function() renderer.draw_text(gm, "caf\u{E9} \u{20AC}5 \u{4E2D}\u{6587}", 6, 10, WHITE) end)
  shot("group fira+icons", function() renderer.draw_text(gi, "A\u{E000}B\u{F000}C \u{4E2D}", 6, 10, WHITE) end)
  shot("single font, missing glyph", function() renderer.draw_text(load(FIRA, 16, {antialiasing = "grayscale"}), "a\u{4E2D}b\u{1F600}c", 6, 10, WHITE) end)
  shot("box glyph itself missing", function() renderer.draw_text(load(FIRA, 16, {antialiasing = "grayscale"}), "a\u{25A1}b", 6, 10, WHITE) end)
  shot("whitespace kinds", function()
    renderer.draw_text(gm, "a b\u{A0}c\u{3000}d\u{2003}e\u{85}f", 6, 10, WHITE)
    renderer.draw_text(gi, "a b\u{A0}c\u{3000}d\u{2003}e", 6, 40, WHITE)
  end)
  shot("group underline", function()
    local f1 = load(MONO, 16, {underline = true, antialiasing = "grayscale"})
    local f2 = load(FIRA, 16, {underline = true, antialiasing = "grayscale"})
    renderer.draw_text(renderer.font.group({f1, f2}), "ab\u{4E2D}\u{6587}cd \u{E9}", 6, 10, WHITE)
  end)
  shot("group strikethrough mixed", function()
    local f1 = load(MONO, 16, {strikethrough = true, antialiasing = "grayscale"})
    local f2 = load(FIRA, 16, {antialiasing = "grayscale"})
    renderer.draw_text(renderer.font.group({f1, f2}), "ab\u{E9}cd", 6, 10, WHITE)
  end)

  -- Odd input.
  local of = load(FIRA, 14, {antialiasing = "grayscale"})
  shot("empty string", function() renderer.draw_text(of, "", 6, 10, WHITE) end)
  shot("invalid utf8", function() renderer.draw_text(of, "a\xffb\xc3c\xe4\xb8d", 6, 10, WHITE) end)
  shot("controls", function() renderer.draw_text(of, "a\1\2\31b\127c", 6, 10, WHITE) end)
  shot("NUL inside", function() renderer.draw_text(of, "ab\0cd", 6, 10, WHITE) end)
  shot("long line", function() renderer.draw_text(of, string.rep("The quick brown fox ", 12), 0, 10, WHITE) end)
  shot("many small draws", function()
    for i = 0, 40 do renderer.draw_text(of, tostring(i), (i % 10) * 50, 5 + (i // 10) * 22, WHITE) end
  end)
  shot("same font after set_size", function()
    local f = load(MONO, 12, {antialiasing = "grayscale"})
    renderer.draw_text(f, "before", 6, 5, WHITE)
    f:set_size(22)
    renderer.draw_text(f, "after", 6, 30, WHITE)
  end)
  shot("font copy renders like original", function()
    local f = load(FIRA, 15, {antialiasing = "grayscale"})
    renderer.draw_text(f, "Copy test", 6, 5, WHITE)
    renderer.draw_text(f:copy(15), "Copy test", 6, 40, WHITE)
  end)

  -- Return value: the pen position after the run, which callers use to chain draws.
  renderer.begin_frame(win)
  rec("draw_text return, mono", renderer.draw_text(mono, "abc", 10, 10, WHITE))
  rec("draw_text return, fira+tab", renderer.draw_text(fira, "a\tb", 10.5, 10, WHITE, {tab_offset = 2}))
  rec("draw_text return, group", renderer.draw_text(gm, "a\u{E9}\u{4E2D}", 3, 10, WHITE))
  rec("draw_text return, empty", renderer.draw_text(mono, "", 3.25, 10, WHITE))
  renderer.end_frame()

  -- Deferred draws must keep temporary fonts/groups alive until replay, then release them.
  local weak = setmetatable({}, {__mode = "v"})
  renderer.begin_frame(win)
  renderer.draw_rect(0, 0, W, H, {0, 0, 0})
  do
    local temporary = renderer.font.group({load(FIRA, 14), load(ICONS, 14)})
    weak[1] = temporary
    renderer.draw_text(temporary, "GC lifetime", 10, 10, WHITE)
  end
  collectgarbage()
  assert(weak[1], "font group collected before replay")
  renderer.end_frame()
  assert(tonumber(lxl_test.surface_hash(win, 0, 0, W, H):match("/(%d+)$")) > 0)
  collectgarbage()
  assert(not weak[1], "frame font references leaked")

  -- Rust rejects malformed font tables rather than dereferencing a null font.
  assert(not pcall(renderer.font.get_height, {}))
  assert(not pcall(renderer.font.get_width, {true}, "x"))
end
