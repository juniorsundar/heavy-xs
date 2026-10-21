-- W3 contract suite: the `process` module (src/api/process.c) through the
-- data/core/process.lua wrapper, which is what plugins actually use.
--
-- The golden is a historical recording from the C build. Cases that involve
-- timing or the kill-list worker poll with generous timeouts and record only a
-- boolean, so the output stays stable run to run.

local process = require "core.process"
local system = require "system"

-- contract.lua's rec() escapes control bytes but not newlines, which would
-- split a baseline line in two.
local function one_line(s)
  return (tostring(s):gsub("\n", "\\n"))
end

local function select_n(fn, ...)
  local r = table.pack(pcall(fn, ...))
  local out = { tostring(r[1]) }
  if r[1] then
    out[#out + 1] = tostring(r.n - 1) -- number of return values
    for i = 2, r.n do out[#out + 1] = tostring(r[i]) end
  end
  return table.concat(out, "|")
end

-- Waits for the process to exit so the rest of a case is race free.
local function wait_exit(p, ms)
  p:wait(ms or 5000)
  return p:returncode()
end

-- Reads a stream until EOF. `read_stdout` returns "" both for "nothing yet"
-- and for EOF, so the loop only stops once the process is gone too.
local function drain(p, method, limit)
  local parts = {}
  local last = system.get_time()
  while true do
    local chunk = p[method](p)
    if #chunk > 0 then
      parts[#parts + 1] = chunk
      last = system.get_time()
    elseif not p:running() then
      break
    elseif system.get_time() - last > (limit or 3) then
      break
    end
  end
  return table.concat(parts)
end

-- Starts a command, lets it finish, returns exit code plus output.
local function capture(args, opts, method)
  local p = process.start(args, opts)
  local code = wait_exit(p)
  return p, code, drain(p, method or "read_stdout")
end

local function wait_dead(p, limit)
  local last = system.get_time()
  while p:running() do
    if system.get_time() - last > (limit or 3) then return false end
  end
  return true
end

return function(rec)
  local live = {} -- everything still running at the end of the suite

  -- Module constants and helpers.
  for _, k in ipairs({
    "WAIT_INFINITE", "WAIT_DEADLINE",
    "STREAM_STDIN", "STREAM_STDOUT", "STREAM_STDERR",
    "REDIRECT_DEFAULT", "REDIRECT_STDOUT", "REDIRECT_STDERR",
    "REDIRECT_PARENT", "REDIRECT_DISCARD",
  }) do
    rec("const " .. k, tostring(process[k]))
  end
  rec("strerror ENOENT", process.strerror(2))
  rec("strerror 0", process.strerror(0))
  rec("strerror huge", select_n(process.strerror, 999999))
  rec("start is function", type(process.start))
  rec("metatable __gc", type(getmetatable(process).__gc))

  -- Exit codes and output.
  local p, code, out = capture({ "/bin/sh", "-c", "printf abc" })
  rec("echo code", code)
  rec("echo stdout", out)
  rec("echo stdout at EOF", "[" .. p:read_stdout() .. "]")
  rec("echo stderr at EOF", "[" .. p:read_stderr() .. "]")
  rec("type", type(p))
  rec("metatable is wrapper", tostring(getmetatable(p) == process))
  rec("pid positive", p:pid() > 0)
  rec("running after exit", tostring(p:running()))
  rec("returncode after exit", select_n(p.returncode, p))
  rec("wait after exit", select_n(p.wait, p, 100))
  rec("read after exit", select_n(p.read_stdout, p))

  local _, ec = capture({ "/bin/sh", "-c", "exit 7" })
  rec("exit 7", ec)
  local _, ec2 = capture({ "/bin/sh", "-c", "kill -TERM $$" })
  rec("signal exit code", ec2)

  local _, ecode, eout = capture({ "/bin/sh", "-c", "printf err 1>&2" }, nil, "read_stderr")
  rec("stderr code", ecode)
  rec("stderr out", eout)
  local _, _, eout2 = capture({ "/bin/sh", "-c", "printf err 1>&2" }, nil, "read_stdout")
  rec("stdout empty when stderr used", eout2)

  -- Returning 0 values while still running.
  local slow = process.start({ "sleep", "5" })
  live[#live + 1] = slow
  rec("returncode while running", select_n(slow.returncode, slow))
  rec("wait 0 while running", select_n(slow.wait, slow, 0))
  rec("running", tostring(slow:running()))
  rec("pid matches", slow:pid() > 0)

  -- read/write on streams.
  local cat = process.start({ "cat" })
  live[#live + 1] = cat
  cat:write("hello")
  rec("write returns count", cat:write("!"))
  rec("close_stream", tostring(cat:close_stream(process.STREAM_STDIN)))
  rec("close_stream twice", tostring(cat:close_stream(process.STREAM_STDIN)))
  rec("cat code", wait_exit(cat))
  rec("cat stdout", drain(cat, "read_stdout"))
  rec("cat stderr at EOF", "[" .. cat:read_stderr() .. "]")

  local p2 = process.start({ "/bin/sh", "-c", "printf 1234567890" })
  wait_exit(p2)
  rec("read sized 1", p2:read(process.STREAM_STDOUT, 4))
  rec("read sized 2", p2:read(process.STREAM_STDOUT, 4))
  rec("read sized 3", "[" .. p2:read(process.STREAM_STDOUT, 4) .. "]")
  rec("read stdout shorthand", "[" .. p2:read_stdout() .. "]")

  local p3 = process.start({ "/bin/sh", "-c", "printf 1234567890" })
  wait_exit(p3)
  rec("read size 0", select_n(p3.read, p3, process.STREAM_STDOUT, 0))
  rec("read size -1", select_n(p3.read, p3, process.STREAM_STDOUT, -1))
  rec("read stdin stream", select_n(p3.read, p3, process.STREAM_STDIN, 8))
  rec("read bad stream", select_n(p3.read, p3, process.REDIRECT_DISCARD, 8))

  -- The wrapper's stream object.
  local p4 = process.start({ "/bin/sh", "-c", "printf 'a\\nb\\n'" })
  wait_exit(p4)
  rec("stream read one byte", p4.stdout:read(1))
  rec("stream read all", one_line(p4.stdout:read("all")))
  rec("stream read at EOF", select_n(p4.stdout.read, p4.stdout, 10))
  rec("stream close", tostring(p4.stdout:close()))

  -- Redirects.
  local _, _, rout = capture({ "/bin/sh", "-c", "printf out" },
                             { stdout = process.REDIRECT_STDERR }, "read_stderr")
  rec("redirect stdout to stderr", rout)
  local _, _, dout = capture({ "/bin/sh", "-c", "printf out" },
                             { stdout = process.REDIRECT_DISCARD })
  rec("redirect discard", "[" .. dout .. "]")
  local p5 = process.start({ "/bin/sh", "-c", "printf out" },
                           { stdout = process.REDIRECT_DISCARD })
  rec("redirect discard code", wait_exit(p5))
  local _, pc = capture({ "/bin/sh", "-c", "exit 3" }, { stdout = process.REDIRECT_PARENT })
  rec("redirect parent code", pc)
  rec("bad stdout option", select_n(process.start, { "sleep", "1" }, { stdout = 5 }))
  rec("bad stdout low", select_n(process.start, { "sleep", "1" }, { stdout = -4 }))

  -- cwd and env.
  local _, _, cwdout = capture({ "/bin/sh", "-c", "pwd" }, { cwd = "/tmp" })
  rec("cwd", one_line(cwdout))
  local _, _, envout = capture({ "/bin/sh", "-c", "printf %s \"$FOO\"" },
                               { env = { FOO = "bar" } })
  rec("env", envout)
  -- On POSIX the block is added to the inherited environment, it does not
  -- replace it, so BAR must still be unset while FOO is set.
  local _, _, envonly = capture(
    { "/bin/sh", "-c", "printf %s \"${FOO:-unset}\"; printf ' '; printf %s \"${BAR:-unset}\"" },
    { env = { FOO = "bar" } })
  rec("env does not replace", envonly)
  rec("env not a function", select_n(function()
    local r = process.start({ "sleep", "1" }, { env = {} })
    return type(r)
  end))

  -- Missing executables and bad arguments.
  rec("missing exe", select_n(process.start, { "definitely_not_a_command_xyz" }))
  rec("string command", select_n(process.start, "echo hi"))
  rec("no arguments", select_n(process.start))
  rec("bad command type", select_n(process.start, 42))
  rec("bad options type", select_n(process.start, { "sleep", "1" }, 5))

  -- The deadline option drives WAIT_DEADLINE.
  local dl = process.start({ "sleep", "5" }, { timeout = 1 })
  live[#dl + 1] = dl
  rec("wait deadline", select_n(dl.wait, dl, process.WAIT_DEADLINE))
  rec("wait infinite polls", select_n(dl.wait, dl, 1))

  -- Signals.
  local sig = process.start({ "sleep", "5" })
  rec("terminate", tostring(sig:terminate()))
  rec("terminate then dead", tostring(wait_dead(sig)))
  rec("terminated returncode", select_n(sig.returncode, sig))
  local sig2 = process.start({ "sleep", "5" })
  rec("kill", tostring(sig2:kill()))
  rec("kill then dead", tostring(wait_dead(sig2)))
  local sig3 = process.start({ "sleep", "5" })
  rec("interrupt", tostring(sig3:interrupt()))
  rec("interrupt then dead", tostring(wait_dead(sig3)))
  rec("running after term", tostring(sig:running()))
  rec("pid still readable", sig:pid() > 0)

  -- Detached processes are not terminated by the destructor.
  local det = process.start({ "sleep", "5" }, { detach = true })
  rec("detached running", tostring(det:running()))
  rec("detached terminate", tostring(det:terminate()))
  rec("detached dead", tostring(wait_dead(det)))

  -- Every live process must be gone before the destructor case below.
  for _, proc in ipairs(live) do
    if proc:running() then proc:terminate() end
  end
  for _, proc in ipairs(live) do wait_dead(proc, 2) end

  -- Dropping the userdata must terminate the child through the kill-list
  -- worker (or synchronously if the list is gone). /proc tells us it is gone.
  local victim_pid
  do
    local victim = process.start({ "sleep", "30" })
    victim_pid = victim:pid()
    victim = nil
  end
  collectgarbage("collect")
  collectgarbage("collect")
  local last = system.get_time()
  local gone = false
  while system.get_time() - last < 3 do
    local f = io.open("/proc/" .. victim_pid)
    if f then f:close() else gone = true break end
  end
  rec("gc terminated child", tostring(gone))
  local f = io.open("/proc/" .. victim_pid)
  if f then f:close() rec("victim still alive", "yes") else rec("victim still alive", "no") end

  -- The module table's destructor drains the kill list and joins the worker.
  -- Called manually because the harness leaves through os.exit, which skips
  -- finalizers; calling it twice would be a double free in C.
  rec("module gc", select_n(getmetatable(process).__gc, process))
end
