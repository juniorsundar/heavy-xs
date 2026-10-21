-- system.load_native_plugin beyond the `sysplugin` suite: each failure's exact message, the
-- underscore-prefixed entrypoint fallback, and handle lifetime (one close per loaded copy,
-- observed through /proc/self/maps). The golden was recorded from the pinned SDL reference
-- (scripts/rust-port/sdl-reference.sh) before loading moved off SDL, so the wording stays SDL's.

local ROOT = "/tmp/lxl_sysloader_contract"
local SRC = os.getenv("LXL_PLUGIN_SRC") or "tests/contract/plugin"
local INC = os.getenv("LXL_PLUGIN_INC") or "resources/include"

local function sh(cmd) return os.execute(cmd) end

local function load(name, path)
  local r = table.pack(pcall(system.load_native_plugin, name, path))
  local out = { tostring(r[1]) }
  for i = 2, r.n do out[#out + 1] = (tostring(r[i]):gsub(ROOT, "<ROOT>")) end
  return table.concat(out, "|")
end

local function mapped(file)
  local n = 0
  for line in io.lines("/proc/self/maps") do
    if line:sub(-#file - 1) == "/" .. file then n = n + 1 end
  end
  return n > 0
end

local function collect() collectgarbage("collect"); collectgarbage("collect") end

return function(rec)
  assert(sh("rm -rf " .. ROOT .. " && mkdir -p " .. ROOT), "mkdir")
  for _, name in ipairs { "plainplug", "underplug", "undefplug", "emptyplug", "provplug", "consplug" } do
    assert(sh(("cc -shared -fPIC -O1 -w -I%s %s/%s.c -o %s/%s.so"):format(INC, SRC, name, ROOT, name)), name)
  end
  local f = assert(io.open(ROOT .. "/notelf.so", "wb")); f:write("not a shared object\n"); f:close()

  rec("missing file", load("nope", ROOT .. "/missing.so"))
  rec("not an ELF file", load("notelf", ROOT .. "/notelf.so"))
  rec("directory", load("dir", ROOT))
  rec("empty path", load("empty", ""))
  rec("undefined symbol", load("undefplug", ROOT .. "/undefplug.so"))
  rec("undefined symbol not registered", type(package.native_plugins.undefplug))
  rec("no entrypoint", load("emptyplug", ROOT .. "/emptyplug.so"))
  rec("no entrypoint still registered", type(package.native_plugins.emptyplug))
  rec("underscore entrypoint", load("underplug", ROOT .. "/underplug.so"))
  rec("NUL in path truncates", load("nul.plainplug", ROOT .. "/plainplug.so\0junk"))
  rec("number path", load("num", 5))
  local nb = table.pack(pcall(system.load_native_plugin, "nb", ROOT .. "/\xff\xfe.so"))
  rec("missing non-UTF-8 path, message bytes", tostring(nb[1]) .. "|" .. (tostring(nb[2]):gsub(ROOT, "<ROOT>"):gsub("[\x80-\xff]", function(c) return ("<%02x>"):format(c:byte()) end)))
  rec("provider", load("provplug", ROOT .. "/provplug.so"))
  rec("consumer sees provider (scope)", load("consplug", ROOT .. "/consplug.so"))

  -- Lifetime: each load owns one reference; __gc releases it exactly once.
  package.native_plugins["nul.plainplug"] = nil
  collect()
  rec("unmapped after its owner is collected", tostring(not mapped("plainplug.so")))
  rec("load a", load("a.plainplug", ROOT .. "/plainplug.so"))
  rec("load b, same file", load("b.plainplug", ROOT .. "/plainplug.so"))
  package.native_plugins["a.plainplug"] = nil
  collect()
  rec("still mapped while b owns it", tostring(mapped("plainplug.so")))
  package.native_plugins["b.plainplug"] = nil
  collect()
  rec("unmapped after both are collected", tostring(not mapped("plainplug.so")))
  rec("reload after unload", load("c.plainplug", ROOT .. "/plainplug.so"))
  rec("handle is light userdata", type(package.native_plugins["c.plainplug"].handle))
  package.native_plugins["c.plainplug"] = nil
end
