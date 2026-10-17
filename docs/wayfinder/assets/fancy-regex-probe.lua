-- Bounded evidence only. Unsafe-reference checks require the Rust-safe spike adapter.
local M = {}
function M.init()
  require "core.regex"
  local core = require "core"
  core.threads, core.log_items = setmetatable({}, { __mode = "k" }), {}
  local mode = os.getenv("FANCY_CASE") or "dialect"
  if mode == "workload" then
    assert(type(regex.set_limits) == "function", "candidate-only workload sweep")
    regex.set_limits(assert(tonumber(os.getenv("FANCY_WORK"))), assert(tonumber(os.getenv("FANCY_OUTPUT"))))
    return require("regex-release-baseline").init()
  end
  local checks, failures = 0, 0
  local function pack(v)
    local a = {}
    for i = 1, v.n do
      a[i] = type(v[i]) == "string" and string.format("%q", v[i]):gsub("\\\n", "\\n") or tostring(v[i])
    end
    return table.concat(a, " | ")
  end
  local function check(id, expected, fn)
    local r = table.pack(pcall(fn))
    local got = { n = r.n - 1 }; table.move(r, 2, r.n, 1, got)
    local ok = r[1] and got.n == expected.n
    for i = 1, expected.n do ok = ok and got[i] == expected[i] end
    checks = checks + 1
    if not ok then failures = failures + 1 end
    print((ok and "PASS " or "FAIL ") .. id .. ":" .. (got.n > 0 and " " .. pack(got) or ""))
    if not ok then print("expected:" .. (expected.n > 0 and " " .. pack(expected) or "")) end
  end
  local function errors(id, fn, prefix)
    local started = system.get_time()
    check(id, table.pack(false, true), function()
      local ok, msg = pcall(fn)
      return ok, not ok and tostring(msg):find(prefix, 1, true) ~= nil
    end)
    if mode == "limits" then
      local elapsed = system.get_time() - started
      print(string.format("LIMIT\t%s\t-\t%.6f\t%s",id,elapsed,prefix))
      check(id.."_under_second",table.pack(true),function() return elapsed<1 end)
    end
  end
  if mode == "dialect" then
    local cases = {
      {"word_non_ascii", "\\w", "é", table.pack()},
      {"digit_non_ascii", "\\d", "٣", table.pack()},
      {"space_non_ascii", "\\s", "\194\160", table.pack()},
      {"horizontal_space", "\\h", " ", table.pack(1,2)},
      {"Q_pattern_quote", [[\Qa.b\E]], "a.b", table.pack(1,4)},
      {"absolute_start_notbol", [[\Aa]], "a", table.pack(1,2), 1},
      {"absolute_end_noteol", [[a\z]], "a", table.pack(1,2), 2},
      {"global_empty_mixed", nil, nil, table.pack("---",3)},
    }
    for _,c in ipairs(cases) do
      check(c[1], c[4], function()
        if c[1] == "global_empty_mixed" then return regex.gsub("(?:|a)", "a", "-") end
        return regex.cmatch(c[2], c[3], 1, c[5] or 0)
      end)
    end
    require "plugins.language_js"
    local js = require("core.syntax").get("test.js", "")
    for i,p in ipairs(js.patterns) do
      local r = p.regex
      if type(r) == "string" then r = {r} end
      if r then
        for j = 1, math.min(#r,2) do
          local s = r[j]
          local compiled, err = regex.compile(s)
          print("JS_PATTERN " .. i .. ":" .. j .. " " .. string.format("%q", s) .. " => " .. (compiled and "OK" or tostring(err)))
        end
      end
    end
  else
    assert(type(regex.set_limits) == "function", "candidate-only: never run these cases on raw-pointer reference")
    if mode == "safety" then
      check("invalid_pattern_utf_compile_pair", table.pack(true,"string"), function()
        local p,e=regex.compile("\255"); return p==nil,type(e)
      end)
      check("invalid_flag_byte_ignored", table.pack(1,2), function()
        return regex.cmatch(regex.compile("a","\255"),"a")
      end)
      check("middle_cmatch", table.pack(1,2,nil,nil,1,2), function() return regex.cmatch("(a)?(b)","b") end)
      check("middle_find_offsets", table.pack(1,1,nil,nil,1,1), function() return regex.find_offsets("(a)?(b)","b") end)
      check("middle_find", table.pack(1,1,nil,"b"), function() return regex.find("(a)?(b)","b") end)
      check("middle_match", table.pack(nil,"b"), function() return regex.match("(a)?(b)","b") end)
      check("middle_gmatch", table.pack(nil,"b"), function() return regex.gmatch("(a)?(b)","b")() end)
      check("trailing_omitted", table.pack(1,2,1,2), function() return regex.cmatch("(a)(b)?","a") end)
      check("iterator_keeps_pattern_after_GC", table.pack("a","a",0), function()
        local p = regex.compile("a")
        local iter = regex.gmatch(p,"aa")
        p=nil; collectgarbage("collect"); collectgarbage("collect")
        local a,b=iter(),iter(); return a,b,select("#",iter())
      end)
      errors("bad_handle", function() return regex.cmatch({},"a") end, "REGEX_HANDLE_ERROR")
      errors("foreign_handle", function() return regex.cmatch({io.stdout},"a") end, "REGEX_HANDLE_ERROR")
      errors("offset_past_end", function() return regex.cmatch(".","é",4) end, "REGEX_OFFSET_ERROR")
      errors("offset_inside_utf8", function() return regex.cmatch(".","é",2) end, "REGEX_OFFSET_ERROR")
      check("offset_at_eof", table.pack(3,3,3,3), function() return regex.cmatch("()","é",3) end)
      check("mixed_empty_nonempty_iteration", table.pack(1,"a",2,0), function()
        local i=regex.gmatch("(?:|a)","a")
        local a,b,c=i(),i(),i(); return a,b,c,select("#",i())
      end)
      check("iterator_keeps_prefix", table.pack("a"), function() return regex.gmatch("(?<=b)a","ba",2)() end)
      check("notempty_same_position", table.pack(1,2), function() return regex.cmatch("(?:|b)","b",1,regex.NOTEMPTY) end)
    elseif mode == "limits" then
      for _,work in ipairs({1000,10000,100000,1000000}) do
        regex.set_limits(work,8*1024*1024)
        for _,c in ipairs({ {"backreference_fail", "(a|aa)+\\1b", string.rep("a",5000)},
                             {"nested_backreference_fail", "((a|aa)+)+\\1b", string.rep("a",128)} }) do
          local p = assert(regex.compile(c[2]))
          local t=system.get_time()
          local ok,msg=pcall(regex.cmatch,p,c[3])
          local elapsed=system.get_time()-t
          print(string.format("LIMIT\t%s\t%d\t%.6f\t%s",c[1],work,elapsed,tostring(msg):match("REGEX_WORK_LIMIT") or tostring(ok)))
          check(c[1].."_"..work,table.pack(false,true,true),function()
            return ok, tostring(msg):find("REGEX_WORK_LIMIT",1,true)~=nil, elapsed<1
          end)
        end
      end
      regex.set_limits(1000000,65536)
      errors("output_exhaustion_no_partial", function()
        return regex.gsub("a",string.rep("a",4096),string.rep("x",1024))
      end,"REGEX_OUTPUT_LIMIT")
      regex.set_limits(1000,8*1024*1024)
      errors("aggregate_call_quota", function() return regex.gsub("a",string.rep("a",5000),"b") end,"REGEX_WORK_LIMIT")
      regex.set_limits(1,8*1024*1024)
      check("delegated_scan_ignores_character_work_quota",table.pack(0),function()
        return select("#",regex.cmatch("z",string.rep("a",1024*1024)))
      end)
    else error("unknown FANCY_CASE") end
  end
  print(string.format("SUMMARY mode=%s checks=%d failures=%d",mode,checks,failures))
  os.exit(failures==0 and 0 or 1)
end
function M.run() end
return M
