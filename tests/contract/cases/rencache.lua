-- W5/W6 contract suite: the renderer command cache (src/rencache.c).
--
-- rencache batches a frame's drawing commands, hashes them into a grid of 96x96 cells, and
-- redraws only the cells that changed since the previous frame. The dirty rect list is the
-- observable that matters: it decides what gets repainted, and it must be identical between the
-- C and Rust implementations for the same scene.
--
-- A cell is 96x96 and the grid is 80x50 cells. The window below is 800x480 points, so the grid
-- covers 9x6 cells. Every rect here is chosen to land on a known cell boundary so the baseline
-- is readable.

local renderer = require "renderer"
local renwindow = require "renwindow"

local function scene(reset)
  if reset then
    renderer.show_debug(false)
  end
end

return function(rec)
  assert(lxl_test and lxl_test.dirty_rects, "needs the test_hooks cargo feature")
  local win = renwindow.create("rencache-contract", 800, 480)
  -- DATADIR is a global the normal runtime sets from LITE_PREFIX; the contract runtime bypasses
  -- core.init, so build the path here.
  local font = renderer.font.load(
    (os.getenv("LITE_PREFIX") or "") .. "/share/lite-xl/fonts/JetBrainsMono-Regular.ttf", 14)

  local function frame(fn)
    renderer.begin_frame(win)
    fn()
    renderer.end_frame()
    return lxl_test.dirty_rects()
  end

  scene(true)
  -- Outside a frame there is no target window.
  rec("size outside frame", tostring(renderer.get_size()))

  -- The surface size, in points, seen from inside a frame (get_size reads the target window).
  renderer.begin_frame(win)
  rec("size in frame", tostring(renderer.get_size()))
  renderer.end_frame()
  -- The first two frames repaint the whole screen (begin_frame invalidates the grid when the
  -- size changes from 0x0, and the previous grid is still being primed); from the third frame on
  -- an unchanged scene produces no dirty rects at all. The baseline shows which is which.
  frame(function() end)
  rec("second frame", lxl_test.dirty_rects())
  frame(function() end)
  rec("third frame", lxl_test.dirty_rects())
  frame(function() end)
  rec("fourth frame", lxl_test.dirty_rects())

  -- A rect in the first cell. Only cell (0,0) is dirty.
  rec("one rect", frame(function() renderer.draw_rect(10, 10, 20, 20, {255, 0, 0, 255}) end))
  -- The same rect again: still no change.
  rec("same rect", frame(function() renderer.draw_rect(10, 10, 20, 20, {255, 0, 0, 255}) end))
  -- Move it into cell (1,0): two cells change, and the merge joins them into one rect.
  rec("rect moved one cell", frame(function() renderer.draw_rect(100, 10, 20, 20, {255, 0, 0, 255}) end))
  -- Back: again two cells.
  rec("rect moved back", frame(function() renderer.draw_rect(10, 10, 20, 20, {255, 0, 0, 255}) end))

  -- A rect spanning several cells: all of them are dirty, merged into one rect.
  rec("wide rect", frame(function() renderer.draw_rect(10, 10, 400, 10, {0, 255, 0, 255}) end))
  rec("wide rect again", frame(function() renderer.draw_rect(10, 10, 400, 10, {0, 255, 0, 255}) end))
  -- Changing only the colour of part of it dirties just the cells that overlap.
  rec("wide rect recoloured", frame(function() renderer.draw_rect(10, 10, 400, 10, {0, 0, 255, 255}) end))

  -- Two disjoint rects in distant cells: two dirty rects that cannot merge.
  rec("two far rects", frame(function()
    renderer.draw_rect(10, 10, 10, 10, {255, 255, 0, 255})
    renderer.draw_rect(700, 400, 10, 10, {0, 255, 255, 255})
  end))
  rec("two far rects again", frame(function()
    renderer.draw_rect(10, 10, 10, 10, {255, 255, 0, 255})
    renderer.draw_rect(700, 400, 10, 10, {0, 255, 255, 255})
  end))

  -- Zero-size rects are dropped before they can dirty anything.
  rec("zero-size rect", frame(function() renderer.draw_rect(10, 10, 0, 0, {255, 0, 0, 255}) end))
  rec("zero-height rect", frame(function() renderer.draw_rect(10, 10, 50, 0, {255, 0, 0, 255}) end))

  -- set_clip_rect clips the following draw commands.
  rec("clip excludes rect", frame(function()
    renderer.set_clip_rect(0, 0, 96, 96)
    renderer.draw_rect(500, 300, 20, 20, {255, 0, 0, 255})
  end))
  rec("clip includes rect", frame(function()
    renderer.set_clip_rect(0, 0, 96, 96)
    renderer.draw_rect(10, 10, 20, 20, {255, 0, 0, 255})
  end))
  rec("clip same, rect moved", frame(function()
    renderer.set_clip_rect(0, 0, 96, 96)
    renderer.draw_rect(200, 10, 20, 20, {255, 0, 0, 255})
  end))
  -- The clip rect is part of the command stream, so changing it alone dirties the cells it covers.
  rec("clip changed", frame(function()
    renderer.set_clip_rect(300, 200, 96, 96)
  end))

  -- Text: the command carries the glyph run, so drawing text dirties the cells it covers.
  rec("text", frame(function() renderer.draw_text(font, "hello rencache", 10, 20, {255, 255, 255, 255}) end))
  rec("text again", frame(function() renderer.draw_text(font, "hello rencache", 10, 20, {255, 255, 255, 255}) end))
  rec("text changed", frame(function() renderer.draw_text(font, "hello renCache", 10, 20, {255, 255, 255, 255}) end))
  rec("text moved down", frame(function() renderer.draw_text(font, "hello renCache", 10, 120, {255, 255, 255, 255}) end))
  rec("text empty", frame(function() renderer.draw_text(font, "", 10, 120, {255, 255, 255, 255}) end))

  -- A command that is entirely outside the screen still hashes into its cells, but the rect is
  -- intersected with the screen before the cells are marked, so nothing becomes dirty.
  rec("offscreen rect", frame(function() renderer.draw_rect(5000, 5000, 20, 20, {255, 0, 0, 255}) end))

  -- Dirty rects are clamped to the screen.
  rec("rect past right edge", frame(function() renderer.draw_rect(790, 10, 100, 10, {255, 0, 0, 255}) end))
  rec("rect past bottom edge", frame(function() renderer.draw_rect(10, 470, 10, 100, {255, 0, 0, 255}) end))

  -- Resizing invalidates every cell, so the next frame repaints the whole screen.
  renderer.begin_frame(win)
  renderer.end_frame()
  rec("before resize", lxl_test.dirty_rects())
end
