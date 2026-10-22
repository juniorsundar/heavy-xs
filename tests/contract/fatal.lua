-- Real reporter, fixed nonprivate fixtures, private daemon; no hooks or native display required.
local role = assert(os.getenv("FATAL_CHECK_ROLE"))
local work = assert(os.getenv("DIALOG_CHECK_WORK"))
local dir = work .. "/fatal-" .. role
if role == "bootstrap" then error("deliberate fatal notification startup failure") end
local title, body = "Fatal <&>字", [[a <b href="x&y">字</b> &amp; 'q']]
local function read(path) local f = assert(io.open(path, "rb")); local s = f:read("a"); f:close(); return s end
local M = {}
function M.init()
  local before = read(dir .. "/stderr")
  local bad = {
    {{}, "bad argument #1 to 'system.show_fatal_error' (string expected, got no value)"},
    {{"title"}, "bad argument #2 to 'system.show_fatal_error' (string expected, got no value)"},
    {{{}, "message"}, "bad argument #1 to 'system.show_fatal_error' (string expected, got table)"},
    {{"title", {}}, "bad argument #2 to 'system.show_fatal_error' (string expected, got table)"},
    {{true, "message"}, "bad argument #1 to 'system.show_fatal_error' (string expected, got boolean)"},
  }
  for _, case in ipairs(bad) do
    local ok, err = pcall(system.show_fatal_error, table.unpack(case[1]))
    assert(not ok and err == case[2], tostring(err))
  end
  assert(read(dir .. "/stderr") == before, "argument errors wrote diagnostics")
  if role == "failed-init" then
    local errors = {}
    for i = 1, 2 do
      local ok, err = pcall(renwindow.create, "unavailable", 120, 80)
      assert(not ok and err:match("Error creating lite%-xl window"), tostring(err))
      errors[i] = err
    end
    assert(errors[1] == errors[2], "failed native initialization was retried")
  elseif role == "independent" then
    local w = renwindow.create("fatal-independent-client", 120, 80)
    system.open_file_dialog(w, 51, {title = "independence"})
    local deadline, done = system.get_time() + 5, false
    while system.get_time() < deadline and not done do
      for event, id, status in system.poll_event do
        if event == "dialogfinished" and id == 51 then assert(status == "accept"); done = true end
      end
      if not done then system.wait_event(0.01) end
    end
    assert(done, "portal request did not complete")
  elseif role == "numeric" then title, body = 42, -5
  elseif role == "nul" then title, body = "nul-title\0ignored", "nul-body\0ignored"
  elseif role == "after-nul" then title, body = "nul-title\0\255", "nul-body\0\255"
  elseif role == "invalid-title" then title = "\255"
  elseif role == "invalid-message" then body = "\255"
  end
  before = read(dir .. "/stderr") -- a real native creation may log its backend
  local start = system.get_time()
  assert(select("#", system.show_fatal_error(title, body)) == 0, "reporter returned values")
  local elapsed = system.get_time() - start
  assert(elapsed < 3.0, "fatal IPC exceeded the single deadline: " .. elapsed)
  if role == "ack" then assert(elapsed >= 0.2, "reporter did not wait for acknowledgement") end
  if role:match("timeout$") or role == "deadline" then
    assert(elapsed >= 1.7, "timeout fixture did not reach the deadline: " .. elapsed)
  end
  if role ~= "writefail" then
    local expected = tostring(title):match("^[^%z]*") .. ": " .. tostring(body):match("^[^%z]*") .. "\n"
    assert(read(dir .. "/stderr") == before .. expected, "original diagnostic bytes changed")
  end
  io.stdout:write("FATAL ", role, " passed\n")
end
function M.run() end
function M.on_error(err) io.stderr:write(tostring(err), "\n"); os.exit(1) end
return M
