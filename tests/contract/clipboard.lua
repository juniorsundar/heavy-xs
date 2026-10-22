-- Native clipboard runtime for scripts/rust-port/clipboard-check.sh.
-- CLIPBOARD_MODE selects the process role; owner and peer run in separate editor processes on the
-- same runner-owned Xvfb, so every read is a real OS selection transfer, not a Rust unit-test mock.
local M = {}

-- Written by the owner before its Lua restart and read by the peer afterwards.
local FIXED_CLIPBOARD = "owner clipboard → ok\nsecond line"
local FIXED_PRIMARY = "owner primary λ ok"

local function show(value)
  return type(value) == "string" and string.format("%q", value) or tostring(value)
end

local function fail(message)
  io.stderr:write("CLIPBOARD FAIL ", message, "\n")
  os.exit(1)
end

local function ok(condition, label)
  if not condition then fail(label) end
end

local function eq(got, want, label)
  if got ~= want then fail(label .. ": expected " .. show(want) .. " got " .. show(got)) end
end

local function write_file(path, text)
  local file = assert(io.open(path, "wb"))
  file:write(text)
  file:close()
end

-- The owner must stay alive while the peer reads: on X11 the selection lives in the owner process.
local function wait_for(path, tenths)
  for _ = 1, tenths do
    local file = io.open(path, "rb")
    if file then
      file:close()
      return true
    end
    system.sleep(0.1)
  end
  return false
end

local function unavailable()
  ok(os.getenv("DISPLAY") == nil, "DISPLAY must be unset")
  ok(os.getenv("WAYLAND_DISPLAY") == nil, "WAYLAND_DISPLAY must be unset")
  ok(os.getenv("WAYLAND_SOCKET") == nil, "WAYLAND_SOCKET must be unset")

  -- Always one value: an unavailable clipboard reads as one empty string, never zero values.
  local clipboard = table.pack(system.get_clipboard())
  ok(clipboard.n == 1 and clipboard[1] == "", "get_clipboard() must return one empty string")
  local primary = table.pack(system.get_primary_selection())
  ok(primary.n == 1 and primary[1] == "", "get_primary_selection() must return one empty string")

  -- No display: writes are silent no-ops, including bytes that would be invalid UTF-8.
  ok(pcall(system.set_clipboard, "ignored"), "set_clipboard is silent without a display")
  ok(pcall(system.set_clipboard, "\xff"), "invalid UTF-8 set is silent without a display")
  ok(pcall(system.set_primary_selection, "ignored"), "set_primary_selection is silent without a display")
  ok(pcall(system.set_primary_selection, "\xff"), "invalid primary set is silent without a display")

  -- The Lua boundary still checks and coerces arguments before reaching the clipboard.
  local success, err = pcall(system.set_clipboard)
  ok(not success, "set_clipboard() raises")
  ok(tostring(err):find("bad argument", 1, true) ~= nil, "set_clipboard() error: " .. tostring(err))
  success, err = pcall(system.set_clipboard, nil)
  ok(not success, "set_clipboard(nil) raises")
  success, err = pcall(system.set_primary_selection)
  ok(not success, "set_primary_selection() raises")
  ok(pcall(system.set_clipboard, 42), "set_clipboard(42) coerces")
  ok(pcall(system.set_primary_selection, 43), "set_primary_selection(43) coerces")
  eq(system.get_clipboard(), "", "unavailable clipboard stays empty after writes")
  eq(system.get_primary_selection(), "", "unavailable primary stays empty after writes")

  io.stderr:write("CLIPBOARD unavailable checks passed\n")
end

local function roundtrips()
  local set_clipboard, set_primary = system.set_clipboard, system.set_primary_selection
  local get_clipboard, get_primary = system.get_clipboard, system.get_primary_selection

  local unicode = "héllo→ 🌍"
  set_clipboard(unicode)
  eq(get_clipboard(), unicode, "clipboard Unicode")
  set_primary(unicode)
  eq(get_primary(), unicode, "primary Unicode")

  local multiline = "line1\nline2\r\nline4"
  set_clipboard(multiline)
  eq(get_clipboard(), multiline, "clipboard newline")
  set_primary(multiline)
  eq(get_primary(), multiline, "primary newline")

  set_clipboard("")
  eq(get_clipboard(), "", "clipboard empty")
  eq(get_primary(), multiline, "clearing clipboard leaves primary unchanged")
  set_clipboard("preserved")
  set_primary("")
  eq(get_primary(), "", "primary empty")
  eq(get_clipboard(), "preserved", "clearing primary leaves clipboard unchanged")

  -- NUL truncation happens before UTF-8 validation, so the invalid suffix never matters.
  set_clipboard("ab\0\xff")
  eq(get_clipboard(), "ab", "clipboard NUL before invalid UTF-8")
  set_primary("cd\0\xfe")
  eq(get_primary(), "cd", "primary NUL before invalid UTF-8")

  -- Invalid UTF-8 is rejected whole: the previous selection stays.
  set_clipboard("keep-clipboard")
  set_clipboard("bad\xfftext")
  eq(get_clipboard(), "keep-clipboard", "clipboard invalid UTF-8 leaves it unchanged")
  set_primary("keep-primary")
  set_primary("bad\xfftext")
  eq(get_primary(), "keep-primary", "primary invalid UTF-8 leaves it unchanged")

  -- The two selections are independent.
  set_clipboard("clip only")
  set_primary("primary only")
  eq(get_clipboard(), "clip only", "clipboard independent")
  eq(get_primary(), "primary only", "primary independent")

  -- No argument is an error; a number is coerced like any other Lua string argument.
  local success, err = pcall(set_clipboard)
  ok(not success, "set_clipboard() raises with a display")
  ok(tostring(err):find("bad argument", 1, true) ~= nil, "set_clipboard() error: " .. tostring(err))
  success, err = pcall(set_clipboard, nil)
  ok(not success, "set_clipboard(nil) raises with a display")
  success, err = pcall(set_primary)
  ok(not success, "set_primary_selection() raises with a display")
  ok(pcall(set_clipboard, 42), "set_clipboard(42) coerces with a display")
  eq(get_clipboard(), "42", "clipboard numeric coercion")
  ok(pcall(set_primary, 43), "set_primary_selection(43) coerces with a display")
  eq(get_primary(), "43", "primary numeric coercion")
end

local function owner()
  local work = assert(os.getenv("CLIPBOARD_CHECK_WORK"), "CLIPBOARD_CHECK_WORK is required")
  if not RESTARTED then
    eq(system.get_clipboard(), "", "fresh server has no clipboard owner")
    eq(system.get_primary_selection(), "", "fresh server has no primary owner")
    roundtrips()
    system.set_clipboard(FIXED_CLIPBOARD)
    system.set_primary_selection(FIXED_PRIMARY)
    eq(system.get_clipboard(), FIXED_CLIPBOARD, "saved clipboard before restart")
    eq(system.get_primary_selection(), FIXED_PRIMARY, "saved primary before restart")
    M.restart_request = true
    io.stderr:write("CLIPBOARD owner checks passed; restarting\n")
    return
  end

  -- The Lua state is new, but the process (and its X11 selection owner) is the same.
  eq(system.get_clipboard(), FIXED_CLIPBOARD, "clipboard survives Lua restart")
  eq(system.get_primary_selection(), FIXED_PRIMARY, "primary survives Lua restart")
  write_file(work .. "/ready", "ready\n")
  local tenths = tonumber(os.getenv("CLIPBOARD_WAIT_TENTHS") or "100")
  ok(wait_for(work .. "/done", tenths), "peer did not write done")
  io.stderr:write("CLIPBOARD owner and peer completed\n")
end

local function peer()
  local work = assert(os.getenv("CLIPBOARD_CHECK_WORK"), "CLIPBOARD_CHECK_WORK is required")
  eq(system.get_clipboard(), FIXED_CLIPBOARD, "peer clipboard")
  eq(system.get_primary_selection(), FIXED_PRIMARY, "peer primary")
  write_file(work .. "/done", "done\n")
  io.stderr:write("CLIPBOARD peer read both selections\n")
end

function M.init()
  local mode = os.getenv("CLIPBOARD_MODE") or ""
  if mode == "unavailable" then
    unavailable()
  elseif mode == "owner" then
    owner()
  elseif mode == "peer" then
    peer()
  else
    fail("unknown CLIPBOARD_MODE " .. show(mode))
  end
end

function M.run() end

function M.on_error(err)
  io.stderr:write("CLIPBOARD ERROR ", tostring(err), "\n")
  os.exit(1)
end

return M
