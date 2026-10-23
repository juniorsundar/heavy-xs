-- The `dirmonitor` module (src/dirmonitor.rs), including its worker thread, which pushes the
-- "dirmonitor" custom event.
--
-- The golden came from the C build, whose inotify walker also reported garbage watch ids made
-- of file-name bytes (dirwatch ignores unknown ids), so the suite records only ids that were
-- really handed out.

local system = require "system"

local ROOT = "/tmp/lxl_dirmonitor_contract"

local function sh(cmd) assert(os.execute(cmd)) end

local function status(fn, ...)
  local r = table.pack(pcall(fn, ...))
  local out = { tostring(r[1]) }
  if r[1] then
    out[#out + 1] = tostring(r.n - 1) -- number of return values
    for i = 2, r.n do out[#out + 1] = tostring(r[i]) end
  end
  return table.concat(out, "|")
end

local function touch(path, text)
  local f = assert(io.open(path, "wb"))
  f:write(text or "x")
  f:close()
end

-- Polls `check` until it reports a change (true) or the time runs out. Returns the
-- last result and how many times each known watch id was reported.
local function wait_change(m, known, limit)
  local seen = {}
  local result = false
  local t0 = system.get_time()
  repeat
    result = m:check(function(id)
      if known[id] then seen[id] = (seen[id] or 0) + 1 end
    end)
  until result or system.get_time() - t0 > (limit or 3)
  return result, seen
end

-- Drains whatever is pending so the next step starts from silence.
local function settle(m)
  local t0 = system.get_time()
  while system.get_time() - t0 < 0.3 do m:check(function() end) end
end

return function(rec)
  sh("rm -rf " .. ROOT .. " && mkdir -p " .. ROOT .. "/a " .. ROOT .. "/b")

  rec("mode", dirmonitor.mode())
  rec("type(dirmonitor)", type(dirmonitor))
  rec("type(new)", type(dirmonitor.new))

  local m = dirmonitor.new()
  rec("type(monitor)", type(m))
  rec("tostring prefix", (tostring(m):match("^[^:]+")))
  rec("monitor:mode()", m:mode())
  rec("check idle (no watches)", tostring(m:check(function() end)))
  rec("check returns 1 value", status(m.check, m, function() end))

  -- Watch ids. inotify_add_watch numbers from 1, and they reach Lua as floats.
  rec("watch missing path", tostring(m:watch(ROOT .. "/nope")))
  local wa = m:watch(ROOT .. "/a")
  local wb = m:watch(ROOT .. "/b")
  rec("watch a", tostring(wa))
  rec("watch b", tostring(wb))
  rec("watch a again", tostring(m:watch(ROOT .. "/a")))
  rec("math.type(watch)", tostring(math.type(wa)))
  rec("unwatch returns", status(m.unwatch, m, 9999))
  rec("unwatch nil", status(m.unwatch, m, nil))
  rec("unwatch string", status(m.unwatch, m, "x"))
  rec("watch no path", status(m.watch, m))
  rec("watch number path", status(m.watch, m, 12))

  local known = { [wa] = true, [wb] = true }
  settle(m)
  rec("check quiet", tostring(m:check(function() end)))

  -- A change in one directory is reported for that directory's id only.
  touch(ROOT .. "/a/file.txt")
  local r, seen = wait_change(m, known)
  rec("create in a: check", tostring(r))
  rec("create in a: a reported once", tostring(seen[wa]))
  rec("create in a: b not reported", tostring(seen[wb]))
  settle(m)
  rec("quiet again", tostring(m:check(function() end)))

  -- Several events for the same watch inside one check are reported once.
  touch(ROOT .. "/b/1.txt")
  touch(ROOT .. "/b/2.txt")
  touch(ROOT .. "/b/3.txt")
  os.remove(ROOT .. "/b/1.txt")
  local r2, seen2 = wait_change(m, known)
  rec("burst in b: check", tostring(r2))
  rec("burst in b: deduped to one", tostring(seen2[wb]))
  rec("burst in b: a not reported", tostring(seen2[wa]))
  settle(m)

  -- Deletion and rename are changes too.
  os.remove(ROOT .. "/a/file.txt")
  local r3, seen3 = wait_change(m, known)
  rec("delete: check", tostring(r3))
  rec("delete: a reported", tostring(seen3[wa]))
  settle(m)
  touch(ROOT .. "/a/old.txt")
  settle(m)
  os.rename(ROOT .. "/a/old.txt", ROOT .. "/b/new.txt")
  local r4, seen4 = wait_change(m, known)
  rec("move: check", tostring(r4))
  rec("move: both dirs reported", tostring(seen4[wa]) .. "," .. tostring(seen4[wb]))
  settle(m)

  -- The callback argument is a number in "multiple" mode.
  touch(ROOT .. "/a/arg.txt")
  local argtype
  local t0 = system.get_time()
  repeat
    m:check(function(id)
      if known[id] then argtype = type(id) end
    end)
  until argtype or system.get_time() - t0 > 3
  rec("callback arg type", tostring(argtype))
  settle(m)

  -- Errors raised by the callback go to the error callback; check still succeeds.
  touch(ROOT .. "/a/err.txt")
  local errs, results = 0, {}
  local last
  t0 = system.get_time()
  repeat
    last = m:check(function(id)
      if known[id] then error("boom") end
    end, function(err)
      errs = errs + 1
      results[#results + 1] = tostring(err):match("boom") and "boom" or tostring(err)
    end)
  until last or system.get_time() - t0 > 3
  rec("callback error: check", tostring(last))
  rec("callback error: handler calls", tostring(errs))
  rec("callback error: message", tostring(results[1]))
  settle(m)

  -- Without an error callback a raising callback is swallowed.
  touch(ROOT .. "/a/err2.txt")
  local swallowed
  t0 = system.get_time()
  repeat
    swallowed = status(m.check, m, function(id)
      if known[id] then error("boom") end
    end)
  until swallowed:find("^true|1|true") or system.get_time() - t0 > 3
  rec("callback error swallowed", swallowed)
  settle(m)

  -- Re-entrancy: dirwatch (data/core/dirwatch.lua) calls unwatch() and watch() on the monitor
  -- from inside the check callback, to re-arm a watch on a file that changed. This must work.
  touch(ROOT .. "/a/rearm.txt")
  local rearm = {}
  local t1 = system.get_time()
  repeat
    m:check(function(id)
      if known[id] and not rearm.done then
        rearm.done = true
        rearm.unwatch = status(m.unwatch, m, id)
        rearm.watch = status(m.watch, m, ROOT .. "/b")
        rearm.mode = status(m.mode, m)
      end
    end)
  until rearm.done or system.get_time() - t1 > 3
  rec("reentrant: callback ran", tostring(rearm.done))
  rec("reentrant: unwatch", (tostring(rearm.unwatch):gsub("%d+%.%d+", "<id>")))
  rec("reentrant: watch", (tostring(rearm.watch):gsub("%d+%.%d+", "<id>")))
  rec("reentrant: mode", tostring(rearm.mode))
  -- A `check` from inside the callback is deliberately not recorded: the C build re-read the
  -- same unconsumed buffer, the port takes it first. Nothing calls check re-entrantly.
  settle(m)
  -- unwatch removed wa's watch in the callback; restore it for the steps below.
  wa = m:watch(ROOT .. "/a")
  known[wa] = true
  settle(m)

  -- After unwatch the directory stays silent.
  m:unwatch(wa)
  settle(m)
  touch(ROOT .. "/a/after.txt")
  local r5, seen5 = wait_change(m, known, 0.6)
  rec("after unwatch: a not reported", tostring(seen5[wa]))
  touch(ROOT .. "/b/still.txt")
  local _, seen6 = wait_change(m, known)
  rec("after unwatch: b still reported", tostring(seen6[wb]))
  settle(m)

  -- Argument validation.
  rec("check no args", status(m.check, m))
  rec("check nil callback", status(m.check, m, nil))
  rec("check number callback", status(m.check, m, 5))
  rec("check bad error callback", status(m.check, m, function() end, 5))
  rec("check nil error callback", status(m.check, m, function() end, nil))
  rec("check on non-monitor", status(m.check, {}, function() end))
  rec("watch on non-monitor", status(m.watch, {}, "/tmp"))

  -- A second monitor is independent.
  local m2 = dirmonitor.new()
  rec("second monitor first id", tostring(m2:watch(ROOT .. "/a")))
  rec("second monitor idle", tostring(m2:check(function() end)))

  -- A monitor that never watched has no thread; dropping it must not hang.
  local idle = dirmonitor.new()
  idle = nil
  -- A monitor whose worker is blocked in poll(): dropping it must wake and join it.
  local busy = dirmonitor.new()
  busy:watch(ROOT .. "/b")
  busy = nil
  m2 = nil
  m = nil
  collectgarbage("collect")
  collectgarbage("collect")
  rec("gc joined workers", "ok")

  -- Many monitors: each owns an inotify instance and a thread, so leaking either
  -- would exhaust the per-user limit (typically 128 instances) well before 300.
  local made = 0
  for _ = 1, 300 do
    local x = dirmonitor.new()
    if x:watch(ROOT .. "/a") >= 0 then made = made + 1 end
    x = nil
    collectgarbage("collect")
  end
  rec("300 monitors all watched", tostring(made))

  sh("rm -rf " .. ROOT)
end
