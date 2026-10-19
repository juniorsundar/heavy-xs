-- PROTOTYPE SDL cost-baseline probe. Appended to a throwaway copy of a user config so the
-- configured fonts, theme and plugins stay active. Runs one workload, writes TSV, exits.
-- Env: COST_MODE = startup | frames | idle | memory, COST_OUT (TSV path), COST_FILE (document).
-- frames: FRAME_WARM (default 120), FRAME_N (default 600). idle: IDLE_S (default 60).
-- memory: MEM_CYCLES (default 50).
-- Declare the global the Rust presentation hook sets, before the first frame; core.strict
-- otherwise aborts on the first renderer.end_frame.
rawset(_G, "LXL_LAST_PRESENT_S", 0)
local core = require "core"
local command = require "core.command"
local config = require "core.config"
local DocView = require "core.docview"

local mode = assert(os.getenv("COST_MODE"), "COST_MODE required")
local out = assert(os.getenv("COST_OUT"), "COST_OUT required")
local file = os.getenv("COST_FILE")

local function proc_status()
  local s = {}
  for line in io.lines("/proc/self/status") do
    local k, v = line:match("^([%w_]+):%s+(%d+)")
    if k then s[k] = tonumber(v) end
  end
  return s
end

-- utime+stime in clock ticks (USER_HZ = 100 on Linux) from /proc/self/stat, after the comm field.
local function cpu_ticks()
  local f = assert(io.open("/proc/self/stat"))
  local rest = f:read("a"):match("%) (.*)$")
  f:close()
  local fields = {}
  for x in rest:gmatch("%S+") do fields[#fields + 1] = x end
  return tonumber(fields[12]) + tonumber(fields[13])
end

local function write(header, rows, meta)
  local ww, wh = core.window:get_size()
  local f = assert(io.open(out, "w"))
  f:write(header, "\n", table.concat(rows, "\n"), "\n")
  f:close()
  local m = assert(io.open(out .. ".meta", "w"))
  m:write(("mode=%s window=%dx%d fps=%s %s\n"):format(mode, ww, wh, tostring(config.fps), meta or ""))
  m:close()
end

local function finish()
  for _, doc in ipairs(core.docs) do doc:clean() end
  os.exit(0)
end

-- Per drawn frame: active work (core.step minus presentation) and presentation.
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

-- Frames are paced by core.run (config.fps); each yield lets one frame happen.
local function frames(count, each)
  for i = 1, count do
    if each then each(i) end
    core.redraw = true
    coroutine.yield()
  end
end

local function wait_seconds(s)
  local t_end = system.get_time() + s
  while system.get_time() < t_end do coroutine.yield(t_end - system.get_time()) end
end

-- Wait for the window to settle at fullscreen: the compositor can resize it in several steps,
-- and a run that measures a half-resized window would not compare with the next run.
local function settle_fullscreen()
  system.set_window_mode(core.window, "fullscreen")
  local last, stable, t_end = nil, 0, system.get_time() + 5
  while system.get_time() < t_end and stable < 10 do
    local w, h = core.window:get_size()
    local key = w .. "x" .. h
    stable = (key == last) and stable + 1 or 0
    last = key
    core.redraw = true
    coroutine.yield(0.05)
  end
end

local function open_document()
  core.root_view:open_doc(core.open_doc(assert(file, "COST_FILE required")))
  local view = core.active_view
  assert(view:is(DocView), "document view")
  return view
end

local modes = {}

-- Process start to the end of the first presented frame. The launcher records the start time.
function modes.startup()
  local launched = tonumber((assert(os.getenv("COST_T0_NS"), "COST_T0_NS required"))) / 1e9
  frames(1)
  -- wall clock from the launcher's date +%s%N, so both ends use the same clock
  local p = assert(io.popen("date +%s%N"))
  local now = tonumber(p:read("l")) / 1e9
  p:close()
  local s = proc_status()
  write("startup_ms\trss_kb\thwm_kb", { ("%.3f\t%d\t%d"):format((now - launched) * 1e3, s.VmRSS, s.VmHWM) })
  finish()
end

function modes.frames()
  local warm = tonumber(os.getenv("FRAME_WARM") or "120")
  local n = tonumber(os.getenv("FRAME_N") or "600")
  -- Fullscreen, not maximised: a tiling window manager (here KWin + Krohnkite) turns maximise
  -- into a tile, so the window size would depend on the desktop layout.
  settle_fullscreen()
  frames(30)
  local view = open_document()
  local lh = view:get_line_height()
  local lines = #view.doc.lines
  local function scroll_to(y)
    view.scroll.to.y = y % math.max(lh, (lines - 80) * lh)
    view.scroll.y = view.scroll.to.y
  end
  phase = "warmup"; frames(warm, function(i) scroll_to(i * lh) end)
  phase = "scroll"; frames(n, function(i) scroll_to(i * lh) end)
  phase = "scroll_page"; frames(n, function(i) scroll_to(i * 40 * lh) end)
  view.doc:set_selection(math.min(40, lines), 1)
  scroll_to(0)
  phase = "type"
  frames(n, function(i)
    if i % 60 == 0 then command.perform("doc:newline")
    else view.doc:text_input(string.char(97 + i % 26)) end
  end)
  write("phase\tactive_ms\tpresent_ms\ttotal_ms", rows)
  finish()
end

-- Idle: count loop wake-ups and CPU over IDLE_S with no input. idle_focused raises the window
-- (caret blinks); idle_unfocused minimises it, which drops focus. Focus at start and end is
-- recorded so a run where focus did not take can be rejected.
-- Unfocused, core.run blocks in system.wait_event() with no timeout until an OS event arrives,
-- so an in-process probe thread never resumes. For idle_unfocused the probe only writes
-- COST_OUT.ready after minimising; cost-run.sh then samples /proc/<pid> from outside and ends
-- the process.
local function idle(focused)
  local idle_s = tonumber(os.getenv("IDLE_S") or "20")
  -- Fullscreen, not maximised: a tiling window manager (here KWin + Krohnkite) turns maximise
  -- into a tile, so the window size would depend on the desktop layout.
  settle_fullscreen()
  open_document()
  frames(30)
  if not focused then
    -- No frames(): once minimised, the loop blocks in wait_event() and this thread would stall.
    system.set_window_mode(core.window, "minimized")
    -- /proc/self/stat field 1 is this process's pid. Write then rename, so the driver never
    -- sees an empty or partial file (io.open truncates before the write lands).
    local st = assert(io.open("/proc/self/stat"))
    local pid = st:read("n")
    st:close()
    local f = assert(io.open(out .. ".ready.tmp", "w"))
    -- Focus is read before the compositor has applied the minimise; the driver re-checks state.
    f:write(pid, " ", tostring(system.window_has_focus(core.window)), " ",
      tostring(system.get_window_mode(core.window)), "\n")
    f:close()
    assert(os.rename(out .. ".ready.tmp", out .. ".ready"))
    while true do coroutine.yield(3600) end
  end
  system.raise_window(core.window)
  wait_seconds(2)
  local counts = { wait = 0, wait_timeout = 0, wait_sub_ms = 0, sleep = 0 }
  local wait_event, sleep = system.wait_event, system.sleep
  system.wait_event = function(t, ...)
    counts.wait = counts.wait + 1
    -- A timeout under 1 ms becomes 0 ms in the SDL call, i.e. a non-blocking poll.
    if t and t < 0.001 then counts.wait_sub_ms = counts.wait_sub_ms + 1 end
    local got = wait_event(t, ...)
    if not got then counts.wait_timeout = counts.wait_timeout + 1 end
    return got
  end
  system.sleep = function(...) counts.sleep = counts.sleep + 1; return sleep(...) end
  local before, ticks0, frames0, t0 = proc_status(), cpu_ticks(), #rows, system.get_time()
  local focused0 = system.window_has_focus(core.window)
  wait_seconds(idle_s)
  local after, ticks1, t1 = proc_status(), cpu_ticks(), system.get_time()
  system.wait_event, system.sleep = wait_event, sleep
  local elapsed = t1 - t0
  write("elapsed_s\tcpu_ms\tcpu_pct\tframes\twait_calls\twait_timeouts\twait_sub_ms\tsleep_calls\tvol_ctxsw\tinvol_ctxsw\tfocused_start\tfocused_end", {
    ("%.3f\t%d\t%.3f\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%s"):format(
      elapsed, (ticks1 - ticks0) * 10, (ticks1 - ticks0) * 10 / (elapsed * 10), #rows - frames0,
      counts.wait, counts.wait_timeout, counts.wait_sub_ms, counts.sleep,
      after.voluntary_ctxt_switches - before.voluntary_ctxt_switches,
      after.nonvoluntary_ctxt_switches - before.nonvoluntary_ctxt_switches,
      tostring(focused0), tostring(system.window_has_focus(core.window)))
  })
  finish()
end
function modes.idle_focused() idle(true) end
function modes.idle_unfocused() idle(false) end

-- Memory: open, draw, resize and drop a second window; resize the main window. RSS per cycle.
function modes.memory()
  local cycles = tonumber(os.getenv("MEM_CYCLES") or "50")
  settle_fullscreen()
  open_document()
  frames(30)
  local style = require "core.style"
  local mem = {}
  local function sample(cycle)
    collectgarbage("collect"); collectgarbage("collect")
    local s = proc_status()
    mem[#mem + 1] = ("%d\t%d\t%d\t%.1f"):format(cycle, s.VmRSS, s.VmHWM, collectgarbage("count"))
  end
  sample(0)
  local sizes = {}
  for cycle = 1, cycles do
    local win = renwindow.create("cost-probe-memory", 640, 400)
    for _, size in ipairs({ { 640, 400 }, { 900, 600 }, { 640, 400 } }) do
      system.set_window_size(win, size[1], size[2], 0, 0)
      renderer.begin_frame(win)
      renderer.draw_rect(0, 0, size[1], size[2], style.background)
      renderer.draw_text(style.code_font, ("cycle %d"):format(cycle), 20, 20, style.text)
      renderer.end_frame()
    end
    win = nil
    -- Main window: leave fullscreen and return, which resizes the surface both ways. A tiling
    -- window manager may ignore explicit sizes, so mode changes are the reliable resize.
    system.set_window_mode(core.window, "normal")
    frames(6)
    local nw, nh = core.window:get_size()
    settle_fullscreen()
    local fw, fh = core.window:get_size()
    sizes[nw .. "x" .. nh .. "->" .. fw .. "x" .. fh] = true
    frames(3)
    sample(cycle)
  end
  local seen = {}
  for k in pairs(sizes) do seen[#seen + 1] = k end
  table.sort(seen)
  write("cycle\trss_kb\thwm_kb\tlua_kb", mem, "resizes=" .. table.concat(seen, ","))
  finish()
end

-- core.add_thread catches errors and keeps the editor running; a broken probe must exit instead.
core.add_thread(function()
  local ok, err = xpcall(assert(modes[mode], "unknown COST_MODE " .. mode), debug.traceback)
  if not ok then
    io.stderr:write("cost-probe error: ", tostring(err), "\n")
    os.exit(3)
  end
end)
