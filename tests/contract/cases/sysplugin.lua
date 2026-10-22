-- W5 contract suite, part 4: native plugin loading (`system.load_native_plugin`, the XL symbol
-- resolver, the NativePlugin metatable) and the file dialogs' error path.
--
-- The plugins in tests/contract/plugin/ are compiled here with the system C compiler, against
-- the shipped lite_xl_plugin_api.h and without linking Lua, exactly as a plugin author would.
--
-- DIALOGS: no test here ever lets a real dialog open. The environment is pointed at a dead
-- D-Bus socket. Native portal errors are checked before normalizing their backend-specific text;
-- the historical SDL runner instead rejects its empty SDL_FILE_DIALOG_DRIVER.

local ROOT = "/tmp/lxl_sysplugin_contract"
local SRC = os.getenv("LXL_PLUGIN_SRC") or "tests/contract/plugin"
local INC = os.getenv("LXL_PLUGIN_INC") or "resources/include"

local function sh(cmd) return os.execute(cmd) end

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  if not r[1] then return "ERR" end
  local out = { tostring(r.n - 1) }
  for i = 2, r.n do
    local v = r[i]
    out[#out + 1] = type(v) == "number" and (math.type(v) .. ":" .. tostring(v)) or tostring(v)
  end
  return table.concat(out, ",")
end

-- A dialog result with the driver-list wording of SDL's error dropped.
local function nodriver(msg)
  msg = tostring(msg)
  if msg:match("^dialogfinished,%-?%d+,error,Portal file chooser: .+") then
    assert(msg:match("ZBus Error:.*Failed to connect to address"), "not the expected unavailable-portal failure: " .. msg)
    return (msg:gsub(",error,Portal file chooser: .*", ",error,File dialog driver unsupported (...)"))
  end
  return (msg:gsub("%(supported values.-%)", "(...)"))
end

-- An error message with machine-specific text (paths, pointers) reduced to its shape.
local function shape(msg)
  msg = tostring(msg):gsub("^runtime error: ", ""):gsub("\nstack traceback:.*", "")
  msg = msg:gsub(ROOT, "<ROOT>"):gsub("0x%x+", "<ptr>"):gsub("^[^:\n]*:%d+: ", "")
  return (msg:gsub("\n", " "))
end

return function(rec)
  assert(sh("rm -rf " .. ROOT .. " && mkdir -p " .. ROOT), "mkdir")
  local function build(name, extra)
    local ok = sh(("cc -shared -fPIC -O1 -w %s -I%s %s/%s.c -o %s/%s.so"):format(extra or "", INC, SRC, name, ROOT, name))
    assert(ok, "cannot compile " .. name)
  end
  build("contractplug"); build("zeroplug"); build("plainplug"); build("emptyplug"); build("closeplug")

  rec("native_plugins table", type(package.native_plugins))
  rec("load_native_plugin type", type(system.load_native_plugin))
  rec("searcher installed", tostring(#package.searchers >= 3))

  -- A plugin written against the XL resolver ----------------------------------------------------
  package.cpath = ROOT .. "/?.so;" .. package.cpath
  local ok, plug = pcall(require, "contractplug")
  rec("require contractplug", tostring(ok) .. ":" .. type(plug))
  assert(ok, "contractplug did not load: " .. tostring(plug))
  rec("native_plugins entry", type(package.native_plugins.contractplug))
  rec("native_plugins handle type", type(package.native_plugins.contractplug.handle))
  rec("native_plugins metatable", tostring(getmetatable(package.native_plugins.contractplug) ~= nil))
  rec("native_plugins mt __gc", type((getmetatable(package.native_plugins.contractplug) or {}).__gc))
  rec("native_plugins mt __name", tostring((getmetatable(package.native_plugins.contractplug) or {}).__name))

  local keys = {}
  for k, v in pairs(plug) do keys[#keys + 1] = k .. ":" .. type(v) end
  table.sort(keys)
  rec("plugin exports", table.concat(keys, " "))

  rec("add", try(plug.add, 2, 40))
  rec("add negative", try(plug.add, -5, 3))
  rec("add large", try(plug.add, math.maxinteger, 1))
  rec("add strings", try(plug.add, "7", "8"))
  rec("add bad arg", try(plug.add, "x", 1))
  rec("add missing arg", try(plug.add, 1))
  rec("join", try(plug.join, 1, "two", 3.5, nil, true))
  rec("join empty", try(plug.join))
  rec("table", (function()
    local t = plug.table()
    return table.concat({ t[1], t[2], t[3], tostring(t.k), tostring(#t) }, ",")
  end)())
  rec("call ok", try(plug.call, function(x) return x * 2 end, 21))
  rec("call lua error", (function()
    local r = table.pack(pcall(plug.call, function() error("lua boom", 0) end, 1))
    return tostring(r[1]) .. ":" .. tostring(r[2]) .. ":" .. shape(r[3])
  end)())
  rec("call not a function", try(plug.call, 5, 1))
  rec("fail", (function() local ok, e = pcall(plug.fail) return tostring(ok) .. ":" .. shape(e) end)())
  rec("lua_version", try(plug.version))
  rec("lua_typename", try(plug.typename, 1) .. " " .. try(plug.typename, "s") .. " " .. try(plug.typename, {}) .. " " .. try(plug.typename))

  -- Userdata finalizers run through the plugin's __gc ---------------------------------------------
  local before = plug.gc_count()
  do
    for _ = 1, 25 do local t = plug.userdata() end
  end
  collectgarbage("collect"); collectgarbage("collect")
  rec("userdata __gc ran", tostring(plug.gc_count() - before))

  -- Which symbols does the resolver know? The plugin asks for each by name. The C table is a
  -- hand-kept copy that lacks `lua_closethread` (a 5.4.6 name in the header), so a plugin that
  -- calls it hits the header's exit(1) stub. Rust resolves it; the suite records that one name
  -- as "n/a" so the baseline stays the C behaviour for everything else.
  local cp = require "closeplug"
  local names = {}
  for name in pairs(cp) do if name ~= "lua_closethread" then names[#names + 1] = name end end
  table.sort(names)
  local resolved = {}
  for _, name in ipairs(names) do resolved[#resolved + 1] = (name == "" and "<empty>" or name) .. "=" .. tostring(cp[name]) end
  rec("resolver", table.concat(resolved, " "))
  rec("resolver knows lua_closethread (C: false, Rust: true; see PLAN.md)", "n/a")

  -- Fallback entrypoint, and the three failure modes --------------------------------------------
  -- The plain entry returns 1 without pushing anything, so Lua sees one value of whatever
  -- is on the stack: the call succeeds, which is all the fallback path promises.
  local pok = pcall(system.load_native_plugin, "plainplug", ROOT .. "/plainplug.so")
  rec("plain luaopen_ fallback", tostring(pok))
  rec("plain plugin registered", type(package.native_plugins.plainplug))
  local zok, zerr = pcall(require, "zeroplug")
  rec("entrypoint returns 0", tostring(zok) .. ":" .. shape(zerr):match("entrypoint must return a value") )
  local eok, eerr = pcall(require, "emptyplug")
  rec("no entrypoint", tostring(eok) .. ":" .. (shape(eerr):match("Can't find luaopen_emptyplug") or shape(eerr)))
  rec("missing file", (function()
    local ok, e = pcall(system.load_native_plugin, "nope", ROOT .. "/missing.so")
    return tostring(ok) .. ":" .. (type(e) == "string" and (#e > 0 and "message" or "empty") or type(e))
  end)())
  rec("load_native_plugin no args", try(system.load_native_plugin))
  rec("load_native_plugin one arg", try(system.load_native_plugin, "x"))
  rec("load_native_plugin tables", try(system.load_native_plugin, {}, {}))

  -- Dotted names use the part after the last dot as the entrypoint -----------------------------------
  sh(("cp %s/contractplug.so %s/dotted.so"):format(ROOT, ROOT))
  local dok, dres = pcall(system.load_native_plugin, "some.dotted.contractplug", ROOT .. "/dotted.so")
  rec("dotted name", tostring(dok) .. ":" .. type(dres))
  rec("dotted name registered", type(package.native_plugins["some.dotted.contractplug"]))

  -- Loading the same library twice is allowed and gives independent state ---------------------------
  sh(("cp %s/contractplug.so %s/contractplug2.so"):format(ROOT, ROOT))
  -- A different file, so the dynamic loader gives a second, independent copy of the plugin
  -- (its gc counter is separate), registered under the entrypoint name "contractplug".
  local aok, again = pcall(system.load_native_plugin, "other.contractplug", ROOT .. "/contractplug2.so")
  rec("second load", tostring(aok) .. ":" .. type(again))
  rec("second load add", try(again.add, 1, 2))
  rec("second copy has its own counter", tostring(again.gc_count() == 0 and plug.gc_count() > 0))

  -- Dialogs: only the error path, never a real window ---------------------------------------------
  -- (driven by verify.sh's dead DBUS address; see the header comment)
  local win = renwindow.create("dlg", 200, 150)
  system.sleep(0.1)
  for _ = 1, 100 do if not system.poll_event() then break end end

  local function await(id)
    local t0 = system.get_time()
    while system.get_time() - t0 < 5 do
      local ev = table.pack(system.poll_event())
      -- Only this dialog's own event: a late event from an earlier dialog is dropped, so one
      -- stray result cannot shift every following observation by a row.
      if ev.n > 0 and ev[1] == "dialogfinished" and ev[2] == id then
        local parts = {}
        for i = 1, ev.n do
          local v = ev[i]
          parts[i] = type(v) == "table" and ("{" .. table.concat(v, ",") .. "}") or tostring(v)
        end
        return table.concat(parts, ",")
      end
      system.sleep(0.01)
    end
    return "<no event>"
  end

  rec("open_file_dialog returns", try(system.open_file_dialog, win, 11))
  rec("open_file_dialog event", nodriver(await(11)))
  rec("save_file_dialog returns", try(system.save_file_dialog, win, 12, { title = "t", default_location = "/tmp", accept_label = "ok", cancel_label = "no" }))
  rec("save_file_dialog event", nodriver(await(12)))
  rec("open_directory_dialog returns", try(system.open_directory_dialog, win, 13, { allow_many = true }))
  rec("open_directory_dialog event", nodriver(await(13)))
  rec("dialog with filters", try(system.open_file_dialog, win, 14, {
    filters = { { name = "Lua", pattern = "lua" }, { name = "All", pattern = "*" } }, allow_many = true }))
  rec("dialog with filters event", nodriver(await(14)))
  rec("dialog id is arbitrary", try(system.open_file_dialog, win, 123456789))
  rec("dialog id event", nodriver(await(123456789)))

  rec("dialog bad window", try(system.open_file_dialog, {}, 1))
  rec("dialog no window", try(system.open_file_dialog))
  rec("dialog no id", try(system.open_file_dialog, win))
  rec("dialog id not a number", try(system.open_file_dialog, win, "x"))
  rec("dialog options not a table", try(system.open_file_dialog, win, 15, 5))
  rec("dialog filters not tables", try(system.open_file_dialog, win, 16, { filters = { "x" } }))
  rec("dialog filter missing pattern", try(system.open_file_dialog, win, 17, { filters = { { name = "a" } } }))
  rec("dialog filter name not string", try(system.open_file_dialog, win, 18, { filters = { { name = {}, pattern = "x" } } }))
  rec("dialog title not string", try(system.open_file_dialog, win, 19, { title = {} }))
  rec("dialog empty options", try(system.open_file_dialog, win, 20, {}))
  rec("dialog empty options event", nodriver(await(20)))
  -- The success path needs a real dialog, so it is driven by forging the event a finished
  -- dialog produces (test_hooks). This checks the C-to-Lua packing, not SDL.
  local function forged(spec)
    spec.type = "dialog_finished"
    lxl_test.push_event(spec)
    return nodriver(await(spec.id))
  end
  if lxl_test then
    rec("forged accept, 1 path", forged { id = 31, code = 0, paths = { "/tmp/a b.txt" } })
    rec("forged accept, 3 paths", forged { id = 32, code = 0, paths = { "/x/1", "/x/2", "/x/3" } })
    rec("forged accept, 0 paths", forged { id = 33, code = 0, paths = {} })
    rec("forged accept, unicode", forged { id = 34, code = 0, paths = { "/tmp/日本語.txt", "/tmp/é" } })
    rec("forged accept, long path", (select(1, (forged { id = 35, code = 0, paths = { "/" .. string.rep("#", 3000) } }):gsub("#+", function(run) return "<" .. #run .. " bytes>" end, 1))))
    rec("forged cancel", forged { id = 36, code = 1 })
    rec("forged error", forged { id = 37, code = 2, text = "something broke" })
    rec("forged error, empty", forged { id = 38, code = 2, text = "" })
    rec("forged unknown code", forged { id = 39, code = 9 })
    rec("forged id 0", forged { id = 0, code = 1 })
    rec("forged id large", forged { id = 2 ^ 40, code = 1 })
  end

  -- Settle every dialog started above so nothing is left pending at exit.
  local t0 = system.get_time()
  while system.get_time() - t0 < 1 do system.poll_event(); system.sleep(0.01) end

  sh("rm -rf " .. ROOT)
end
