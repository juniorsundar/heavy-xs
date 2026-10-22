-- Ordinary-binary runtime for native-check.sh: no injection hooks or host services.
local M = {}
local mode = assert(os.getenv("NATIVE_CHECK_MODE"))
local work = assert(os.getenv("NATIVE_CHECK_WORK"))
local function mark(name)
  local f = assert(io.open(work .. "/" .. name, "w")); f:write("ready\n"); f:close()
end
local function drain() while system.poll_event() do end end

function M.init()
  if mode == "input" then
    local window = renwindow.create("native-input-check", 320, 160)
    renderer.begin_frame(window)
    renderer.draw_rect(0, 0, 320, 160, {32, 32, 32})
    renderer.end_frame()
    system.text_input(window, true)
    mark("ready")
    local deadline = system.get_time() + 8
    while not system.window_has_focus(window) do
      assert(system.get_time() < deadline, "focus timeout")
      system.wait_event(0.1); drain()
    end
    drain() -- focus gain flushes keys by design; ask the driver for keys only afterwards
    mark("focused")
    local keys, text = {}, {}
    while system.get_time() < deadline do
      system.wait_event(0.1)
      while true do
        local kind, value = system.poll_event()
        if not kind then break end
        if kind == "textinput" then text[#text + 1] = value end
        if kind == "keypressed" then
          if value == "f12" then
            assert(table.concat(keys, ",") == "a,left shift,a,right ctrl,s,end,keypad 1,keypad enter",
              "native key sequence: " .. table.concat(keys, ","))
            assert(table.concat(text, "|") == "a|A|1", "native text sequence: " .. table.concat(text, "|"))
            print("NATIVE input passed"); return
          end
          keys[#keys + 1] = value
        end
      end
    end
    error("native input timeout")
  end

  if RESTARTED then
    assert(system.wait_event(0), "pending signal lost across restart")
    assert(system.poll_event() == "quit", "signal did not become quit")
    while true do
      local event = system.poll_event()
      if not event then break end
      assert(event ~= "quit", "duplicate process quit")
    end
    assert(not system.wait_event(0), "quit was consumed but remained ready")
    print("NATIVE " .. mode .. " signal/restart passed")
    return
  end
  if mode == "window" then
    renwindow.create("native-wait-check", 200, 100) -- hidden, to avoid focus/expose noise
    system.sleep(0.1); drain()
  elseif mode == "failed" then
    local ok, first = pcall(renwindow.create, "unavailable", 200, 100)
    assert(not ok, "missing-display creation unexpectedly succeeded")
    local again, second = pcall(renwindow.create, "unavailable", 200, 100)
    assert(not again and first == second, "native creation failure was not retained")
  else
    assert(mode == "preloop")
  end
  mark("ready")
  assert(system.wait_event(), "signal failed to wake indefinite wait")
  assert(system.wait_event(0), "readiness consumed signal")
  M.restart_request = true -- deliberately do not consume quit before retiring Lua
end
function M.run() end
function M.on_error(err) io.stderr:write(tostring(err), "\n"); os.exit(1) end
return M
