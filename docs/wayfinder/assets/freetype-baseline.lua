-- Evidence-only runtime; use the existing test_hooks build, never a candidate backend.
-- FONT_BASELINE_MODE=capture|bench|retention; FONT_BASELINE_OUT=existing directory.
-- Optional public paths: FONT_BASELINE_SANS/MONO/CFF/CJK. Retention selects
-- FONT_BASELINE_CASE=mono|custom_cjk. No configured path is written to evidence.
local renderer, renwindow, system = require "renderer", require "renwindow", require "system"
local M = {}
local AA, HINT = {"none", "grayscale", "subpixel"}, {"none", "slight", "full"}
local ORDER = {"mono", "fira", "icons", "custom_sans", "custom_mono", "custom_cff", "custom_cjk"}
local ENV = {custom_sans="SANS", custom_mono="MONO", custom_cff="CFF", custom_cjk="CJK"}
local SAMPLE = {
  mono="Hamburgefonstiv 0123456789 {} =>",
  fira="Hamburgefonstiv ÉÀÇ ÆØ Ωμέγα",
  icons="!+-56789<>BCDLMPS",
  custom_sans="Hamburgefonstiv ÉÀÇ", custom_mono="Hamburgefonstiv 0123456789 {}",
  custom_cff="Hamburgefonstiv ÉÀÇ", custom_cjk="漢字かな中文テスト",
}
local WHITE, BLACK, RED = {255,255,255,255}, {0,0,0,255}, {255,0,0,255}
local REPS, MAX_LIVE, MAX_GLYPHS = 5, 8, 256
local base = (os.getenv("LITE_PREFIX") or "") .. "/share/lite-xl/fonts/"
local out, api, checks = nil, nil, 0

local function check(ok, name)
  assert(ok, name)
  checks = checks + 1
end
local function emit(s) io.stdout:write(s, "\n"); io.stdout:flush() end
local function gc() collectgarbage("collect"); collectgarbage("collect") end
local function write(path, data)
  local f = assert(io.open(path, "wb")); assert(f:write(data)); assert(f:close())
end
local function tsv(name, header, rows)
  local lines, columns = {header}, 1
  for _ in header:gmatch("\t") do columns = columns + 1 end
  for _, row in ipairs(rows) do
    assert(#row == columns, "TSV arity: " .. name)
    local cells = {}
    for i, v in ipairs(row) do
      cells[i] = tostring(v)
      assert(not cells[i]:find("[\t\r\n]"), "TSV separator in cell")
    end
    lines[#lines+1] = table.concat(cells, "\t")
  end
  write(out .. "/" .. name, table.concat(lines, "\n") .. "\n")
end
local function fonts()
  local paths = {mono=base.."JetBrainsMono-Regular.ttf", fira=base.."FiraSans-Regular.ttf", icons=base.."icons.ttf"}
  for id, suffix in pairs(ENV) do
    local p = os.getenv("FONT_BASELINE_" .. suffix)
    if p and p ~= "" then
      local info = system.get_file_info(p)
      paths[id] = info and info.type == "file" and p or nil
    end
  end
  -- Do not preload fonts: that would contaminate fresh-load and retention phases.
  return paths
end
local function load(path, size, opts)
  return renderer.font.load(assert(path), size or 16, opts or {antialiasing="grayscale"})
end
local function dirty_area()
  local area = 0
  for x,y,w,h in api.dirty_rects():gmatch("(-?%d+),(-?%d+),(%d+),(%d+)") do
    assert(tonumber(x) >= 0 and tonumber(y) >= 0)
    area = area + tonumber(w)*tonumber(h)
  end
  return area
end
local function frame(win, w, h, f, text, shade)
  renderer.begin_frame(win)
  renderer.draw_rect(0,0,w,h,{shade,shade,shade,255})
  if f then renderer.draw_text(f,text,4,4,WHITE) end
  renderer.end_frame()
end
local function replay_checks(win, w, h, path)
  local f = load(path)
  -- Prime the two cache grids and first-raster bearings before checking a stable frame.
  for _=1,4 do frame(win,w,h,f,"cache self-check",0) end
  frame(win,w,h,f,"cache self-check",0)
  check(api.dirty_rects()=="", "identical frame must skip replay")
  frame(win,w,h,f,"cache self-check",1)
  check(dirty_area()>=w*h, "full-background change must force full replay")
  local weak = setmetatable({}, {__mode="v"})
  renderer.begin_frame(win)
  do
    local temporary = load(path)
    weak[1] = temporary
    renderer.draw_text(temporary,"deferred GC",4,4,WHITE)
  end
  gc(); check(weak[1]~=nil, "queued font must stay rooted")
  renderer.end_frame()
  gc(); check(weak[1]==nil, "frame roots must release after replay")
end
local function ppm(win, id, w, h)
  local hash = api.surface_hash(win,0,0,w,h)
  assert(hash:match("^%x+/%d+$"))
  check(api.pixel(win,w,0)==nil and api.pixel(win,0,h)==nil, "capture surface bounds match logical scale 1")
  local f = assert(io.open(out.."/sheet-"..id..".ppm","wb"))
  assert(f:write(("P6\n%d %d\n255\n"):format(w,h)))
  for y=0,h-1 do
    local row = {}
    for x=0,w-1 do
      local hex = assert(api.pixel(win,x,y))
      assert(hex:match("^%x%x%x%x%x%x$"))
      row[#row+1] = string.char(tonumber(hex:sub(1,2),16),tonumber(hex:sub(3,4),16),tonumber(hex:sub(5,6),16))
    end
    assert(f:write(table.concat(row)))
  end
  assert(f:close())
  return {id,w,h,hash,tonumber(hash:match("/(%d+)$"))}
end

function M.capture()
  local paths, metrics, images, geometry = fonts(), {}, {}, {}
  local label = load(paths.fira,10)
  local function marked(id, f, text, x, y, tab)
    assert(#text<=4*MAX_GLYPHS, "bounded byte length (including malformed UTF-8)")
    local width = f:get_width(text,tab)
    local finish = renderer.draw_text(f,text,x,y,WHITE,tab)
    check(math.abs(finish-x-width)<0.00001, "returned pen must equal measured end")
    renderer.draw_rect(math.floor(finish+0.5),y,2,f:get_height(),RED)
    geometry[#geometry+1] = {id,x,y,width,finish,f:get_height()}
  end
  for _,id in ipairs(ORDER) do
    local path = paths[id]
    if path then
      local win = renwindow.create("font-matrix",960,200)
      -- Same-size windows keep rencache's grid; a distinct frame forces every cell to repaint.
      frame(win,960,200,nil,nil,1)
      renderer.begin_frame(win); renderer.draw_rect(0,0,960,200,BLACK)
      for col,aa in ipairs(AA) do for row,hint in ipairs(HINT) do
        local x,y = (col-1)*320+6,(row-1)*66+4
        renderer.set_clip_rect((col-1)*320,(row-1)*66,320,66)
        renderer.draw_text(label,aa.." / "..hint,x,y,WHITE)
        local f = load(path,16,{antialiasing=aa,hinting=hint})
        marked(id.."/"..aa.."/"..hint,f,SAMPLE[id],x,y+14)
        metrics[#metrics+1] = {id,aa,hint,f:get_size(),f:get_height(),f:get_width(SAMPLE[id]),f:get_width("A"),f:get_width(" "),f:get_width("\t"),f:get_width("\t",{tab_offset=0}),f:get_width("\t",{tab_offset=3.5}),f:get_width("\u{301}")}
      end end
      renderer.set_clip_rect(0,0,960,200); renderer.end_frame()
      check(dirty_area()>=960*200, "sheet frame repaints the whole surface")
      images[#images+1] = ppm(win,id,960,200)
      gc()
    else
      emit("CENSORED font="..id.." reason=unset_or_missing_custom_file")
    end
  end
  local win = renwindow.create("font-features",960,600)
  frame(win,960,600,nil,nil,1)
  renderer.begin_frame(win); renderer.draw_rect(0,0,960,600,BLACK)
  local y = 4
  local function line(id, f, text, tab, step)
    renderer.draw_text(label,id,6,y,WHITE)
    marked(id,f,text,210,y,tab)
    y = y+(step or 22)
  end
  local mono,fira,icons = load(paths.mono),load(paths.fira),load(paths.icons)
  line("ASCII-mapped icons",icons,"!+-56789<>BCDLMPSWX_dfghiw")
  local group = renderer.font.group({icons,fira})
  check(group:get_width("é")==fira:get_width("é"), "genuine second-font glyph width")
  line("icons + Fira fallback",group,"é Ω €")
  if paths.custom_cjk then
    local cjk = load(paths.custom_cjk)
    group = renderer.font.group({mono,cjk})
    check(group:get_width("中")==cjk:get_width("中"), "CJK fallback width")
    check(group:get_width("中")~=mono:get_width("中"), "CJK differs from Mono missing glyph")
    line("Mono + CJK face 0",group,"abc 中文 日本語")
  else
    renderer.draw_text(label,"CJK fallback CENSORED",6,y,WHITE); y=y+22
  end
  group = renderer.font.group({fira,icons})
  check(group:get_width("中")==icons:get_width("中"), "last-font notdef width")
  line("absent: last .notdef",group,"a中b")
  mono:set_tab_size(4)
  line("tabs: no offset",mono,"a\tb\tc")
  line("tabs: offset 3.5",mono,"a\tb\tc",{tab_offset=3.5})
  renderer.draw_text(label,"fractional x (0,.2,.34,.5,.67,.99)",6,y,WHITE); y=y+14
  local sub = load(paths.mono,16,{antialiasing="subpixel",hinting="slight"})
  for i,x in ipairs{10,10.2,10.34,10.5,10.67,10.99} do marked("phase"..i,sub,"iIl1|.:",x+(i-1)*150,y) end
  y=y+24
  for _,style in ipairs{
    {"bold",{bold=true}}, {"italic",{italic=true}}, {"bold italic",{bold=true,italic=true}},
    {"smoothing",{smoothing=true}}, {"underline",{underline=true}}, {"strike",{strikethrough=true}},
    {"underline + strike",{underline=true,strikethrough=true}},
    {"all styles",{bold=true,italic=true,smoothing=true,underline=true,strikethrough=true}},
  } do
    style[2].antialiasing="grayscale"
    line(style[1],load(paths.fira,16,style[2]),"Styled Wgq fi",nil,20)
  end
  for _,size in ipairs{12,15,30} do
    line("logical size "..size,load(paths.fira,size),"Agmw é € []",nil,size+6)
  end
  line("malformed bytes",fira,"a\xffb\xc3c\xe4\xb8d")
  line("embedded NUL",fira,"ab\0cd")
  line("combining / whitespace",fira,"e\u{301} \u{3000}space")
  check(y<=600, "feature sheet fits fixed bounds")
  renderer.end_frame()
  check(dirty_area()>=960*600, "feature frame repaints the whole surface")
  images[#images+1]=ppm(win,"features",960,600)
  replay_checks(win,960,600,paths.mono)
  tsv("font-metrics.tsv","id\taa\thint\tsize\theight\tsample_width\twidth_A\tspace\ttab_none\ttab_off0\ttab_off3.5\tcombining",metrics)
  tsv("image-sheets.tsv","id\tW\tH\tsurface_hash\tpixels",images)
  tsv("feature-metrics.tsv","id\tx\ty\twidth\treturned_x\theight",geometry)
  emit(("CAPTURE sheets=%d metrics=%d geometry=%d checks=%d"):format(#images,#metrics,#geometry,checks))
end

function M.bench()
  local paths, rows = fonts(), {}
  local win,w,h = renwindow.create("font-cost",400,100),400,100
  replay_checks(win,w,h,paths.mono)
  local shade = 0
  local function timed_frame(f,text)
    shade=1-shade
    local t=system.get_time()
    frame(win,w,h,f,text,shade)
    local elapsed=system.get_time()-t
    check(dirty_area()>=w*h, "timed frame really replays whole surface")
    return elapsed
  end
  for _,id in ipairs(ORDER) do
    local path = paths[id]
    if path then
      local text=SAMPLE[id]
      assert(assert(utf8.len(text))<=MAX_GLYPHS)
      for _,aa in ipairs(AA) do for _,hint in ipairs(HINT) do
        for rep=1,REPS do
          gc()
          local t=system.get_time()
          local f=load(path,16,{antialiasing=aa,hinting=hint})
          local loaded=system.get_time()-t
          t=system.get_time(); f:get_width(text); local firstwidth=system.get_time()-t
          local firstdraw=timed_frame(f,text)
          t=system.get_time(); for _=1,100 do f:get_width(text) end
          local warmwidth=system.get_time()-t
          local warmdraw,warmmax=0,0
          for _=1,10 do
            local elapsed=timed_frame(f,text)
            warmdraw=warmdraw+elapsed; warmmax=math.max(warmmax,elapsed)
          end
          rows[#rows+1]={id,aa,hint,rep,("%.9f"):format(loaded),("%.9f"):format(firstwidth),("%.9f"):format(firstdraw),("%.9f"):format(warmwidth),("%.9f"):format(warmdraw),("%.9f"):format(warmmax)}
        end
      end end
    else emit("CENSORED font="..id.." reason=unset_or_missing_custom_file") end
  end
  tsv("font-timing.tsv","id\taa\thint\trep\tload_s\twidth_s\tfirstdraw_s\twarmwidth100_s\twarmdraw10_s\twarmdrawmax_s",rows)
  emit(("BENCH samples=%d reps=%d widths_per_batch=100 frames_per_batch=10 checks=%d timer=wall"):format(#rows,REPS,checks))
end

function M.retention()
  local id=os.getenv("FONT_BASELINE_CASE") or "mono"
  assert(id=="mono" or id=="custom_cjk", "retention case must be mono or custom_cjk")
  local path=assert(fonts()[id], "retention font not configured")
  local win,w,h=renwindow.create("font-retention",960,140),960,140
  local rows,shade={},0
  local function draw(f,text)
    shade=1-shade; frame(win,w,h,f,text,shade)
  end
  local function record(phase)
    gc()
    local file=assert(io.open("/proc/self/status","r"))
    local s=file:read("a"); assert(file:close())
    local rss=assert(tonumber(s:match("\nVmRSS:%s+(%d+) kB")))
    local hwm=assert(tonumber(s:match("\nVmHWM:%s+(%d+) kB")))
    check(rss>0 and hwm>=rss, "valid process memory counters")
    rows[#rows+1]={id,phase,rss,hwm,("%.3f"):format(collectgarbage("count"))}
  end
  local function glyphs(n)
    assert(n<=MAX_GLYPHS)
    local chars={}
    for i=1,n do chars[i]=utf8.char((id=="custom_cjk" and 0x4e00 or 32)+i-1) end
    return table.concat(chars)
  end
  draw(nil); record("baseline")
  local weak=setmetatable({}, {__mode="v"})
  local f=load(path); weak[1]=f; record("loaded")
  for _,n in ipairs{16,64,256} do
    local text=glyphs(n)
    f:get_width(text); record("width"..n)
    draw(f,text); record("raster"..n)
  end
  f=nil; draw(nil); gc(); check(weak[1]==nil, "grown font released"); record("drop")
  for cycle=1,8 do
    do
      local font=load(path)
      font:set_size(16+cycle)
      local copy=font:copy(16+cycle)
      weak[1],weak[2]=font,copy
      draw(font,glyphs(64)); draw(copy,glyphs(64))
    end
    draw(nil); gc(); check(weak[1]==nil and weak[2]==nil, "resize/copy cycle released")
    record("cycle"..cycle)
  end
  local held={}
  for i=1,MAX_LIVE do held[i]=load(path,12+i); weak[i]=held[i]; draw(held[i],glyphs(64)) end
  record("live8")
  for i=1,MAX_LIVE do check(weak[i]~=nil, "held font alive") end
  held=nil; draw(nil); gc()
  for i=1,MAX_LIVE do check(weak[i]==nil, "held font released") end
  record("live8-released")
  tsv("font-retention-"..id..".tsv","id\tphase\trss_kb\thwm_kb\tlua_kb",rows)
  emit(("RETENTION id=%s phases=%d checks=%d max_live=%d max_glyphs=%d"):format(id,#rows,checks,MAX_LIVE,MAX_GLYPHS))
end

function M.init()
  api=assert(lxl_test, "requires test_hooks")
  out=assert(os.getenv("FONT_BASELINE_OUT"), "FONT_BASELINE_OUT required")
  assert((system.get_file_info(out) or {}).type=="dir", "output directory must exist")
  local mode=os.getenv("FONT_BASELINE_MODE")
  assert(mode=="capture" or mode=="bench" or mode=="retention", "invalid FONT_BASELINE_MODE")
  M[mode]()
  os.exit(0)
end
function M.run() end
function M.on_error(err)
  io.stderr:write("FREETYPE BASELINE FAILED: ",tostring(err),"\n"); os.exit(1)
end
return M
