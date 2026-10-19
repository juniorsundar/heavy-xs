-- PROTOTYPE geometry probe: run once per backend (LITE_FONT_BACKEND=freetype|candidate) and diff.
-- Invariants are asserted; numbers are printed for the paired FreeType/candidate comparison.
-- PROBE_FONTS=id=path;id=path (absolute paths; ids only are printed, never paths).
local renderer, renwindow = require "renderer", require "renwindow"
local M = {}
local checks = 0

local function check(name, ok)
  assert(ok, name)
  checks = checks + 1
end

local SAMPLES = {
  code = "local function f(x) return x * 2 end -- ok",
  ui = "The quick brown fox jumps over 0123456789",
  tabs = "a\tbb\tccc\td",
  mixed = "naïve café ≠ ∑ → 中文 😀",
}
local MODES = {}
for _, aa in ipairs {"none", "grayscale", "subpixel"} do
  for _, hint in ipairs {"none", "slight", "full"} do MODES[#MODES + 1] = {aa, hint} end
end

function M.init()
  local base = assert(os.getenv("LITE_PREFIX")) .. "/share/lite-xl/fonts/"
  local fonts = {
    {"jetbrains_mono", base .. "JetBrainsMono-Regular.ttf"},
    {"fira_sans", base .. "FiraSans-Regular.ttf"},
  }
  for entry in (os.getenv("PROBE_FONTS") or ""):gmatch("[^;]+") do
    local id, path = entry:match("^([%w_]+)=(.+)$")
    fonts[#fonts + 1] = {assert(id), path}
  end
  local icons = renderer.font.load(base .. "icons.ttf", 15)
  local win = renwindow.create("geometry-probe", 1200, 80)
  print(("backend=%s"):format(os.getenv("LITE_FONT_BACKEND") or "freetype"))
  for _, spec in ipairs(fonts) do
    for _, size in ipairs {12, 15, 19} do
      for _, mode in ipairs(MODES) do
        local f = renderer.font.load(spec[2], size, {antialiasing = mode[1], hinting = mode[2]})
        local row = {spec[1], size, mode[1] .. "/" .. mode[2], "h=" .. f:get_height()}
        for _, key in ipairs {"code", "ui", "tabs", "mixed"} do
          local text = SAMPLES[key]
          local w = f:get_width(text)
          -- Prefix widths are monotonic and sum per codepoint (no kerning/shaping).
          local sum, prev = 0, 0
          for _, cp in utf8.codes(text) do
            local ch = utf8.char(cp)
            if ch ~= "\t" then sum = sum + f:get_width(ch) end
          end
          if not text:find("\t") then
            check(spec[1] .. " codepoint sum " .. key, math.abs(sum - w) < 1e-6)
          end
          for i = 1, #text do
            if utf8.len(text:sub(1, i)) then
              local pw = f:get_width(text:sub(1, i))
              check(spec[1] .. " monotonic prefix " .. key, pw >= prev - 1e-9)
              prev = pw
            end
          end
          -- draw_text returns the same end x that get_width predicts.
          renderer.begin_frame(win)
          local x_end = renderer.draw_text(f, text, 7.25, 10, {255, 255, 255, 255})
          renderer.end_frame()
          check(spec[1] .. " draw end x == width " .. key, math.abs((x_end - 7.25) - w) < 1e-4)
          row[#row + 1] = ("%s=%.4f"):format(key, w)
        end
        -- Fallback: a group with icons resolves missing glyphs in the last font.
        local group = renderer.font.group {f, icons}
        row[#row + 1] = ("grp=%.4f"):format(group:get_width("A\u{E000}中"))
        print(table.concat(row, " "))
      end
    end
  end
  print(("SUMMARY checks=%d"):format(checks))
  os.exit(0)
end

function M.run() end
function M.on_error(err)
  io.stderr:write("PROBE FAILED: ", tostring(err), "\n")
  os.exit(1)
end
return M
