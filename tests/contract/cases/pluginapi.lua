-- The native-plugin ABI. tests/contract/plugin/apiplug.c is a real plugin,
-- compiled against the shipped lite_xl_plugin_api.h without linking Lua, that calls (nearly)
-- every function of the API through the XL resolver. The baseline is recorded from the upstream
-- C build, so it pins what third-party plugins actually see: each call's return values and Lua
-- state effects, error text, and the standard-library surface of the Lua they run in.
local ROOT = "/tmp/lxl_pluginapi_contract"
local SRC = os.getenv("LXL_PLUGIN_SRC") or "tests/contract/plugin"
local INC = os.getenv("LXL_PLUGIN_INC") or "resources/include"

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  local out = { r[1] and "ok" or "ERR" }
  for i = 2, r.n do
    local v = r[i]
    if type(v) == "number" then out[#out + 1] = math.type(v) .. ":" .. tostring(v)
    elseif type(v) == "string" then out[#out + 1] = ("%q"):format(v):gsub("\\\n", "\\n")
    else out[#out + 1] = type(v) == "table" and "table" or tostring(v):gsub("0x%x+", "<ptr>") end
  end
  return table.concat(out, " ")
end

-- Error text reduced to its stable shape (no addresses, no stack traces).
local function err(fn, ...)
  local ok, e = pcall(fn, ...)
  if ok then return "no error" end
  e = tostring(e):gsub("^runtime error: ", ""):gsub("\nstack traceback:.*", ""):gsub("0x%x+", "<ptr>")
  return (e:gsub("\n", " "))
end

return function(rec)
  assert(os.execute("rm -rf " .. ROOT .. " && mkdir -p " .. ROOT), "mkdir")
  assert(os.execute(("cc -shared -fPIC -O1 -w -I%s %s/apiplug.c -o %s/apiplug.so"):format(INC, SRC, ROOT)), "compile apiplug")
  package.cpath = ROOT .. "/?.so;" .. package.cpath
  local p = require "apiplug"

  local names = {}
  for k in pairs(p) do names[#names + 1] = k end
  table.sort(names)
  rec("exports", #names .. " " .. table.concat(names, ","))

  for _, name in ipairs({
    "stack", "xmove", "types", "convert", "userdata", "arith", "compare", "concat", "len", "pushes",
    "tables", "globals", "next", "metatables", "calls", "load_dump", "dump", "loadfuncs", "coroutine",
    "thread_error", "toclose", "warning", "atpanic", "gc", "alloc", "version", "debug_info", "upvalues",
    "hooks", "locals", "metafield", "tolstring", "udata", "refs", "strfuncs", "buffer", "setfuncs",
    "requiref", "traceback", "results", "newstate", "openlibs",
  }) do
    rec(name, try(p[name]))
  end

  -- lua_pushcclosure: upvalues are visible to the C function.
  rec("closure", try(p.closure()))
  rec("closure result", select("#", p.closure()()) .. " " .. table.concat({ p.closure()() }, ","))

  -- lua_error with a non-string value, and luaL_error text.
  rec("error string", err(p.error))
  rec("error table", (function() local ok, e = pcall(p.error_table) return tostring(ok) .. " " .. type(e) .. " " .. tostring(e.code) end)())

  -- luaL_check* / luaL_opt* / luaL_arg* errors, driven from Lua.
  local function arg(n, ...) return try(p.argcheck, n, ...) end
  rec("checklstring ok", arg(1, "s"))
  rec("checklstring number", arg(1, 5))
  rec("checklstring table", err(p.argcheck, 1, {}))
  rec("optlstring nil", arg(2))
  rec("optlstring set", arg(2, "given"))
  rec("checknumber ok", arg(3, "2.5"))
  rec("checknumber bad", err(p.argcheck, 3, "x"))
  rec("optnumber nil", arg(4))
  rec("checkinteger ok", arg(5, 4.0))
  rec("checkinteger float", err(p.argcheck, 5, 4.5))
  rec("checkinteger string", err(p.argcheck, 5, "zz"))
  rec("optinteger nil", arg(6))
  rec("checktype ok", arg(7, {}))
  rec("checktype bad", err(p.argcheck, 7, 1))
  rec("checkany none", err(p.argcheck, 8))
  rec("checkany nil", arg(8, nil))
  rec("argerror", err(p.argcheck, 9, 1))
  rec("typeerror", err(p.argcheck, 10, 1))
  rec("typeerror nil", err(p.argcheck, 10))
  rec("checkoption default", arg(11))
  rec("checkoption set", arg(11, "three"))
  rec("checkoption bad", err(p.argcheck, 11, "four"))
  rec("checkstack overflow", err(p.argcheck, 12))
  -- luaL_where(L, 1) is the function that called the plugin function, so call it from a Lua chunk.
  local lua_caller = load("local argcheck = ...\nreturn function(n)\n  return argcheck(n)\nend", "=wherechunk")(p.argcheck)
  rec("where", lua_caller(13))
  rec("where (C caller)", arg(13))
  rec("luaL_error", err(p.argcheck, 14))
  rec("checkudata wrong", err(p.argcheck, 15, io.stdout))
  rec("not integer first arg", err(p.argcheck, "x"))

  -- Library surface of the Lua the plugin runs in: plugins written against 5.4 must not be
  -- able to depend on compatibility functions the shipped build does not have.
  local compat = {}
  for _, k in ipairs { "pow", "cosh", "sinh", "tanh", "frexp", "ldexp", "log10" } do compat[#compat + 1] = k .. "=" .. type(math[k]) end
  rec("math compat functions", table.concat(compat, " "))
  local mt = { __lt = function() return true end }
  rec("__le falls back to __lt", tostring((pcall(function() return setmetatable({}, mt) <= setmetatable({}, mt) end))))
  rec("lua version", _VERSION .. " " .. math.type(1) .. " " .. tostring(utf8.charpattern ~= nil))
  rec("integer overflow wraps", tostring(math.maxinteger + 1 == math.mininteger))
  rec("string to number coercion", tostring("10" + 5) .. " " .. tostring("0x10" + 0) .. " " .. tostring(10 // 3))
  rec("goto and integer division", select(2, pcall(load, "goto l1; ::l1:: return 7 // 2"))() and "ok")

  collectgarbage()
end
