-- PROTOTYPE paired frame-timing probe. Appended to a throwaway copy of a user config, so the
-- real fonts, theme and plugins are active. Drives a maximized window through fixed phases and
-- records, per drawn frame: active work (core.step minus presentation) and presentation.
-- Env: FRAME_OUT (TSV path), FRAME_FILE (document), FRAME_WARM (frames, default 120),
--      FRAME_N (measured frames per phase, default 600).
local core = require "core"
local command = require "core.command"
local DocView = require "core.docview"

local out = assert(os.getenv("FRAME_OUT"), "FRAME_OUT required")
local file = assert(os.getenv("FRAME_FILE"), "FRAME_FILE required")
local warm = tonumber(os.getenv("FRAME_WARM") or "120")
local n = tonumber(os.getenv("FRAME_N") or "600")

local rows, phase = {}, "startup"
local step = core.step
core.step = function(...)
  local t0 = system.get_time()
  rawset(_G, "LXL_LAST_PRESENT_S", 0)
  local drew = step(...)
  local total = system.get_time() - t0
  if drew then
    local present = rawget(_G, "LXL_LAST_PRESENT_S") or 0
    rows[#rows + 1] = ("%s\t%.6f\t%.6f\t%.6f"):format(phase, (total - present) * 1e3, present * 1e3, total * 1e3)
  end
  return drew
end

-- Frames are paced by core.run (config.fps); every yield lets one frame happen.
local function frames(count, each)
  for i = 1, count do
    if each then each(i) end
    core.redraw = true
    coroutine.yield()
  end
end

core.add_thread(function()
  system.set_window_mode(core.window, "maximized")
  frames(30)
  core.root_view:open_doc(core.open_doc(file))
  local view = core.active_view
  assert(view:is(DocView), "document view")
  local lh = view:get_line_height()
  local lines = #view.doc.lines
  local function scroll(i)
    -- One line per frame, wrapping, instant (no smooth-scroll animation).
    view.scroll.to.y = ((i * lh) % math.max(lh, (lines - 80) * lh))
    view.scroll.y = view.scroll.to.y
  end

  phase = "warmup"; frames(warm, scroll)
  phase = "scroll"; frames(n, scroll)
  phase = "scroll_page"
  frames(n, function(i)
    view.scroll.to.y = ((i * 40 * lh) % math.max(lh, (lines - 80) * lh))
    view.scroll.y = view.scroll.to.y
  end)
  view.doc:set_selection(math.min(40, lines), 1)
  view.scroll.to.y, view.scroll.y = 0, 0
  phase = "type"
  frames(n, function(i)
    if i % 60 == 0 then
      command.perform("doc:newline")
    else
      view.doc:text_input(string.char(97 + i % 26))
    end
  end)
  phase = "idle_cursor"; frames(math.floor(n / 4))

  -- Record where the window landed so mismatched FreeType/candidate pairs can be rejected.
  local ww, wh = core.window:get_size()
  local m = assert(io.open(out .. ".meta", "w"))
  m:write(("window=%dx%d fps=%s\n"):format(ww, wh, tostring(require("core.config").fps)))
  m:close()
  local f = assert(io.open(out, "w"))
  f:write("phase\tactive_ms\tpresent_ms\ttotal_ms\n", table.concat(rows, "\n"), "\n")
  f:close()
  -- The typed text is throwaway: discard it without a save prompt.
  view.doc:clean()
  os.exit(0)
end)
