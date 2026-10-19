-- PROTOTYPE malformed-font probe: run per backend and per case (one process per case, so a
-- crash or hang is attributed to exactly one input). Each case loads a mutated font, then, if
-- the load succeeded, measures and draws text including malformed UTF-8 at several sizes.
-- Env: MAL_FONT (mutated font path), MAL_ID (case label). Prints one RESULT line.
local renderer, renwindow = require "renderer", require "renwindow"
local M = {}

function M.init()
  local path, id = assert(os.getenv("MAL_FONT")), assert(os.getenv("MAL_ID"))
  local win = renwindow.create("malformed", 400, 120)
  local ok, font = pcall(renderer.font.load, path, 15)
  local outcome
  if not ok then
    -- Error contract: Lua error with the existing prefix; detail may differ by backend.
    assert(tostring(font):find("failed to load font: ", 1, true), "error prefix: " .. tostring(font))
    outcome = "load-error"
  else
    local texts = {"Hello, world", "\xff\xfe\xc0\x80 bad utf8", "中文 😀 \u{25A1}", ("W"):rep(200)}
    for _, size in ipairs {1, 8, 15, 64, 300} do
      font:set_size(size)
      for _, t in ipairs(texts) do
        local w = font:get_width(t)
        assert(w == w and w >= 0 and w < 1e7, "width finite")
        renderer.begin_frame(win)
        renderer.draw_rect(0, 0, 400, 120, {0, 0, 0, 255})
        renderer.draw_text(font, t, 4, 4, {255, 255, 255, 255})
        renderer.end_frame()
      end
      for _, aa in ipairs {"none", "grayscale", "subpixel"} do
        local c = font:copy(size, {antialiasing = aa, bold = true, italic = true})
        renderer.begin_frame(win)
        renderer.draw_text(c, texts[1], 4, 4, {255, 255, 255, 255})
        renderer.end_frame()
      end
    end
    outcome = "loaded-drew"
  end
  print(("RESULT %s %s"):format(id, outcome))
  os.exit(0)
end

function M.run() end
function M.on_error(err)
  print(("RESULT %s lua-error %s"):format(os.getenv("MAL_ID") or "?", tostring(err):gsub("%s+", " "):sub(1, 160)))
  os.exit(3)
end
return M
