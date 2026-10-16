-- W5 contract suite, part 5: the custom-event registry (src/custom_events.c).
--
-- Reached through lxl_test (test_hooks): register a name with a tracing/silent/absent callback,
-- push an event of it, and watch what poll_event hands to Lua. The registry's observable
-- behaviour is: unique names, a type number per name (never reused), overriding a callback
-- keeps the type, '#'-names and empty names are refused, and an unknown name cannot be pushed.

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

local function poll()
  local r = try(system.poll_event)
  for _ = 1, 4 do
    if r ~= "0" then break end
    system.sleep(0.02)
    r = try(system.poll_event)
  end
  return r
end

local function drain()
  for _ = 1, 100 do if try(system.poll_event) == "0" then break end end
end

return function(rec)
  assert(lxl_test and lxl_test.custom_register, "needs the test_hooks cargo feature")
  local reg, push, ctype, cname, has_cb = lxl_test.custom_register, lxl_test.custom_push, lxl_test.custom_type, lxl_test.custom_name, lxl_test.custom_has_callback
  drain()

  -- What the editor registers at startup, in order. The values are offsets from the first
  -- user event type, so the baseline does not depend on SDL's numbering.
  rec("dialogfinished type", tostring(ctype("dialogfinished")))
  rec("dirmonitor type", tostring(ctype("dirmonitor")))
  rec("type 0 name", tostring(cname(0)))
  rec("type 1 name", tostring(cname(1)))
  rec("type 2 name (unused)", tostring(cname(2)))
  rec("type 500 name (unused)", tostring(cname(500)))
  rec("dialogfinished has callback", tostring(has_cb("dialogfinished")))
  rec("dirmonitor has callback (noop)", tostring(has_cb("dirmonitor")))
  rec("unknown has callback", tostring(has_cb("nothing")))

  -- Registration ------------------------------------------------------------------------------
  rec("register new/tracer", tostring(reg("t_one", "tracer")))
  local t1 = ctype("t_one")
  rec("t_one is next type", tostring(t1))
  rec("register second", tostring(reg("t_two", "silent")))
  rec("t_two follows", tostring(ctype("t_two")))
  rec("register no callback", tostring(reg("t_three", "none")))
  rec("t_three follows", tostring(ctype("t_three")))
  rec("name round trip", tostring(cname(ctype("t_two"))))
  rec("has callback: tracer", tostring(has_cb("t_one")))
  rec("has callback: none (noop)", tostring(has_cb("t_three")))

  -- Re-registering overrides the callback and keeps the type -------------------------------------------
  rec("re-register same name", tostring(reg("t_one", "silent")))
  rec("type unchanged", tostring(ctype("t_one") == t1))
  rec("no extra type consumed", tostring(ctype("t_four")))
  -- Delivery must follow the override, and the *same* type number must still reach it. A
  -- registry that ignored the override, or minted a second type, would deliver to the old
  -- callback or to a type nobody polls.
  drain()
  rec("push after override to silent", tostring(push("t_one", 11, 111)))
  rec("poll: silent callback swallows it", poll())
  rec("override to tracer again", tostring(reg("t_one", "tracer")))
  rec("override keeps type", tostring(ctype("t_one") == t1))
  rec("next registration right after", tostring(reg("t_five", "tracer")) .. ":" .. tostring(ctype("t_five") - t1))
  rec("push after override to tracer", tostring(push("t_one", 12, 112)))
  rec("poll: tracer callback delivers it", poll())
  rec("override to no callback", tostring(reg("t_one", "none")))
  rec("push after override to none", tostring(push("t_one", 13, 113)))
  rec("poll: noop swallows it", poll())
  reg("t_one", "tracer")

  -- Refusals --------------------------------------------------------------------------------------------
  rec("register empty", tostring(reg("", "tracer")))
  rec("register '#'", tostring(reg("#", "tracer")))
  rec("register '#name'", tostring(reg("#t_one", "tracer")))
  rec("register '#1' (a type key)", tostring(reg("#8", "tracer")))
  rec("'#'-name not stored", tostring(ctype("#t_one")))
  rec("register with space", tostring(reg("has space", "tracer")))
  rec("register unicode", tostring(reg("événement", "tracer")))
  rec("register long name", tostring(reg(string.rep("n", 300), "tracer")))
  rec("long name is distinct", tostring(has_cb(string.rep("n", 300))))
  rec("name with trailing char differs", tostring(reg("t_one ", "tracer") and ctype("t_one ") ~= ctype("t_one")))
  rec("case sensitive", tostring(reg("T_ONE", "tracer") and ctype("T_ONE") ~= ctype("t_one")))
  rec("prefix is not a match", tostring(has_cb("t_on")))

  -- Pushing and delivery to Lua -------------------------------------------------------------------------------
  drain()
  rec("push tracer", tostring(push("t_one", 7, 99)))
  rec("poll tracer", poll())
  rec("push tracer negative code", tostring(push("t_one", -5, -1)))
  rec("poll tracer negative", poll())
  rec("push tracer zero", tostring(push("t_one", 0, 0)))
  rec("poll tracer zero", poll())
  rec("push silent", tostring(push("t_two", 1, 1)))
  rec("poll silent (swallowed)", poll())
  rec("push no-callback", tostring(push("t_three", 1, 1)))
  rec("poll no-callback (swallowed)", poll())
  rec("push dirmonitor (swallowed)", tostring(push("dirmonitor", 0, 0)))
  rec("poll dirmonitor", poll())

  -- Order is preserved, and a swallowed event does not stop later ones -------------------------------
  push("t_one", 1, 10); push("t_two", 2, 20); push("t_one", 3, 30); push("t_three", 4, 40); push("t_one", 5, 50)
  local got = {}
  for _ = 1, 6 do
    local r = poll()
    if r == "0" then break end
    got[#got + 1] = r
  end
  rec("ordered delivery", table.concat(got, " | "))

  -- Pushing something that cannot be pushed -------------------------------------------------------------
  rec("push unknown", tostring(push("never_registered", 0, 0)))
  rec("push empty name", tostring(push("", 0, 0)))
  rec("push '#8'", tostring(push("#8", 0, 0)))
  rec("push '#'", tostring(push("#", 0, 0)))
  rec("push after refusal still works", tostring(push("t_one", 9, 9)))
  rec("poll after refusal", poll())

  -- Many registrations: each consumes one SDL event type -----------------------------------------------------
  local made, last = 0, nil
  for i = 1, 40 do
    if reg("bulk_" .. i, "tracer") then made = made + 1 end
  end
  rec("40 bulk registrations", tostring(made))
  local sequential = true
  local prev = ctype("bulk_1")
  for i = 2, 40 do
    local t = ctype("bulk_" .. i)
    if t ~= prev + 1 then sequential = false end
    prev = t
  end
  rec("bulk types are consecutive", tostring(sequential))
  push("bulk_40", 40, 400)
  rec("last bulk event delivered", poll())
  rec("bulk_1 name round trip", tostring(cname(ctype("bulk_1"))))
  drain()
end
