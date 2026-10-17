-- regex-release-baseline.lua — bounded release-workload evidence runtime for Wayfinder issue
-- "Collect the release regex workload baseline" (docs/wayfinder/issues/08-...md). Not
-- production code / not a framework: one process runs one BASELINE_CASE and writes TSV to
-- stdout, exiting non-zero on any correctness or compile failure. Fixtures and created files
-- live under the disposable BASELINE_DIR from the caller; contractplug.c is read from
-- BASELINE_ROOT (public repo root). No private paths or matched output text are printed.
--
-- Cases (12; caps 1 MiB / 16384 lines / 65536 B longest line): hl_small/hl_mid/hl_large = JS
-- highlighter initial pass at 128/2048/8192 synthetic lines; hl_long = 128 plus a 32768 B
-- comment line; hl_edit = initial pass + 12 incremental insert/re-highlight edits; search_first /
-- search_miss = one core.doc.search.find call: first hit / full miss on a 2048-line UTF-8
-- fixture (search_first does NOT enumerate its dense matches); project_regex =
-- plugins.projectsearch.search_regex over 8 files (contractplug.c x1/x16/x128); replace_doc =
-- Doc:replace with the findreplace regex.compile("m")/gsub closure; syn_small/syn_mid/
-- syn_large = synthetic compile/match/gsub method flow, unit x128/2048/8192.
--
-- TSV: META case kind pattern flags bytes lines longest files hash matches output; SAMPLE case
-- rep action_s max_resume_s resumes yields lua_pre/post/clean_kb rss_pre/post/clean_kb
-- hwm_clean_kb matches output; COMPILE case attempt compile_s status; SUMMARY case checks
-- failures result; BASELINE_COMPLETE case checks failures.
--
-- rep 0 warms caches, 1..5 measured; action_s excludes setup. max_resume_s is the longest
-- coroutine.resume while driving real Highlighter/projectsearch threads, including the
-- synchronous action/edit prefix before its first resume; synchronous cases use
-- action_s with resumes=1, yields=0. lua_*_kb = collectgarbage("count") after two GCs;
-- rss/hwm = VmRSS/VmHWM kB from /proc/self/status. pre = before setup, post = action done with
-- outputs/undo retained, clean = after cleanup+GC. None is UI latency. COMPILE = 5 independent
-- regex.compile calls (allocator-warm, not disk-cold). Hooks: no core.init(); only
-- core.threads/log_items/projects/redraw and a root_view add_view stub are seeded so the real
-- search_regex -> begin_search attachment works headless. Errors go to stderr with a non-zero
-- exit. hash = FNV-1a 32-bit over fixture bytes.

local M = {}
local core = require "core"
core.log_items = core.log_items or {}
local search = require "core.doc.search"
local Doc = require "core.doc"
local Project = require "core.project"
local projectsearch = require "plugins.projectsearch"
require "plugins.language_js"
local js_pattern
for _, pattern in ipairs(require("core.syntax").get("baseline.js", "").patterns) do
  local expression = type(pattern.regex) == "table" and pattern.regex[1] or pattern.regex
  if type(expression) == "string" and expression:find("?>", 1, true) then
    js_pattern = expression
  end
end
assert(js_pattern, "bundled JavaScript atomic regex missing")

local CASE = os.getenv("BASELINE_CASE") or ""
local ROOT = os.getenv("BASELINE_ROOT") or ""
local DIR = os.getenv("BASELINE_DIR") or ""
local checks, failures = 0, 0

io.stdout:setvbuf("line")
local function emit(...) io.stdout:write(table.concat({ ... }, "\t"), "\n") end

local function eq(id, actual, expected)
  checks = checks + 1
  if actual ~= expected then
    failures = failures + 1
    emit("CHECK", CASE, "fail", id, tostring(actual), tostring(expected))
  end
end

local function fnv(text)
  local h = 2166136261
  for i = 1, #text do h = ((h ~ text:byte(i)) * 16777619) & 0xffffffff end
  return string.format("%08x", h)
end

local function stats(text)
  local lines, longest = 0, 0
  for line in text:gmatch("([^\n]*)\n") do
    lines = lines + 1
    if #line > longest then longest = #line end
  end
  if lines == 0 then longest = #text end
  return #text, lines, longest
end

local function readall(path)
  local f = assert(io.open(path, "rb"), path)
  local data = f:read("*a")
  f:close()
  return data
end

local function write(path, data)
  local f = assert(io.open(path, "wb"), path)
  f:write(data)
  f:close()
end

local function snapshot()
  collectgarbage("collect"); collectgarbage("collect")
  local rss, hwm, f = 0, 0, io.open("/proc/self/status", "r")
  if f then
    for line in f:lines() do
      rss = tonumber(line:match("^VmRSS:%s+(%d+)")) or rss
      hwm = tonumber(line:match("^VmHWM:%s+(%d+)")) or hwm
    end
    f:close()
  end
  return { lua = collectgarbage("count"), rss = rss, hwm = hwm }
end

local function bootstrap()
  core.threads = setmetatable({}, { __mode = "k" })
  core.log_items, core.projects, core.redraw = {}, {}, true
  -- Disclosed probe-local hook: only carries the real begin_search view attachment.
  core.root_view = { get_active_node_default = function() return { add_view = function() end } end }
end

-- Resume every live core.threads coroutine until dead, as core's frame loop would.
local function drive(prefix_started)
  local resumes, yields, maxdt = 0, 0, 0
  while next(core.threads) do
    for k, th in pairs(core.threads) do
      if core.threads[k] then
        local t0 = prefix_started or system.get_time()
        prefix_started = nil
        local ok, err = coroutine.resume(th.cr)
        assert(ok, err)
        local dt = system.get_time() - t0
        resumes = resumes + 1
        if dt > maxdt then maxdt = dt end
        if coroutine.status(th.cr) == "dead" then core.threads[k] = nil
        else yields = yields + 1; th.wake = 0 end
      end
    end
    assert(resumes < 500000, "thread drive made no progress")
  end
  return resumes, yields, maxdt
end

local function flush_threads() core.threads = setmetatable({}, { __mode = "k" }) end

local function error_logs()
  local n = 0
  for _, item in ipairs(core.log_items) do if item.level == "ERROR" then n = n + 1 end end
  return n
end

local JS_UNIT = { "let re = /[a-z]+\\/[a-z]*/gi;", "function f(a) { return a + 1; }",
  "const n = 0x1fUL; // 0b1010u", "/* comment */ let s = \"café\";" }

local function js_text(nlines, longest)
  local out, n = {}, 0
  while n < nlines do
    for i = 1, #JS_UNIT do
      out[#out + 1] = JS_UNIT[i] .. "\n"
      n = n + 1
      if n >= nlines then break end
    end
  end
  if longest then out[#out + 1] = "// " .. string.rep("x", longest - 3) .. "\n" end
  return table.concat(out)
end

local function rep_text(fmt, nlines)
  local out = {}
  for i = 1, nlines do out[i] = string.format(fmt, i, i, i) end
  return table.concat(out)
end

local function hl_case(name, nlines, longest, edit)
  return {
    kind = edit and "highlighter-incremental" or "highlighter-initial",
    pattern = js_pattern, flags = "",
    build = function()
      local text = js_text(nlines, longest)
      local path = DIR .. PATHSEP .. name .. ".js"
      write(path, text)
      local b, l, w = stats(text)
      return { file = path, bytes = b, lines = l, longest = w, hash = fnv(text) }
    end,
    setup = function(fx)
      local doc = Doc(fx.file, fx.file)
      assert(doc.syntax.name == "JavaScript", "highlighter fixture did not select JavaScript")
      if not edit then return { doc = doc } end
      doc.highlighter:get_line(#doc.lines); drive()
      return { doc = doc, before = #doc.lines }
    end,
    action = function(ctx)
      local started = system.get_time()
      local doc, h = ctx.doc, ctx.doc.highlighter
      local resumes, yields, maxdt = 0, 0, 0
      if edit then
        for i = 1, 12 do
          local step_started = system.get_time()
          local line = 2 + (i % 15)
          doc:insert(line, 1, "// edit " .. i .. "\n")
          h:get_line(math.min(line + 60, #doc.lines))
          local r, y, m = drive(step_started)
          resumes, yields = resumes + r, yields + y
          if m > maxdt then maxdt = m end
        end
      else
        h:get_line(#doc.lines)
        resumes, yields, maxdt = drive(started)
      end
      local tok = 0
      for i = 1, #doc.lines do if h.lines[i] then tok = tok + 1 end end
      return { resumes = resumes, yields = yields, max_resume = maxdt,
               matches = edit and (#doc.lines - ctx.before) or tok, output = #doc.lines }
    end,
    check = function(ctx, outs)
      local doc, h = ctx.doc, ctx.doc.highlighter
      eq(name .. ":complete", h.first_invalid_line > h.max_wanted_line, true)
      eq(name .. ":lines", #h.lines, #doc.lines)
      local tokens, has_regex_string = h:get_line(1).tokens, false
      for i = 1, #tokens, 2 do
        if tokens[i] == "string" then has_regex_string = true end
      end
      eq(name .. ":regex_literal", has_regex_string, true)
      if edit then
        eq(name .. ":added", outs.matches, 12)
        eq(name .. ":doc_lines", #doc.lines, ctx.before + 12)
      else
        eq(name .. ":tokenized", outs.matches, #doc.lines)
      end
      eq(name .. ":threads", next(core.threads), nil)
      eq(name .. ":errors", error_logs(), 0)
    end,
    clean = function(ctx) ctx.doc = nil; flush_threads() end,
  }
end

local function search_case(name, hit)
  return {
    kind = hit and "doc-search-hit" or "doc-search-miss",
    pattern = hit and "café" or "日本語", flags = "",
    build = function()
      local text = rep_text("line%d café αβγ naïve résumé = %d\n", 2048)
      local path = DIR .. PATHSEP .. "utf8.txt"
      write(path, text)
      local b, l, w = stats(text)
      return { file = path, bytes = b, lines = l, longest = w, hash = fnv(text) }
    end,
    setup = function(fx) return { doc = Doc(fx.file, fx.file) } end,
    action = function(ctx, fx)
      local result = { search.find(ctx.doc, 1, 1, fx.pattern, { regex = true }) }
      return { result = result, matches = result[1] and 1 or 0, output = 0 }
    end,
    check = function(ctx, outs)
      if hit then
        eq(name .. ":line", outs.result[1], 1)
        eq(name .. ":end", outs.result[2] ~= nil, true)
      else
        eq(name .. ":miss", outs.result[1], nil)
      end
    end,
    clean = function(ctx) ctx.doc = nil; flush_threads() end,
  }
end

local function project_case()
  return {
    kind = "project-search-regex", pattern = "lua_pushinteger", flags = "",
    build = function()
      local base = readall(ROOT .. "/tests/contract/plugin/contractplug.c")
      if base:sub(-1) ~= "\n" then base = base .. "\n" end
      local dir = DIR .. PATHSEP .. "proj"
      system.mkdir(dir)
      local expected, bytes, lines, longest, all = 0, 0, 0, 0, {}
      for i, k in ipairs({ 1, 16, 128, 1, 16, 16, 1, 16 }) do
        local data = string.rep(base, k)
        write(dir .. PATHSEP .. ("f%d.c"):format(i), data)
        for line in data:gmatch("([^\n]*)\n") do
          if line:find("lua_pushinteger", 1, true) then expected = expected + 1 end
        end
        local b, l, w = stats(data)
        bytes, lines = bytes + b, lines + l
        if w > longest then longest = w end
        all[#all + 1] = data
      end
      return { dir = dir, expected = expected, bytes = bytes, lines = lines, longest = longest,
               files = 8, hash = fnv(table.concat(all)) }
    end,
    setup = function(fx) core.projects = { Project(fx.dir) }; return {} end,
    action = function(ctx, fx)
      local started = system.get_time()
      local rv = projectsearch.search_regex(fx.pattern, nil, false)
      local resumes, yields, maxdt = drive(started)
      return { resumes = resumes, yields = yields, max_resume = maxdt, rv = rv,
               matches = #rv.results, output = 0 }
    end,
    check = function(ctx, outs, fx)
      eq("project:searching", outs.rv.searching, false)
      eq("project:results", #outs.rv.results, fx.expected)
      eq("project:threads", next(core.threads), nil)
      eq("project:errors", error_logs(), 0)
    end,
    clean = function(ctx, fx, outs) outs.rv = nil; core.projects = {}; flush_threads() end,
  }
end

local function replace_case()
  return {
    kind = "doc-replace-findreplace", pattern = "name", flags = "m",
    build = function()
      local text = rep_text("let name%d = name + %d; name = name%d;\n", 512)
      local path = DIR .. PATHSEP .. "replace.js"
      write(path, text)
      local b, l, w = stats(text)
      return { file = path, bytes = b, lines = l, longest = w, hash = fnv(text),
               replacement = "label", expected = 2048 }
    end,
    setup = function(fx) return { doc = Doc(fx.file, fx.file) } end,
    action = function(ctx, fx)
      -- Same closure shape as core.commands.findreplace find_replace regex mode.
      local results = ctx.doc:replace(function(text)
        return regex.gsub(regex.compile(fx.pattern, "m"), text, fx.replacement)
      end)
      return { n = results[1], matches = results[1], output = #table.concat(ctx.doc.lines) }
    end,
    check = function(ctx, outs, fx)
      eq("replace:count", outs.n, fx.expected)
      eq("replace:gone", table.concat(ctx.doc.lines):find("name", 1, true), nil)
      eq("replace:threads", next(core.threads), nil)
      eq("replace:errors", error_logs(), 0)
    end,
    clean = function(ctx) ctx.doc = nil; flush_threads() end,
  }
end

local function syn_case(name, units)
  return {
    kind = "synthetic-snippet-methods", pattern = "([a-z]+)-([a-z]+)", flags = "",
    build = function()
      local subject = string.rep("alpha-beta.Widget.TS; name=Name;\n", units)
      local bytes, lines, longest = stats(subject)
      return { subject = subject, units = units, bytes = bytes, lines = lines,
               longest = longest, hash = fnv(subject) }
    end,
    setup = function(fx) return { subject = fx.subject } end,
    action = function(ctx, fx)
      local s = fx.subject
      local out, n = regex.compile("([a-z]+)-([a-z]+)"):gsub(s, "$2-$1")
      local c1, c2 = regex.compile("([a-z]+)-([a-z]+)"):match(s)
      local out2, n2 = regex.compile("name", "i"):gsub(s, "VALUE")
      return { n = n, out_len = #out, text = out, text2 = out2,
               c1 = c1, c2 = c2, n2 = n2, out2_len = #out2,
               matches = n, output = #out + #out2 }
    end,
    check = function(ctx, outs, fx)
      eq(name .. ":gsub_count", outs.n, fx.units)
      eq(name .. ":gsub_len", outs.out_len, fx.bytes)
      eq(name .. ":cap1", outs.c1, "alpha")
      eq(name .. ":cap2", outs.c2, "beta")
      eq(name .. ":i_count", outs.n2, fx.units * 2)
      eq(name .. ":i_len", outs.out2_len, fx.bytes + fx.units * 2)
    end,
    clean = function(_, _, outs)
      outs.text, outs.text2 = nil, nil
      flush_threads()
    end,
  }
end

local cases = {
  hl_small = hl_case("hl_small", 128, nil), hl_mid = hl_case("hl_mid", 2048, nil),
  hl_large = hl_case("hl_large", 8192, nil), hl_edit = hl_case("hl_edit", 2048, nil, true),
  hl_long = hl_case("hl_long", 128, 32768),
  search_first = search_case("search_first", true), search_miss = search_case("search_miss", false),
  project_regex = project_case(), replace_doc = replace_case(),
  syn_small = syn_case("syn_small", 128), syn_mid = syn_case("syn_mid", 2048),
  syn_large = syn_case("syn_large", 8192),
}

local function compile_phase(c)
  for i = 1, 5 do
    local t0 = system.get_time()
    local re, err
    if c.flags == "" then re, err = regex.compile(c.pattern)
    else re, err = regex.compile(c.pattern, c.flags) end
    checks = checks + 1
    if not re then failures = failures + 1 end
    emit("COMPILE", CASE, i, string.format("%.6f", system.get_time() - t0),
      re and "ok" or ("error:" .. tostring(err)))
  end
end

local function measure(c, fx, rep)
  local pre = snapshot()
  local ctx = c.setup(fx, rep)
  local t0 = system.get_time()
  local outs = c.action(ctx, fx)
  local action_s = system.get_time() - t0
  local post = snapshot()
  c.check(ctx, outs, fx)
  c.clean(ctx, fx, outs)
  ctx = nil
  return { rep = rep, outs = outs, action_s = action_s, pre = pre, post = post, clean = snapshot() }
end

local function emit_sample(r)
  local o = r.outs
  emit("SAMPLE", CASE, r.rep, string.format("%.6f", r.action_s),
    string.format("%.6f", o.max_resume or r.action_s), o.resumes or 1, o.yields or 0,
    string.format("%.2f", r.pre.lua), string.format("%.2f", r.post.lua),
    string.format("%.2f", r.clean.lua), r.pre.rss, r.post.rss, r.clean.rss, r.clean.hwm,
    o.matches or 0, o.output or 0)
end

local function run()
  assert(CASE ~= "", "BASELINE_CASE required")
  assert(ROOT ~= "", "BASELINE_ROOT required")
  local info = system.get_file_info(DIR)
  assert(info and info.type == "dir", "BASELINE_DIR must be an existing directory")
  bootstrap()
  local c = assert(cases[CASE], "unknown BASELINE_CASE: " .. CASE)
  emit("#", "regex-release-baseline TSV: META SAMPLE COMPILE SUMMARY BASELINE_COMPLETE")
  local fx = c.build()
  fx.pattern = c.pattern
  emit("FIXTURE", CASE, fx.bytes, fx.lines, fx.longest, fx.files or 1, fx.hash)
  assert(fx.bytes <= 1048576 and fx.lines <= 16384 and fx.longest <= 65536,
    "fixture exceeds predeclared survey envelope")
  compile_phase(c)
  local warm = measure(c, fx, 0)
  emit("META", CASE, c.kind, c.pattern, c.flags, fx.bytes, fx.lines, fx.longest, fx.files or 1,
    fx.hash, warm.outs.matches or 0, warm.outs.output or 0)
  emit_sample(warm)
  for rep = 1, 5 do emit_sample(measure(c, fx, rep)) end
  emit("SUMMARY", CASE, checks, failures, failures == 0 and "PASS" or "FAIL")
  emit("BASELINE_COMPLETE", CASE, checks, failures)
end

function M.init()
  local ok, err = xpcall(run, function(e) return tostring(e) .. "\n" .. debug.traceback("", 2) end)
  if not ok then
    io.stderr:write("BASELINE_ERROR\t", CASE, "\t", tostring(err), "\n")
    os.exit(1)
  end
  os.exit(failures == 0 and 0 or 1)
end

function M.run() end

return M
