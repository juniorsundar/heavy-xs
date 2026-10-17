-- Bounded inventory checks, not a replacement-backend acceptance corpus.
local M = {}

function M.init()
  local renderer = require "renderer"
  local font = renderer.font
  local base = assert(os.getenv("LITE_PREFIX")) .. "/share/lite-xl/fonts/"
  local mono_path = base .. "JetBrainsMono-Regular.ttf"
  local mono = font.load(mono_path, 14)
  local fira = font.load(base .. "FiraSans-Regular.ttf", 14)
  local icons = font.load(base .. "icons.ttf", 14)
  local checks = 0
  local function check(name, ok)
    assert(ok, name)
    checks = checks + 1
    print("PASS " .. name)
  end

  local members = {mono, fira}
  local group = font.group(members)
  check("group returns original table", rawequal(group, members))
  check("group shares Font metatable", getmetatable(group) == getmetatable(mono))
  check("group height comes from first font", group:get_height() == mono:get_height())
  check("reverse group height comes from first font", font.group({fira, mono}):get_height() == fira:get_height())
  check("no Lua get_tab_size method", mono.get_tab_size == nil)
  check("copy defaults to height", mono:copy():get_size() == mono:get_height())
  check("explicit nil copy size raises", not pcall(mono.copy, mono, nil))

  local file = assert(io.tmpfile())
  local foreign = font.group({file})
  local usable, err = pcall(foreign.get_height, foreign)
  check("constructor admits foreign userdata but reader rejects it", not usable and tostring(err):find("Font", 1, true) ~= nil)
  assert(file:close())

  local path_with_nul = font.load(mono_path .. "\0ignored", 14)
  check("font paths stop at embedded NUL", path_with_nul:get_path() == mono_path)
  check("missing glyph uses last group font", font.group({fira, icons}):get_width("\u{4E2D}") == icons:get_width("\u{4E2D}"))
  check("actual glyph can come from second font", font.group({icons, fira}):get_width("é") == fira:get_width("é"))
  check("group whitespace uses first font", font.group({fira, icons}):get_width("\u{3000}") == fira:get_width(" "))
  check("zero-advance combining glyph receives space advance", fira:get_width("\u{301}") == fira:get_width(" "))
  check("width sums codepoints without pair kerning", fira:get_width("AV") == fira:get_width("A") + fira:get_width("V"))

  local reference = fira:get_width("AV")
  for _, aa in ipairs {"none", "grayscale", "subpixel"} do
    for _, hint in ipairs {"none", "slight", "full"} do
      local f = font.load(base .. "FiraSans-Regular.ttf", 14, {antialiasing = aa, hinting = hint})
      check("unhinted advances: " .. aa .. "/" .. hint, f:get_width("AV") == reference)
    end
  end
  print(("SUMMARY checks=%d failures=0"):format(checks))
  os.exit(0)
end

function M.run() end
function M.on_error(err)
  io.stderr:write("INVENTORY CHECK FAILED: ", tostring(err), "\n")
  os.exit(1)
end

return M
