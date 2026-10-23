-- Disposable user init: normal core.init, plugin discovery and frame loop still run.
local core = require "core"
local config = require "core.config"
local command = require "core.command"
local View = require "core.view"
local DocView = require "core.docview"
local ime = require "core.ime"
local base_update, base_draw = DocView.update, DocView.draw_line_text
local selected = {}
for id in (os.getenv("GUI_REFERENCE_PLUGINS") or ""):gmatch("[^,]+") do selected[id] = true end
local errors = 0
local function emit(kind, name, detail)
  io.stdout:write("GUIREF\t", kind, "\t", name, "\t", tostring(detail or ""):gsub("[\t\r\n]+", " "), "\n")
  io.stdout:flush()
end
local log_error = core.error
core.error = function(...)
  errors = errors + 1
  emit("ERROR", "core.error", string.format(...))
  return log_error(...)
end
local function read(path)
  local f = assert(io.open(path, "rb")); local s = f:read("*a"); f:close(); return s
end
local function chain(fn, target, seen)
  if fn == target then return {"docview.lua"} end
  if seen[fn] then return end
  seen[fn] = true
  for i = 1, 32 do
    local name, value = debug.getupvalue(fn, i)
    if not name then break end
    if type(value) == "function" then
      local path = chain(value, target, seen)
      if path then
        table.insert(path, 1, debug.getinfo(fn, "S").source:match("[^/]+$"))
        return path
      end
    end
  end
end
local Probe = View:extend()
function Probe:new() Probe.super.new(self); self.updates = 0; self.draws = 0 end
function Probe:update() self.updates = self.updates + 1 end
function Probe:draw()
  self.draws = self.draws + 1
  self.clip = {table.unpack(core.clip_rect_stack[#core.clip_rect_stack])}
  self:draw_background()
end
local fixture, leaf, tab, panel, panel_node, deferred, scratch, sv, clock
local previous_update, previous_draw, previous_caret, old_blink, old_shape, stats, prompt_cancelled
local menu_invoked = false
local frames, phase = 0, 1
local phases = {}
phases[1] = function()
  assert(rawget(_G, "lxl_test") == nil, "test hooks unexpectedly present")
  local w, h = core.window:get_size(); assert(w == 1024 and h == 640)
  fixture = core.active_view; assert(fixture:extends(DocView))
  emit("READY", "focus", "private_x11_driver_required")
  local entries, order = {}, {}
  for _, entry in ipairs(core.plugin_list) do
    entries[entry.name] = entry
    if selected[entry.name] then order[#order+1] = entry.name end
  end
  for id in pairs(selected) do
    assert(entries[id] and entries[id].version_match, "version gate: " .. id)
    assert(package.loaded["plugins." .. id], "not loaded: " .. id)
  end
  emit("PASS", "normal_loader", table.concat(order, ","))
  if selected.custom_caret or selected.smoothcaret then
    local source = debug.getinfo(DocView.draw_caret, "S").source:match("[^/]+$")
    assert(source == (selected.smoothcaret and "smoothcaret.lua" or "custom_caret.lua"))
    if selected.custom_caret and selected.smoothcaret then
      local found = false
      for i = 1, 32 do
        local name, value = debug.getupvalue(DocView.draw_caret, i)
        if not name then break end
        if name == "docview_draw_caret" then
          found = debug.getinfo(value, "S").source:match("[^/]+$") == "custom_caret.lua"
        end
      end
      assert(found, "smoothcaret did not capture custom_caret")
    end
    emit("PASS", "caret_implementation", source)
  end
  for _, pair in ipairs({{DocView.update, base_update, "update"}, {DocView.draw_line_text, base_draw, "draw"}}) do
    local path = assert(chain(pair[1], pair[2], {}), "wrapper base unreachable")
    local joined = table.concat(path, ">")
    for _, id in ipairs({"indentguide", "bracketmatch"}) do
      if selected[id] then assert(joined:find(id .. ".lua", 1, true), "missing wrapper: " .. id) end
    end
    emit("PASS", "wrapper_chain_" .. pair[3], joined)
  end
  leaf = core.root_view:get_active_node()
  tab = Probe(); leaf:add_view(tab)
  assert(tab:is(Probe) and tab:extends(View) and not tab:is(View))
  panel = Probe(); panel_node = leaf:split("right", panel)
  leaf.a:set_active_view(fixture); core.set_active_view(panel)
end
phases[2] = function()
  assert(system.window_has_focus(core.window), "native focus not acquired")
  emit("PASS", "native_x11_focus_acquired")
  assert(tab.updates > 0 and tab.draws == 0, "inactive view lifecycle")
  assert(panel.updates > 0 and panel.draws > 0 and panel.size.x > 0 and panel.size.y > 0)
  for i, n in ipairs({panel.position.x, panel.position.y, panel.size.x, panel.size.y}) do
    assert(math.abs(panel.clip[i] - n) < 0.5, "clip/geometry mismatch")
  end
  assert(core.root_view.root_node:get_child_overlapping_point(panel.position.x+1, panel.position.y+1) == panel_node)
  local x, y = leaf.a:get_tab_rect(1)
  assert(leaf.a:get_tab_overlapping_point(x+1, y+1) == 1)
  panel_node:close_view(core.root_view.root_node, panel)
  leaf:close_view(core.root_view.root_node, tab)
  assert(leaf.type == "leaf" and not core.root_view.root_node:get_node_for_view(panel))
  core.set_active_view(fixture)
  emit("PASS", "view_identity_layout_clip_activation_close", "inactive_update=1;inactive_draw=0")
  deferred = {}
  for i = 1, 3 do core.root_view:defer_draw(function() deferred[#deferred+1] = i end) end
end
phases[3] = function()
  assert(table.concat(deferred, ",") == "1,2,3", "deferred order")
  emit("PASS", "deferred_draw_order", "1,2,3")
  local keymap = require "core.keymap"
  local forwarded, skipped, performed
  command.add(function() return true, fixture end, {["gui-ref:args"] = function(v) forwarded = v end})
  assert(command.perform("gui-ref:args", "ignored") and forwarded == fixture)
  command.add(function() skipped = true; return false end, {["gui-ref:skip"] = function() error("ineligible") end})
  command.add(nil, {["gui-ref:run"] = function() performed = true end})
  local old = keymap.map.f12
  keymap.add({f12 = {"gui-ref:skip", "gui-ref:run"}}, true)
  assert(keymap.on_key_pressed("f12") and skipped and performed)
  keymap.map.f12 = old
  emit("PASS", "command_predicate_identity_keymap_fallthrough")
  scratch = core.open_doc("scratch.lua")
  scratch:insert(1, 1, "local function probe(value)\n  if value then\n    return (value + 1)\n  end\nend\n")
  sv = core.root_view:open_doc(scratch)
  local opening = assert(scratch.lines[1]:find("(", 1, true)); scratch:set_selection(1, opening)
  if selected.bracketmatch then
    assert(command.perform("bracket-match:move-to-matching"))
    local l, c = scratch:get_selection()
    assert(l == 1 and c == scratch.lines[1]:find(")", opening, true), "bracket navigation")
    emit("PASS", "bracketmatch_navigation", "synthetic_command=1")
  end
  previous_update, previous_draw = DocView.update, DocView.draw_line_text
  stats = {updates=0, draws=0, rects=0, carets=0, caret_rects=0}
  old_blink = config.disable_blink; config.disable_blink = true
  if selected.custom_caret then
    old_shape = config.plugins.custom_caret.shape; config.plugins.custom_caret.shape = "block"
  end
  previous_caret = DocView.draw_caret
  DocView.draw_caret = function(self, x, y)
    if self ~= sv then return previous_caret(self, x, y) end
    assert(type(x) == "number" and type(y) == "number")
    stats.carets = stats.carets+1
    local rect = renderer.draw_rect
    renderer.draw_rect = function(...) stats.caret_rects=stats.caret_rects+1; return rect(...) end
    local result = table.pack(pcall(previous_caret, self, x, y))
    renderer.draw_rect = rect; assert(result[1], result[2])
    return table.unpack(result, 2, result.n)
  end
  DocView.update = function(self, ...)
    if self == sv then stats.updates = stats.updates+1; assert(select("#", ...) == 0) end
    return previous_update(self, ...)
  end
  DocView.draw_line_text = function(self, ...)
    if self ~= sv then return previous_draw(self, ...) end
    assert(select("#", ...) == 3)
    local rect = renderer.draw_rect
    renderer.draw_rect = function(...) stats.rects=stats.rects+1; return rect(...) end
    local result = table.pack(pcall(previous_draw, self, ...))
    renderer.draw_rect = rect
    assert(result[1], result[2]); assert(result.n == 2 and type(result[2]) == "number")
    stats.draws = stats.draws+1
    return table.unpack(result, 2, result.n)
  end
end
phases[4] = function()
  DocView.update, DocView.draw_line_text, DocView.draw_caret = previous_update, previous_draw, previous_caret
  config.disable_blink = old_blink
  if selected.custom_caret then config.plugins.custom_caret.shape = old_shape end
  assert(stats.carets > 0 and stats.caret_rects > 0, "focused caret did not draw")
  if selected.smoothcaret then assert(type(sv.carets) == "table" and #sv.visible_carets > 0) end
  emit("PASS", "focused_caret_draw", "calls=" .. stats.carets .. ";rects=" .. stats.caret_rects .. ";block_shape=" .. tostring(selected.custom_caret == true))
  assert(stats.updates > 0 and stats.draws > 0, "DocView not composed")
  if selected.indentguide then assert(type(sv.indentguide_indents) == "table") end
  if selected.indentguide or selected.bracketmatch then assert(stats.rects > 0, "plugin decorations absent") end
  emit("PASS", "docview_update_draw_args_return", "updates=" .. stats.updates .. ";draws=" .. stats.draws .. ";decoration_rects=" .. stats.rects)
  if selected.autosaveonfocuslost then
    core.redraw = false; core.on_event("focuslost")
    assert(core.redraw, "focuslost predecessor not delegated")
    assert(not scratch:is_dirty() and read("/work/project/scratch.lua") == table.concat(scratch.lines))
    emit("PASS", "autosave_focuslost", "synthetic_event=1;scratch_saved=1")
  end
  assert(not fixture.doc:is_dirty(), "fixture modified")
  if selected.scalestatus then
    local item = assert(core.status_view:get_item("status:scale"))
    local scale = require "plugins.scale"
    assert(item:get_item()[1] == string.format(config.plugins.scalestatus.format, scale.get()*100))
    assert(item.tooltip == "scale"); item:hide(); assert(not item.visible); item:show(); assert(item.visible)
    emit("PASS", "scalestatus_structured_item", item:get_item()[1])
  end
  local cv = core.command_view
  cv:set_text("legacy")
  cv:enter("Probe", function() end, function(t) assert(t == "legacy"); return {} end,
    function(explicit) prompt_cancelled = not explicit end, function() return true end)
  assert(core.active_view == cv and cv:get_text() == "legacy")
end
phases[5] = function()
  assert(core.active_view == core.command_view and core.command_view.size.y > 0)
  core.command_view:exit(false, true); assert(prompt_cancelled and core.active_view == sv)
  emit("PASS", "deprecated_prompt_focus_and_draw")
  scratch:set_selection(1, 1); core.on_event("textinput", "Q")
  assert(scratch.lines[1]:sub(1, 1) == "Q")
  core.on_event("textediting", "ab", 0, 2); assert(ime.editing)
  ime.stop(); assert(not ime.editing)
  emit("PASS", "synthetic_text_ime_dispatch", "native_ime=unverified")
  if selected.bigclock then
    assert(command.perform("big-clock:open")); clock = core.active_view
    assert(clock:is(package.loaded["plugins.bigclock"]) and clock:extends(View))
    config.plugins.bigclock.scale = 2
  end
end
phases[6] = function()
  if clock then
    assert(#clock.time_text > 0 and #clock.date_text > 0 and clock.last_scale == 2 and clock.font_size > 0)
    config.plugins.bigclock.scale = 1
  end
end
phases[7] = function()
  if clock then
    assert(clock.last_scale == 1 and clock.font_size > 0)
    core.root_view.root_node:get_node_for_view(clock):close_view(core.root_view.root_node, clock)
    emit("PASS", "bigclock_view_draw_font_scale_close", "scale=2>1")
  end
  core.set_active_view(sv)
  command.add(nil, {["gui-ref:menu"] = function(view) assert(view == fixture); menu_invoked = true end})
  assert(core.root_view.context_menu:show(100, 100, {{text="Reference action",command="gui-ref:menu"}}, fixture))
end
phases[8] = function()
  local menu = core.root_view.context_menu
  assert(menu.visible and menu.height > 0 and core.active_view == menu)
  local x, y = menu.position.x+1, menu.position.y+math.min(menu.height/2, 2)
  menu:on_mouse_moved(x, y)
  assert(core.root_view:on_mouse_pressed("left", x, y, 1))
  assert(menu_invoked and not menu.visible and core.active_view == sv)
  emit("PASS", "context_menu_draw_hit_command_focus", "synthetic_pointer=1;argument_identity=1")
end
phases[9] = function()
  assert(errors == 0, "logged core errors=" .. errors)
  emit("PASS", "normal_frames", frames)
  emit("DONE", "reference", frames)
  core.quit(true)
end
local run = core.run
core.run = function(...)
  local step = core.step
  core.step = function(...)
    local ok, result = xpcall(function(...)
      core.redraw = true
      local drew = step(...)
      if drew then
        frames = frames+1
        if phase == 2 and not system.window_has_focus(core.window) then return drew end
        phases[phase](); phase = phase+1
      end
      return drew
    end, debug.traceback, ...)
    if not ok then emit("FAIL", "phase_" .. phase, result); core.quit(true); return false end
    return result
  end
  return run(...)
end
