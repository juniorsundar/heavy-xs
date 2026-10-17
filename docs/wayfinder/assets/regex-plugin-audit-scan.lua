RAW={}
-- Extract every regex expression from Lua plugin sources (syntax `regex =` rules,
-- regex.compile/match/find/gsub/cmatch/gmatch string-literal arguments) and
-- record PCRE2-flag and replacement-template usage. Prints TSV.
local function lit(s, i)
  -- read a Lua string literal at position i; returns value, next index
  local c = s:sub(i, i)
  if c == '"' or c == "'" then
    local j, out = i + 1, {}
    while j <= #s do
      local ch = s:sub(j, j)
      if ch == "\\" then
        local n = s:sub(j + 1, j + 1)
        local map = { n = "\n", t = "\t", r = "\r", ["\\"] = "\\", ['"'] = '"', ["'"] = "'" }
        if map[n] then out[#out + 1] = map[n]; j = j + 2
        elseif n:match("%d") then local d = s:match("^%d%d?%d?", j + 1); out[#out + 1] = string.char(tonumber(d) % 256); j = j + 1 + #d
        else out[#out + 1] = "\\" .. n; j = j + 2 end
      elseif ch == c then return table.concat(out), j + 1
      elseif ch == "\n" then return nil
      else out[#out + 1] = ch; j = j + 1 end
    end
  elseif c == "[" then
    local eq = s:match("^%[(=*)%[", i)
    if eq then
      local open = i + #eq + 2
      local close = s:find("]" .. eq .. "]", open, true)
      if close then
        local v = s:sub(open, close - 1):gsub("^\n", "")
        return v, close + #eq + 2
      end
    end
  end
end

local out = {}
for path in io.lines() do
  local f = io.open(path); local s = f and f:read("a") or ""; if f then f:close() end
  local function add(kind, pos, v) if v and #v > 0 and (kind == "rule" or kind:match("^call:")) then RAW[#RAW+1]={path..":"..(select(2, s:sub(1, pos):gsub("\n", "")) + 1), v} end if v and #v > 0 then out[#out + 1] = table.concat({ kind, path, select(2, s:sub(1, pos):gsub("\n", "")) + 1, string.format("%q", v):gsub("\n", "n") }, "\t") end end
  -- syntax rules: regex = "..." or regex = { "...", "...", "\\" }
  for pos in s:gmatch("()regex%s*=%s*") do
    local i = s:find("[^%s]", s:find("=", pos) + 1)
    if s:sub(i, i) == "{" then
      local j = i + 1
      for _ = 1, 2 do
        j = s:find("[^%s,]", j)
        local v, nx = lit(s, j); if not v then break end
        add("rule", j, v); j = nx
      end
    else add("rule", i, (lit(s, i))) end
  end
  for pos, fn in s:gmatch("()regex%.(%a+)%s*%(") do
    local i = s:find("[^%s]", s:find("%(", pos) + 1)
    if fn == "compile" or fn == "match" or fn == "find" or fn == "find_offsets" or fn == "cmatch" or fn == "gmatch" or fn == "gsub" then
      add("call:" .. fn, i, (lit(s, i)))
    end
  end
  for pos, flag in s:gmatch("()regex%.(%u[%u_]+)") do add("flag", pos, flag) end
  for pos in s:gmatch("()regex%.gsub") do add("gsub_site", pos, s:match("[^\n]*", pos)) end
  for pos in s:gmatch("():gsub%(") do
    local line = s:match("[^\n]*", math.max(1, (s:sub(1, pos):match(".*()\n") or 1)))
    if line:find("reg") then add("method_gsub_site", pos, line) end
  end
end
io.write(table.concat(out, "\n"), "\n") local raw=io.open("/tmp/lxl-patterns.bin","wb") for _,r in ipairs(RAW) do raw:write(r[1],"\t",r[2],"\0") end raw:close()
