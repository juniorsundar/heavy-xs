-- One public font/size per process; metadata proof comes from font-format-metadata.rs.
-- FORMAT_ID, FORMAT_FONT, FORMAT_SIZE, FORMAT_CP (hex), FORMAT_AA, FORMAT_HINT, FORMAT_OUT.
-- Use none AA for packed-mono bitmap payloads; the current grayscale copy path is unsafe.
local renderer, renwindow = require "renderer", require "renwindow"
local M = {}
local W,H,X,Y = 256,180,8,8
local checks = 0
local function check(ok, why) assert(ok,why); checks=checks+1 end
local function emit(s) io.stdout:write(s,"\n"); io.stdout:flush() end
local function clean(err, path)
  local s=tostring(err):gsub("^.-:%d+:%s*",""):gsub(path:gsub("(%W)","%%%1"),"<font>")
  return (s:gsub("[\t\r\n]"," "))
end
local function save(path, data)
  local f=assert(io.open(path,"wb")); assert(f:write(data)); assert(f:close())
end
local function area()
  local n=0
  for w,h in lxl_test.dirty_rects():gmatch("%-?%d+,%-?%d+,(%d+),(%d+)") do n=n+tonumber(w)*tonumber(h) end
  return n
end
local function capture(win, path)
  local hash=lxl_test.surface_hash(win,0,0,W,H)
  check(lxl_test.pixel(win,W,0)==nil and lxl_test.pixel(win,0,H)==nil,"scale-1 capture bounds")
  local ppm={}; local lit,chromatic=0,0
  local left,top,right,bottom=W,H,-1,-1
  for y=0,H-1 do
    local row={}
    for x=0,W-1 do
      local hex=assert(lxl_test.pixel(win,x,y))
      assert(hex:match("^%x%x%x%x%x%x$"))
      local r,g,b=tonumber(hex:sub(1,2),16),tonumber(hex:sub(3,4),16),tonumber(hex:sub(5,6),16)
      row[#row+1]=string.char(r,g,b)
      if r~=0 or g~=0 or b~=0 then
        lit=lit+1; left=math.min(left,x); top=math.min(top,y); right=math.max(right,x); bottom=math.max(bottom,y)
      end
      if r~=g or g~=b then chromatic=chromatic+1 end
    end
    ppm[#ppm+1]=table.concat(row)
  end
  check(lit==tonumber(hash:match("/(%d+)$")),"pixel scan agrees with hook lit count")
  check(chromatic<=lit,"chromatic count bounded by lit pixels")
  local bw,bh=right>=left and right-left+1 or 0,bottom>=top and bottom-top+1 or 0
  save(path,("P6\n%d %d\n255\n"):format(W,H)..table.concat(ppm))
  return hash,lit,chromatic,("%d,%d,%d,%d"):format(bw>0 and left or 0,bh>0 and top or 0,bw,bh),lit>0 and lit==bw*bh
end
function M.init()
  assert(lxl_test,"requires test_hooks")
  local id=assert(os.getenv("FORMAT_ID"))
  local path=assert(os.getenv("FORMAT_FONT"))
  local out=assert(os.getenv("FORMAT_OUT"))
  local size=assert(tonumber(os.getenv("FORMAT_SIZE")))
  local cp=assert(tonumber(os.getenv("FORMAT_CP"),16))
  local hint=os.getenv("FORMAT_HINT") or "none"
  local aa=assert(os.getenv("FORMAT_AA"))
  assert(#id<=64 and id:match("^[%w_-]+$"),"invalid case id")
  assert(size>=1 and size<=256 and size==math.floor(size),"size bound")
  assert(cp>0 and cp<=0x10ffff and not (cp>=0xd800 and cp<=0xdfff),"invalid target scalar")
  assert(hint=="none" or hint=="slight" or hint=="full","invalid hint")
  assert(aa=="none" or aa=="grayscale","invalid antialiasing")
  local rows={"id\tkind\tcp\tsize\taa\thint\tload\twidth_status\tdraw_status\tfont_height\twidth\treturned_x\thash\tlit\tchromatic\tbbox\tsolid_bbox\terror"}
  local function record(values)
    check(#values==18,"TSV arity")
    for i,v in ipairs(values) do values[i]=tostring(v); assert(not values[i]:find("[\t\r\n]"),"TSV cell boundary") end
    rows[#rows+1]=table.concat(values,"\t")
  end
  emit("PHASE load "..id)
  local loaded,font=pcall(renderer.font.load,path,size,{antialiasing=aa,hinting=hint})
  if not loaded then
    record({id,"load",("U+%04X"):format(cp),size,aa,hint,"error","-","-","-","-","-","-",0,0,"-",false,clean(font,path)})
  else
    local win=renwindow.create("font-format-reference",W,H)
    local hashes={}
    for _,sample in ipairs{{"empty","",0},{"target",utf8.char(cp),cp},{"missing",utf8.char(0x10ffff),0x10ffff},{"missing2",utf8.char(0x10fffe),0x10fffe}} do
      local kind,text,code=table.unpack(sample)
      emit("PHASE measure "..id.." "..kind)
      local measured,width=pcall(font.get_width,font,text)
      if not measured then
        record({id,kind,("U+%04X"):format(code),size,aa,hint,"ok","error","-",font:get_height(),"-","-","-",0,0,"-",false,clean(width,path)})
      else
        -- The distinct priming frame prevents cross-frame/window cache reuse from hiding pixels.
        renderer.begin_frame(win); renderer.draw_rect(0,0,W,H,{1,1,1,255}); renderer.end_frame()
        emit("PHASE draw "..id.." "..kind)
        local drawn,finish=pcall(function()
          renderer.begin_frame(win); renderer.draw_rect(0,0,W,H,{0,0,0,255})
          local x=renderer.draw_text(font,text,X,Y,{255,255,255,255})
          renderer.end_frame(); return x
        end)
        if not drawn then
          -- An interrupted native frame is not reusable; the next sample cannot be trusted.
          record({id,kind,("U+%04X"):format(code),size,aa,hint,"ok","ok","error",font:get_height(),width,"-","-",0,0,"-",false,clean(finish,path)})
          break
        end
        check(area()>=W*H,"capture frame must fully repaint")
        check(math.abs(finish-X-width)<0.00001,"returned pen matches measured advance")
        local hash,lit,colour,bbox,solid=capture(win,out.."/"..id.."-"..kind..".ppm")
        hashes[kind]=hash
        if kind=="empty" then check(lit==0 and width==0,"empty text has no ink or advance") end
        record({id,kind,kind=="empty" and "-" or ("U+%04X"):format(code),size,aa,hint,"ok","ok","ok",font:get_height(),width,finish,hash,lit,colour,bbox,solid,"-"})
      end
    end
    -- The metadata helper separately verifies that both control scalars map to glyph 0.
    if hashes.missing and hashes.missing2 then check(hashes.missing==hashes.missing2,"unmapped glyph-0 controls agree") end
  end
  save(out.."/"..id..".tsv",table.concat(rows,"\n").."\n")
  emit(("FORMAT id=%s rows=%d checks=%d"):format(id,#rows-1,checks))
  os.exit(0)
end
function M.run() end
function M.on_error(err)
  io.stderr:write("FORMAT PROBE FAILED: ",tostring(err):gsub("^.-:%d+:%s*",""),"\n"); os.exit(1)
end
return M
