-- Fake portal on a private bus, real production request/parent/queue path, no hook dependency.
local M = {}
local work = assert(os.getenv("DIALOG_CHECK_WORK"))
local role = assert(os.getenv("DIALOG_CHECK_ROLE"))
local function exists(name) local f = io.open(work .. "/" .. name); if f then f:close(); return true end end
local function mark(name) local f = assert(io.open(work .. "/" .. name, "w")); f:write("ready"); f:close() end
local function tick() system.wait_event(0.01) end
local function until_file(name)
  local deadline = system.get_time() + 8
  while not exists(name) do assert(system.get_time() < deadline, "missing barrier " .. name); tick() end
end
local function await(id)
  local deadline = system.get_time() + 8
  while system.get_time() < deadline do
    tick()
    while true do
      local e = table.pack(system.poll_event())
      if e.n == 0 then break end
      if e[1] == "dialogfinished" then assert(e[2] == id, "wrong caller id"); return e end
    end
  end
  error("dialog completion timeout")
end
local function create()
  local w = renwindow.create("dialog-check", 240, 140)
  renderer.begin_frame(w); renderer.draw_rect(0, 0, 240, 140, {32, 32, 32}); renderer.end_frame()
  return w
end
local function accept(e, path) assert(e.n == 4 and e[3] == "accept" and #e[4] == 1 and e[4][1] == path, "bad accepted paths") end

function M.init()
  if role == "restart" then
    local w = create()
    if not RESTARTED then
      system.open_file_dialog(w, 777, {title = "restart"})
      until_file("received-restart")
      M.restart_request = true
      return
    end
    system.open_file_dialog(w, 777, {title = "new"}) -- same caller id, new generation
    until_file("sent-restart")
    for _ = 1, 20 do
      tick()
      while true do
        local kind = system.poll_event()
        if not kind then break end
        assert(kind ~= "dialogfinished", "old generation completion leaked")
      end
    end
    mark("old-filtered")
    accept(await(777), "/tmp/new")
  elseif role == "exit" then
    local w = create()
    system.open_file_dialog(w, 99, {title = "exit"})
    until_file("received-exit")
    print("DIALOG exit passed")
    os.exit(0) -- chooser remains unanswered; delivery closes before Lua is destroyed
  else
    assert(role == "normal")
    local w = create()
    for _, title in ipairs {"accept", "cancel1", "cancel2", "empty", "invalid", "nonutf8", "transport"} do
      assert(select('#', system.open_file_dialog(w, 1, {title = title})) == 0)
      local e = await(1)
      if title == "accept" then accept(e, "/tmp/accept")
      elseif title == "nonutf8" then accept(e, "/tmp/\255 #")
      elseif title == "invalid" or title == "transport" then assert(e.n == 4 and e[3] == "error" and type(e[4]) == "string" and #e[4] > 0)
      else assert(e.n == 3 and e[3] == "cancel") end
    end
    -- Metamethods are evaluated once, before any Rust-owned options exist.
    local reads = {}
    local opts = setmetatable({}, {__index = function(_, field)
      reads[field] = (reads[field] or 0) + 1
      return ({title = "options", default_location = "/tmp/\255", allow_many = true, accept_label = "Choose", cancel_label = "ignored",
        filters = {{name = "Images", pattern = "png;jpg"}, {name = "All", pattern = "*"}}})[field]
    end})
    system.open_file_dialog(w, 1, opts); accept(await(1), "/tmp/options")
    for _, n in pairs(reads) do assert(n == 1, "option metamethod evaluated twice") end
    system.open_directory_dialog(w, 1, {title = "directory", allow_many = true}); accept(await(1), "/tmp/directory")
    system.save_file_dialog(w, 1, {default_location = work .. "/new.txt"}); accept(await(1), "/tmp/Save File")
    local path = work .. "/\255"
    local file = assert(io.open(path, "w")); file:close()
    system.save_file_dialog(w, 1, {title = "savebytes", default_location = path}); accept(await(1), "/tmp/savebytes")
    assert(os.remove(path))
    system.open_file_dialog(w, -5, {title = "accept\0ignored"}); accept(await(-5), "/tmp/accept")
    system.open_file_dialog(w, 0, {title = "\255"}); assert(await(0)[3] == "error")
    for _, bad in ipairs {"", "png;", "*.png"} do
      system.open_file_dialog(w, 0, {filters = {{name = "Bad", pattern = bad}}}); assert(await(0)[3] == "error")
    end
    system.open_file_dialog(w, 44, {title = "dup1"})
    system.open_file_dialog(w, 44, {title = "dup2"})
    local a, b = await(44), await(44)
    assert(a[3] == "accept" and b[3] == "accept" and a[4][1] ~= b[4][1], "duplicate caller ids were conflated")
    system.open_file_dialog(w, 55, {title = "held"})
    until_file("parent-visible")
    w = nil; collectgarbage("collect"); collectgarbage("collect")
    mark("parent-closed")
    accept(await(55), "/tmp/held") -- logical parent close must not cancel/discard
    for _ = 1, 10 do tick(); while system.poll_event() do end end
    mark("parent-released")
    until_file("parent-gone")
  end
  print("DIALOG " .. role .. " passed")
end
function M.run() end
function M.on_error(err) io.stderr:write(tostring(err), "\n"); os.exit(1) end
return M
