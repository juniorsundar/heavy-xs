-- Disposable user init; the normal core and all bundled plugins still run.
local core = require "core"
local config = require "core.config"
local system = system
local rootdir = assert(os.getenv("GUI_BENCH_ROOT"))
local mode = assert(os.getenv("GUI_BENCH_MODE"))
local variant = assert(os.getenv("GUI_BENCH_VARIANT"))
assert(mode == "startup" or mode == "frames" or mode == "idle" or mode == "retention" or mode == "edit")
local function emit(kind, ...)
  local values = {...}
  for i,v in ipairs(values) do values[i] = tostring(v) end
  io.stdout:write("GUIBENCH\t", kind, "\t", table.concat(values,"\t"), "\n")
  io.stdout:flush()
end
if variant == "custom" then
  config.plugins.drawwhitespace.enabled = true
  config.plugins.lineguide.enabled = true
  config.plugins.lineguide.rulers = {80,100,120}
end
local armed, first = false, true
local on_event = core.on_event
core.on_event = function(kind, ...)
  if kind == "keypressed" and select(1,...) == "f12" then
    armed = true
    emit("input_ready", system.get_time(), system.window_has_focus(core.window) and 1 or 0)
  end
  if kind == "focuslost" then emit("focus_lost",system.window_has_focus(core.window) and 1 or 0) end
  return on_event(kind,...)
end
local original_run = core.run
core.run = function()
  local Doc = require "core.doc"
  local DocView = require "core.docview"
  local View = require "core.view"
  local renderer = renderer
  local dv = core.active_view
  if mode=="edit" and RESTARTED and not dv:extends(DocView) then
    dv=core.root_view:open_doc(core.open_doc(assert(os.getenv("GUI_BENCH_FIXTURE"))))
  end
  assert(dv:extends(DocView) and dv.doc)
  if variant == "custom" then
    local Extra = View:extend()
    function Extra:new() Extra.super.new(self); self.scrollable = true end
    function Extra:draw()
      self:draw_background({28,35,45})
      for i=1,80 do
        renderer.draw_rect(self.position.x+8, self.position.y+i*6, self.size.x-16, 2, {80+i,100,160})
      end
    end
    core.root_view:get_active_node():split("right",Extra())
    core.set_active_view(dv)
  end
  local update_ms, draw_ms, finish_ms = 0,0,0
  local update,draw = core.root_view.update,core.root_view.draw
  core.root_view.update = function(self,...)
    local t=system.get_time(); local r=update(self,...)
    update_ms=(system.get_time()-t)*1000; return r
  end
  core.root_view.draw = function(self,...)
    local t=system.get_time(); local r=draw(self,...)
    draw_ms=(system.get_time()-t)*1000; return r
  end
  local finish=renderer.end_frame
  renderer.end_frame=function(...)
    local t=system.get_time(); local r=finish(...)
    finish_ms=(system.get_time()-t)*1000; return r
  end
  local t=system.get_time()
  for _=1,10000 do system.get_time() end
  emit("timer_loop_ms",(system.get_time()-t)*1000)
  local step = core.step
  local phases = {"warmup","type","scroll","layout"}
  local phase, count = 1,0
  local cycles, transient = 0,nil
  local layout_node
  core.step = function(...)
    local label = phases[phase]
    local cycle_finished = false
    local start = system.get_time()
    if armed and mode=="frames" then
      if label=="type" then
        core.on_event("textinput","x")
      elseif label=="scroll" then
        local direction = math.floor(count/20)%2==0 and -3 or 3
        dv:on_mouse_wheel(direction,0)
      elseif label=="layout" then
        if not layout_node then
          layout_node=core.root_view:get_active_node()
          layout_node:split("right",DocView(dv.doc))
          core.set_active_view(dv)
        end
        layout_node.divider=count%2==0 and 0.35 or 0.65
      end
      core.redraw=true
    elseif armed and mode=="retention" then
      local node=core.root_view:get_active_node()
      if not transient then
        local doc=Doc()
        doc:insert(1,1,string.rep("local transient = 42\n",200))
        core.docs[#core.docs+1]=doc
        transient=DocView(doc)
        node:add_view(transient)
      else
        node:remove_view(core.root_view.root_node,transient)
        transient=nil
        cycles=cycles+1
        cycle_finished=true
      end
      core.redraw=true
    end
    local dispatched=system.get_time()
    update_ms,draw_ms,finish_ms=0,0,0
    local drew=step(...)
    local stop=system.get_time()
    if cycle_finished then
      collectgarbage("collect")
      emit("cycle",cycles)
    end
    if drew and first then
      first=false
      local w,h=core.window:get_size()
      assert(w==1024 and h==640,string.format("unexpected size %dx%d",w,h))
      emit(mode=="edit" and RESTARTED and "restarted_frame" or "first_frame",stop,w,h,#core.plugin_list)
      if mode=="edit" and RESTARTED then core.quit(true) end
    end
    if armed and mode=="edit" and not RESTARTED then
      emit("edit_start",system.get_time())
      local doc=core.open_doc(USERDIR .. PATHSEP .. "init.lua")
      doc:insert(1,1,'local edited_style = require "core.style"; edited_style.caret_width = edited_style.caret_width + 1\n')
      doc:save() -- bundled autorestart calls core.restart for this saved user init
    elseif armed and mode=="startup" then
      core.quit(true)
    elseif armed and mode=="frames" and drew then
      if label~="warmup" then
        emit("frame",label,(dispatched-start)*1000,update_ms,draw_ms,finish_ms,(stop-dispatched)*1000,(stop-start)*1000)
      end
      count=count+1
      local limit=label=="warmup" and 30 or 90
      if count>=limit then
        phase,count=phase+1,0
        if phase>#phases then emit("done",mode); core.quit(true) end
      end
      core.redraw=true
    elseif armed and mode=="retention" then
      if cycles>=60 then emit("done",mode); system.sleep(0.05); core.quit(true) else core.redraw=true end
    end
    return drew
  end
  emit("init_complete",system.get_time(),variant)
  return original_run()
end
