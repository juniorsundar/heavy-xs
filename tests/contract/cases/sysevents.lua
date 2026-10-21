-- W5 contract suite, part 2: system.poll_event / system.wait_event.
--
-- Needs the `test_hooks` cargo feature (`lxl_test.push_event`); verify.sh turns it on.
-- The golden was recorded from the C poll_event/wait_event with the same injected events; that
-- implementation is gone, so it is historical evidence, not something this binary can select.

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

-- Reports the exact tuple of one poll_event call, including its arity. SDL can make a
-- pushed event visible one pump late, identically for both implementations, so an empty
-- result is retried briefly before it is believed.
local function poll()
  local r = try(system.poll_event)
  for _ = 1, 4 do
    if r ~= "0" then break end
    system.sleep(0.02)
    r = try(system.poll_event)
  end
  return r
end

-- Drains the queue, returning each tuple (arity-prefixed), so stray events show up.
local function drain()
  local all = {}
  for _ = 1, 100 do
    local p = try(system.poll_event)
    if p == "0" then break end
    all[#all + 1] = p
  end
  return table.concat(all, " | ")
end

return function(rec)
  assert(lxl_test, "this suite needs the test_hooks cargo feature")
  local win = renwindow.create("contract", 400, 300)
  system.sleep(0.1)
  drain() -- window creation events are platform noise
  local push = lxl_test.push_event

  rec("window pixel size", table.concat({ win:get_size() }, "x"))
  rec("poll empty", poll())
  rec("poll empty arity", tostring(select("#", system.poll_event())))

  -- Simple one-value events -------------------------------------------------
  rec("quit", (push { type = "quit" }) and poll() or "push failed")
  for _, name in ipairs { "MINIMIZED", "MAXIMIZED", "RESTORED", "MOUSE_LEAVE", "FOCUS_LOST" } do
    push { type = "window", etype = lxl_test["EVENT_WINDOW_" .. name], window = win }
    rec("window " .. name, poll())
  end
  push { type = "window", etype = lxl_test.EVENT_WINDOW_EXPOSED, window = win }
  rec("window EXPOSED", poll())
  for _, name in ipairs { "WILL_ENTER_BACKGROUND", "DID_ENTER_BACKGROUND", "WILL_ENTER_FOREGROUND", "DID_ENTER_FOREGROUND" } do
    push { type = "app", etype = lxl_test["EVENT_" .. name] }
    rec("app " .. name, poll())
  end

  -- Resize carries the size in points ----------------------------------------
  push { type = "window", etype = lxl_test.EVENT_WINDOW_RESIZED, window = win, data1 = 123, data2 = 77 }
  rec("resized", poll())
  rec("pixel size after resized", table.concat({ win:get_size() }, "x"))
  push { type = "window", etype = lxl_test.EVENT_WINDOW_PIXEL_SIZE_CHANGED, window = win }
  rec("pixel size changed (swallowed)", poll())
  push { type = "window", etype = lxl_test.EVENT_WINDOW_DISPLAY_SCALE_CHANGED, window = win }
  rec("display scale changed (swallowed)", poll())

  -- Focus gained flushes queued key presses and yields nothing ---------------
  push { type = "key_down", scancode = 43, key = 9 }
  push { type = "key_down", scancode = 43, key = 9 }
  push { type = "window", etype = lxl_test.EVENT_WINDOW_FOCUS_GAINED, window = win }
  rec("focus gained flushes key_down", drain())
  push { type = "window", etype = lxl_test.EVENT_WINDOW_FOCUS_GAINED, window = win }
  push { type = "window", etype = lxl_test.EVENT_WINDOW_FOCUS_LOST, window = win }
  rec("focus gained skipped to next", poll())

  -- Keyboard: names are layout aware and lower case ---------------------------
  local keys = {
    { "a", 4, 97 }, { "A (shifted letter code)", 4, 65 }, { "z", 29, 122 }, { "0", 39, 48 },
    { "space", 44, 32 }, { "return", 40, 0x0D }, { "escape", 41, 0x1B }, { "tab", 43, 9 },
    { "backspace", 42, 8 }, { "delete", 76, 0x7F }, { "F1", 58, 0x4000003A }, { "left", 80, 0x40000050 },
    { "right", 79, 0x4000004F }, { "lctrl", 224, 0x400000E0 }, { "lshift", 225, 0x400000E1 },
    { "lalt", 226, 0x400000E2 }, { "lgui", 227, 0x400000E3 }, { "home", 74, 0x4000004A },
    { "pagedown", 78, 0x4000004E }, { "capslock", 57, 0x40000039 }, { "printscreen", 70, 0x40000046 },
    -- non-Latin keycode: falls back to the QWERTY scancode name
    { "cyrillic", 4, 0x0444 }, { "greek", 5, 0x03B2 }, { "cjk", 6, 0x65E5 },
    { "punct [", 47, 91 }, { "punct `", 53, 96 }, { "punct ~", 53, 126 },
    -- scancode names longer than the C's char buf[16]
    { "kp memstore", 208, 0x400000D0 }, { "kp plusminus", 215, 0x400000D7 },
    { "unknown scancode", 1000, 1000 }, { "zero key", 4, 0 },
  }
  for _, k in ipairs(keys) do
    push { type = "key_down", scancode = k[2], key = k[3] }
    rec("key_down " .. k[1], poll())
  end
  push { type = "key_up", scancode = 4, key = 97 }
  rec("key_up a", poll())

  -- Keypad with and without NumLock ------------------------------------------
  local NUMLOCK = 0x1000
  for i = 0, 10 do
    local sc = 89 + i -- KP_1 .. KP_0, KP_PERIOD
    push { type = "key_down", scancode = sc, key = 0x40000059 + i, mod = 0 }
    local off = poll()
    push { type = "key_down", scancode = sc, key = 0x40000059 + i, mod = NUMLOCK }
    rec(("keypad %02d"):format(i), off .. " || " .. poll())
  end
  push { type = "key_down", scancode = 88, key = 0x40000058, mod = 0 }
  rec("keypad enter (outside range)", poll())

  -- Text ----------------------------------------------------------------------
  push { type = "text_input", text = "héllo" }
  rec("textinput", poll())
  push { type = "text_input", text = "" }
  rec("textinput empty", poll())
  push { type = "text_editing", text = "ab", start = 1, length = 2 }
  rec("textediting", poll())
  push { type = "text_editing", text = "", start = 0, length = 0 }
  rec("textediting empty", poll())

  -- Mouse: coordinates are scaled to pixels ----------------------------------
  for _, b in ipairs { 1, 2, 3, 4, 5, 6, 0 } do
    push { type = "mouse_down", window = win, button = b, clicks = b % 3 + 1, x = 10.7, y = 20.2 }
    rec("mouse_down button " .. b, poll())
    drain()
  end
  push { type = "mouse_up", window = win, button = 1, x = 5.5, y = 6.5 }
  rec("mouse_up", poll())
  push { type = "mouse_up", window = win, button = 3, x = 5.5, y = 6.5 }
  rec("mouse_up right", poll())
  push { type = "mouse_motion", window = win, x = 100.9, y = 50.1, xrel = 3.9, yrel = -2.5 }
  rec("mouse_motion", poll())
  -- Queued motion is coalesced: last position, summed deltas.
  for i = 1, 5 do push { type = "mouse_motion", window = win, x = i * 10, y = i * 5, xrel = i, yrel = -i } end
  rec("mouse_motion coalesced", poll())
  rec("mouse_motion coalesced leftover", poll())
  push { type = "mouse_wheel", window = win, x = 1.5, y = -2.5 }
  rec("mouse_wheel", poll())
  push { type = "mouse_wheel", window = win, x = 0, y = 0 }
  rec("mouse_wheel zero", poll())

  -- Touch: coordinates are fractions of the window in points -----------------
  local w, h = system.get_window_size(win)
  rec("window size (points)", tostring(w) .. "x" .. tostring(h))
  push { type = "finger_down", window = win, finger = 7, x = 0.5, y = 0.25 }
  rec("touchpressed", poll())
  push { type = "finger_up", window = win, finger = 7, x = 0.75, y = 1.0 }
  rec("touchreleased", poll())
  push { type = "finger_motion", window = win, finger = 7, x = 0.1, y = 0.2, dx = 0.05, dy = -0.05 }
  rec("touchmoved", poll())
  for i = 1, 4 do push { type = "finger_motion", window = win, finger = 9, x = i / 10, y = i / 10, dx = 0.01, dy = 0.02 } end
  rec("touchmoved coalesced", poll())

  -- File drop ------------------------------------------------------------------
  push { type = "drop_file", window = win, text = "/tmp/some file.txt" }
  rec("filedropped", poll())

  -- Custom events: the registered callback decides what Lua sees. The dirmonitor
  -- event has no callback, so it is swallowed and polling moves on.
  local dm_ok = pcall(function() dirmonitor.new() end)
  rec("dirmonitor module present", tostring(dm_ok))
  -- "dirmonitor" is registered with a NULL callback: its event is swallowed.
  push { type = "other", etype = lxl_test.EVENT_USER + 1 }
  rec("registered, no callback", poll())

  -- Unknown and out-of-range event types fall through the default arm.
  push { type = "other", etype = 0x7FFF }
  rec("unknown low type", poll())
  -- 0x8000 (EVENT_USER) is the first registered custom type, "dialogfinished", whose
  -- callback reads data2 as a string: forging it with NULL crashes the C. Use types
  -- past the registered ones (0x8000, 0x8001) instead.
  push { type = "other", etype = lxl_test.EVENT_USER + 0x100 }
  rec("user range, nothing registered", poll())
  push { type = "other", etype = 0xF000 }
  rec("high type, nothing registered", poll())

  -- Several queued events come out in order, one tuple per call ---------------
  push { type = "text_input", text = "1" }
  push { type = "key_down", scancode = 4, key = 97 }
  push { type = "quit" }
  push { type = "text_input", text = "2" }
  rec("ordered queue", drain())

  -- Events for a window that no longer exists --------------------------------
  -- (a window id of 0 is never registered); the C dereferences NULL, so only the
  -- events that have a NULL-safe path are recorded.
  push { type = "drop_file", text = "/x" }
  rec("filedropped, no window", poll())
  -- WINDOW_RESIZED for an unknown window: the C called ren_resize_window(NULL) and segfaulted
  -- (see docs/rust-port/PLAN.md, W5), so its golden holds a fixed "true" here. The port skips
  -- the event; recorded as a comparison, not a value.
  push { type = "window", etype = lxl_test.EVENT_WINDOW_RESIZED, data1 = 5, data2 = 6 }
  rec("resized, no window survives", tostring(try(system.poll_event) == "0"))

  -- wait_event -----------------------------------------------------------------
  drain()
  local t0 = system.get_time()
  rec("wait_event timeout", try(system.wait_event, 0.05))
  local waited = system.get_time() - t0
  rec("wait_event timeout elapsed", tostring(waited >= 0.04 and waited < 1))
  rec("wait_event zero", try(system.wait_event, 0))
  rec("wait_event negative", try(system.wait_event, -3))
  push { type = "text_input", text = "w" }
  rec("wait_event with event", try(system.wait_event, 1))
  rec("wait_event leaves event queued", poll())
  push { type = "text_input", text = "w2" }
  rec("wait_event no timeout", try(system.wait_event))
  drain()
  rec("wait_event string number", try(system.wait_event, "0.01"))
  rec("wait_event bad arg", try(system.wait_event, "x"))
  rec("wait_event table arg", try(system.wait_event, {}))
  rec("wait_event nil arg", try(system.wait_event, nil))
  rec("wait_event extra args", try(system.wait_event, 0, 1, 2))

  drain()
end
