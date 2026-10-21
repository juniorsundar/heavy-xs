-- W5 contract suite, part 3: window, cursor, text-input and message-box functions of
-- `system`. Needs a window: it runs under the offscreen driver or a private Xvfb.
--
-- What a window reports (focus, positions after a move, whether maximize is honoured) is the
-- window manager's business, not ours. Where that varies, the suite records an invariant that
-- holds with or without one instead of a value.

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  if not r[1] then return "ERR" end
  local out = { tostring(r.n - 1) }
  for i = 2, r.n do
    local v = r[i]
    out[#out + 1] = type(v) == "number" and (math.type(v) .. ":" .. tostring(v)) or tostring(v)
  end
  return table.concat(out, ",")
end

local function pump()
  system.sleep(0.12)
  for _ = 1, 200 do if not system.poll_event() then break end end
end

-- X11 applies a resize on a later pump (offscreen at once): pump, bounded, until the reported
-- size stops changing.
local function settle(win)
  local last
  for _ = 1, 25 do
    pump()
    local now = table.concat({ system.get_window_size(win) }, ",")
    if now == last then return end
    last = now
  end
end

-- The size is ours to honour; where the window lands is not (a bare X server leaves it centred).
local function size_of(win)
  local w, h, x, y = system.get_window_size(win)
  return table.concat({ w, h, math.type(x), math.type(y) }, ",")
end

return function(rec)
  local win = renwindow.create("contract", 400, 300)
  pump()

  -- Arity and types of each query on a fresh window ---------------------------------------
  rec("get_window_size arity", try(system.get_window_size, win):match("^%d+"))
  rec("get_window_size types", (try(system.get_window_size, win):gsub("%-?%d+", "N")))
  local w, h = system.get_window_size(win)
  rec("get_window_size w,h", w .. "x" .. h)
  rec("get_window_mode", try(system.get_window_mode, win))
  rec("window_has_focus type", type(system.window_has_focus(win)))
  rec("window_has_focus arity", tostring(select("#", system.window_has_focus(win))))

  -- Mode round trip: every call is followed by a real query, so a wrong flag mapping shows up.
  -- Maximize and minimize need a window manager (offscreen ignores them, a bare X server keeps
  -- them), so either answer is valid for those; anything else is a wrong mapping.
  local wm_modes = { maximized = true, minimized = true }
  for _, m in ipairs { "normal", "maximized", "minimized", "fullscreen", "normal", "fullscreen", "maximized", "normal" } do
    local r = try(system.set_window_mode, win, m)
    pump()
    local mode = tostring(system.get_window_mode(win))
    if wm_modes[m] and (mode == m or mode == "normal") then mode = m .. "|normal" end
    rec("set_window_mode " .. m, r .. " -> " .. mode)
  end
  rec("set_window_mode default", try(system.set_window_mode, win))
  pump()
  rec("set_window_mode default -> ", tostring(system.get_window_mode(win)))
  rec("set_window_mode bad option", try(system.set_window_mode, win, "sideways"))
  rec("set_window_mode number", try(system.set_window_mode, win, 3))
  rec("set_window_mode no window", try(system.set_window_mode, "x", "normal"))
  rec("set_window_mode nil", try(system.set_window_mode, nil, "normal"))
  rec("set_window_mode table", try(system.set_window_mode, {}, "normal"))
  pump()
  system.set_window_mode(win, "normal")
  pump()

  -- Size and position ----------------------------------------------------------------------
  rec("set_window_size", try(system.set_window_size, win, 250, 180, 40, 50))
  settle(win)
  rec("size after set", size_of(win))
  rec("pixel size after set", table.concat({ win:get_size() }, "x"))
  rec("set_window_size fractions", try(system.set_window_size, win, 301.9, 222.2, 10.5, 11.5))
  settle(win)
  rec("size after fractions", size_of(win))
  rec("set_window_size string numbers", try(system.set_window_size, win, "260", "190", "20", "30"))
  settle(win)
  rec("size after string numbers", size_of(win))
  rec("set_window_size 3 args", try(system.set_window_size, win, 100, 100, 1))
  rec("set_window_size nil arg", try(system.set_window_size, win, 100, nil, 1, 1))
  rec("set_window_size bad window", try(system.set_window_size, {}, 1, 1, 1, 1))
  rec("set_window_size negative", try(system.set_window_size, win, -5, -5, -5, -5))
  settle(win)
  rec("size after negative", size_of(win))
  system.set_window_size(win, 400, 300, 50, 50)
  pump()

  -- Title, bordered, raise, opacity ----------------------------------------------------------
  rec("set_window_title", try(system.set_window_title, win, "hello"))
  rec("set_window_title unicode", try(system.set_window_title, win, "日本語 ✓"))
  rec("set_window_title empty", try(system.set_window_title, win, ""))
  rec("set_window_title number", try(system.set_window_title, win, 42))
  rec("set_window_title nil", try(system.set_window_title, win, nil))
  rec("set_window_title no args", try(system.set_window_title))
  rec("set_window_bordered true", try(system.set_window_bordered, win, true))
  rec("set_window_bordered false", try(system.set_window_bordered, win, false))
  rec("set_window_bordered nil", try(system.set_window_bordered, win, nil))
  rec("set_window_bordered 0", try(system.set_window_bordered, win, 0))
  rec("set_window_bordered none", try(system.set_window_bordered, win))
  system.set_window_bordered(win, true)
  rec("raise_window", try(system.raise_window, win))
  rec("raise_window bad", try(system.raise_window, 1))
  rec("set_window_opacity 0.5", try(system.set_window_opacity, win, 0.5))
  rec("set_window_opacity 1", try(system.set_window_opacity, win, 1))
  rec("set_window_opacity 0", try(system.set_window_opacity, win, 0))
  rec("set_window_opacity 7 (out of range)", try(system.set_window_opacity, win, 7))
  rec("set_window_opacity string", try(system.set_window_opacity, win, "0.5"))
  rec("set_window_opacity none", try(system.set_window_opacity, win))
  system.set_window_opacity(win, 1)

  -- Hit testing: the grid itself is compared by scripts/rust-port/hit-test-check.sh ---------
  rec("set_window_hit_test set", try(system.set_window_hit_test, win, 30, 90, 6))
  rec("set_window_hit_test zeros", try(system.set_window_hit_test, win, 0, 0, 0))
  rec("set_window_hit_test floats", try(system.set_window_hit_test, win, 30.9, 90.1, 6.5))
  rec("set_window_hit_test clear", try(system.set_window_hit_test, win))
  rec("set_window_hit_test 2 args", try(system.set_window_hit_test, win, 30))
  rec("set_window_hit_test 3 args", try(system.set_window_hit_test, win, 30, 90))
  rec("set_window_hit_test strings", try(system.set_window_hit_test, win, "30", "90", "6"))
  rec("set_window_hit_test bad", try(system.set_window_hit_test, win, "a", "b", "c"))
  system.set_window_hit_test(win)

  -- Text input -------------------------------------------------------------------------------
  rec("text_input on", try(system.text_input, win, true))
  rec("text_input off", try(system.text_input, win, false))
  rec("text_input nil", try(system.text_input, win, nil))
  rec("text_input none", try(system.text_input, win))
  rec("text_input bad window", try(system.text_input, {}, true))
  rec("set_text_input_rect", try(system.set_text_input_rect, win, 10, 20, 30, 40))
  rec("set_text_input_rect floats", try(system.set_text_input_rect, win, 10.9, 20.9, 30.9, 40.9))
  rec("set_text_input_rect 4 args", try(system.set_text_input_rect, win, 1, 2, 3))
  rec("set_text_input_rect bad", try(system.set_text_input_rect, win, "a", 2, 3, 4))
  rec("clear_ime", try(system.clear_ime, win))
  rec("clear_ime bad", try(system.clear_ime, nil))

  -- Cursor: arity and the option check; the cache is never freed so repeated sets are safe ---
  for _, c in ipairs { "arrow", "ibeam", "sizeh", "sizev", "hand", "arrow", "hand" } do
    rec("set_cursor " .. c, try(system.set_cursor, c))
  end
  rec("set_cursor default", try(system.set_cursor))
  rec("set_cursor nil", try(system.set_cursor, nil))
  rec("set_cursor bad", try(system.set_cursor, "pointer"))
  rec("set_cursor number", try(system.set_cursor, 2))
  rec("set_cursor table", try(system.set_cursor, {}))
  for _ = 1, 200 do system.set_cursor("ibeam"); system.set_cursor("hand") end
  rec("set_cursor 400 times", "ok")

  -- Argument validation of show_fatal_error only: calling it opens a modal box -----------------
  rec("show_fatal_error no args", try(system.show_fatal_error))
  rec("show_fatal_error one arg", try(system.show_fatal_error, "title"))
  rec("show_fatal_error tables", try(system.show_fatal_error, {}, {}))

  -- A window is a userdata: wrong types must be rejected before SDL is touched ------------------
  for _, fn in ipairs { "get_window_size", "get_window_mode", "window_has_focus", "raise_window", "clear_ime" } do
    rec(fn .. " string window", try(system[fn], "window"))
    rec(fn .. " no window", try(system[fn]))
  end
end
