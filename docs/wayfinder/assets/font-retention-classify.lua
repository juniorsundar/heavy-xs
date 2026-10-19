-- Font retention classification probe for docs/wayfinder/issues/19-classify-font-memory-retention.md.
-- Evidence only; adapted from freetype-baseline.lua (same load/frame/glyph helpers and cycle).
-- Env: RETAIN_OUT=existing dir; RETAIN_CASE=mono|custom_cjk; RETAIN_VARIANT=full|load|width|draw|
--      sameload|frames|drawsame; RETAIN_CYCLES (default 100); RETAIN_EVERY (default 10); RETAIN_TAG (file label).
-- Optional LD_PRELOAD=mallinfo-shim.so adds glibc mallinfo2 columns and a malloc_trim(0) sample.
local renderer, renwindow = require "renderer", require "renwindow"
local M = {}
local base = (os.getenv("LITE_PREFIX") or "") .. "/share/lite-xl/fonts/"
local api, checks = nil, 0

local function check(ok, name) assert(ok, name); checks = checks + 1 end
local function gc() collectgarbage("collect"); collectgarbage("collect") end
local function load(path, size) return renderer.font.load(assert(path), size or 16, {antialiasing="grayscale"}) end
local function frame(win, w, h, f, text, shade)
  renderer.begin_frame(win)
  renderer.draw_rect(0,0,w,h,{shade,shade,shade,255})
  if f then renderer.draw_text(f,text,4,4,{255,255,255,255}) end
  renderer.end_frame()
end
local function status()
  local file = assert(io.open("/proc/self/status","r")); local s = file:read("a"); file:close()
  return assert(tonumber(s:match("\nVmRSS:%s+(%d+) kB"))), assert(tonumber(s:match("\nVmHWM:%s+(%d+) kB")))
end
local function fds()
  local n = 0
  local p = io.popen("ls /proc/" .. assert(io.open("/proc/self/stat"):read("l"):match("^(%d+)")) .. "/fd | wc -l")
  if p then n = tonumber(p:read("a")) or -1; p:close() end
  return n
end
local function smaps_dump(name, out)
  local src = assert(io.open("/proc/self/smaps","r")); local s = src:read("a"); src:close()
  local dst = assert(io.open(out .. "/" .. name, "wb")); dst:write(s); dst:close()
end
local function rollup()
  local f = assert(io.open("/proc/self/smaps_rollup","r")); local s = f:read("a"); f:close()
  return tonumber(s:match("\nAnonymous:%s+(%d+)")) or -1, (tonumber(s:match("\nRss:%s+(%d+)")) or 0) - (tonumber(s:match("\nAnonymous:%s+(%d+)")) or 0)
end
local function malloc_stats(which)
  local f = io.open("/lxl-malloc/" .. which, "r")
  if not f then return nil end
  local s = f:read("a"); f:close()
  local t = {}
  for k, v in s:gmatch("(%w+)=(%-?%d+)") do t[k] = tonumber(v) end
  return t
end

function M.init()
  api = assert(lxl_test, "requires test_hooks")
  local out = assert(os.getenv("RETAIN_OUT"), "RETAIN_OUT required")
  local id = os.getenv("RETAIN_CASE") or "mono"
  local variant = os.getenv("RETAIN_VARIANT") or "full"
  local cycles = tonumber(os.getenv("RETAIN_CYCLES") or "100")
  local every = tonumber(os.getenv("RETAIN_EVERY") or "10")
  local tag = os.getenv("RETAIN_TAG") or "default"
  local path = id == "mono" and base .. "JetBrainsMono-Regular.ttf" or os.getenv("FONT_BASELINE_CJK")
  assert(path and io.open(path, "rb"), "font not available: " .. id)
  assert(({full=1, load=1, width=1, draw=1, sameload=1, frames=1, drawsame=1, hold8=1})[variant], "invalid RETAIN_VARIANT")
  local w, h = tonumber(os.getenv("RETAIN_W") or "960"), tonumber(os.getenv("RETAIN_H") or "140")
  local win = renwindow.create("font-retention", w, h)
  local shade, rows = 0, {}
  local function draw(f, text) shade = 1 - shade; frame(win, w, h, f, text, shade) end
  local function glyphs(n)
    local chars = {}
    for i = 1, n do chars[i] = utf8.char((id == "custom_cjk" and 0x4e00 or 32) + i - 1) end
    return table.concat(chars)
  end
  local function record(phase)
    gc()
    local rss, hwm = status()
    local m = malloc_stats("stats")
    rows[#rows+1] = table.concat({id, variant, tag, phase, rss, hwm, ("%.1f"):format(collectgarbage("count")),
      m and m.arena or -1, m and m.hblkhd or -1, m and m.uordblks or -1, m and m.fordblks or -1, fds(), (function() local a, f = rollup(); return a .. "\t" .. f end)()}, "\t")
  end
  local weak = setmetatable({}, {__mode="v"})
  draw(nil); record("baseline")
  if os.getenv("RETAIN_SMAPS") then smaps_dump(("smaps-%s-%s-%s-baseline.txt"):format(id, variant, tag), out) end
  -- `sameload`: one font kept alive for the whole run, re-drawn each cycle (control: no new fonts).
  local keeper = variant == "sameload" and load(path) or nil
  for cycle = 1, cycles do
    do
      if variant == "hold8" then
        local held = {}
        for i = 1, 8 do held[i] = load(path, 12 + i); weak[i] = held[i]; draw(held[i], glyphs(64)) end
        held = nil
      elseif variant == "drawsame" then
        local font = load(path); weak[1] = font
        draw(font, glyphs(64)); draw(font, glyphs(64)) -- second draw: same glyphs, cache hit, still a frame
      elseif variant == "frames" then
        draw(nil) -- control: no font at all, one extra frame
      elseif variant == "sameload" then
        draw(keeper, glyphs(64))
      else
        local font = load(path)
        weak[1] = font
        if variant == "full" then
          font:set_size(16 + (cycle % 8) + 1)
          local copy = font:copy(16 + (cycle % 8) + 1)
          weak[2] = copy
          draw(font, glyphs(64)); draw(copy, glyphs(64))
        elseif variant == "width" then
          font:get_width(glyphs(64))
        elseif variant == "draw" then
          draw(font, glyphs(tonumber(os.getenv("RETAIN_GLYPHS") or "64")))
        end -- "load": load only
      end
    end
    draw(nil); gc()
    if variant ~= "sameload" and variant ~= "frames" then check(next(weak) == nil, "cycle fonts released") end
    if cycle % every == 0 or cycle == 1 then record("cycle" .. cycle) end
  end
  keeper = nil; draw(nil); gc(); record("end")
  if os.getenv("RETAIN_SMAPS") then smaps_dump(("smaps-%s-%s-%s-end.txt"):format(id, variant, tag), out) end
  local trimmed = malloc_stats("trim")
  if trimmed then record("after-trim") end
  local f = assert(io.open(("%s/retain-%s-%s-%s.tsv"):format(out, id, variant, tag), "wb"))
  f:write("id\tvariant\ttag\tphase\trss_kb\thwm_kb\tlua_kb\tarena_b\tmmap_b\tinuse_b\tfree_b\tfds\tanon_kb\tfile_kb\n", table.concat(rows, "\n"), "\n")
  f:close()
  io.stdout:write(("RETAIN id=%s variant=%s tag=%s cycles=%d rows=%d checks=%d\n"):format(id, variant, tag, cycles, #rows, checks))
  os.exit(0)
end
function M.run() end
function M.on_error(err) io.stderr:write("RETAIN FAILED: ", tostring(err), "\n"); os.exit(1) end
return M
