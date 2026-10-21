-- system.list_dir / system.rmdir beyond the `system` suite: the exact failure messages, native
-- (non-UTF-8) names, symlinks, truncation at NUL and enumeration order. The golden was recorded
-- from the pinned SDL reference (scripts/rust-port/sdl-reference.sh) before these functions moved
-- off SDL, so the wording stays SDL's. Permission cases assume a non-root user.

local ROOT = "/tmp/lxl_sysfs_contract"

local function sh(cmd) assert(os.execute(cmd)) end

local function show(...)
  local n = select("#", ...)
  if n == 0 then return "<none>" end
  local out = {}
  for i = 1, n do
    local v = select(i, ...)
    if type(v) == "table" then
      local names = {}
      for j, name in ipairs(v) do names[j] = name end
      table.sort(names)
      v = "{" .. table.concat(names, "|") .. "}"
    end
    out[i] = tostring(v)
  end
  return n .. ":" .. table.concat(out, ",")
end

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  if not r[1] then return "ERR" end
  return show(table.unpack(r, 2, r.n))
end

local function exists(path) return os.execute("test -e '" .. path .. "' || test -L '" .. path .. "'") == true end

local function hex(s) return (s:gsub(".", function(c) return ("%02x"):format(c:byte()) end)) end

return function(rec)
  sh("chmod -R u+rwx " .. ROOT .. " 2>/dev/null; rm -rf " .. ROOT .. " && mkdir -p " .. ROOT)
  sh("cd " .. ROOT .. " && mkdir ls && touch ls/b ls/a ls/.hidden && mkdir ls/sub && echo x > file.txt")
  sh("cd " .. ROOT .. " && ln -s ls lslink && ln -s nope dangling && ln -s loop loop && ln -s file.txt filelink")
  sh("cd " .. ROOT .. " && mkdir locked && chmod 000 locked")
  local long = ROOT .. "/" .. ("n"):rep(300)

  -- list_dir ------------------------------------------------------------------
  rec("list missing", try(system.list_dir, ROOT .. "/missing"))
  rec("list file", try(system.list_dir, ROOT .. "/file.txt"))
  rec("list empty string", try(system.list_dir, ""))
  rec("list unreadable", try(system.list_dir, ROOT .. "/locked"))
  rec("list trailing slash", try(system.list_dir, ROOT .. "/ls/"))
  rec("list through file", try(system.list_dir, ROOT .. "/file.txt/x"))
  rec("list dangling link", try(system.list_dir, ROOT .. "/dangling"))
  rec("list link loop", try(system.list_dir, ROOT .. "/loop"))
  rec("list file link", try(system.list_dir, ROOT .. "/filelink"))
  rec("list dir link", try(system.list_dir, ROOT .. "/lslink"))
  rec("list name too long", try(system.list_dir, long))
  rec("list NUL truncates", try(system.list_dir, ROOT .. "/ls\0junk"))
  rec("list number arg", try(system.list_dir, 5))

  -- Enumeration order is the directory's own (readdir), not sorted.
  sh("cd " .. ROOT .. " && mkdir order && cd order && for n in m c x a q 10 2 B; do touch $n; done")
  local listed = table.concat(system.list_dir(ROOT .. "/order"), " ")
  local p = io.popen("ls -1aU " .. ROOT .. "/order")
  local raw = {}
  for line in p:lines() do if line ~= "." and line ~= ".." then raw[#raw + 1] = line end end
  p:close()
  rec("list order is readdir order", tostring(listed == table.concat(raw, " ")))

  -- Names are bytes: a non-UTF-8 name comes back unchanged and can be removed by those bytes.
  local native = "\xff\xfe-\x80.bin"
  sh("mkdir " .. ROOT .. "/native")
  assert(io.open(ROOT .. "/native/" .. native, "wb")):close()
  local names = system.list_dir(ROOT .. "/native")
  rec("list native name", #names .. ":" .. hex(names[1] or ""))
  rec("rmdir native name", try(system.rmdir, ROOT .. "/native/" .. names[1]))
  rec("native name gone", tostring(exists(ROOT .. "/native/" .. native)))

  -- rmdir ---------------------------------------------------------------------
  rec("rmdir non-empty", try(system.rmdir, ROOT .. "/ls"))
  rec("rmdir missing", try(system.rmdir, ROOT .. "/missing"))
  rec("rmdir missing parent", try(system.rmdir, ROOT .. "/missing/x"))
  rec("rmdir through file", try(system.rmdir, ROOT .. "/file.txt/x"))
  rec("rmdir empty string", try(system.rmdir, ""))
  rec("rmdir dot", try(system.rmdir, ROOT .. "/ls/sub/."))
  rec("rmdir name too long", try(system.rmdir, long))
  rec("rmdir number arg", try(system.rmdir, 5))
  sh("mkdir -p " .. ROOT .. "/guard/kept && chmod 555 " .. ROOT .. "/guard")
  rec("rmdir in read-only dir", try(system.rmdir, ROOT .. "/guard/kept"))
  sh("chmod 755 " .. ROOT .. "/guard")
  rec("rmdir unreadable empty dir", try(system.rmdir, ROOT .. "/locked"))
  rec("rmdir dir link", try(system.rmdir, ROOT .. "/lslink"))
  rec("dir link gone, target kept", tostring(exists(ROOT .. "/lslink")) .. "|" .. tostring(exists(ROOT .. "/ls/a")))
  rec("rmdir dangling link", try(system.rmdir, ROOT .. "/dangling"))
  rec("dangling link gone", tostring(exists(ROOT .. "/dangling")))
  rec("rmdir file link", try(system.rmdir, ROOT .. "/filelink"))
  rec("file link gone, target kept", tostring(exists(ROOT .. "/filelink")) .. "|" .. tostring(exists(ROOT .. "/file.txt")))
  sh("mkfifo " .. ROOT .. "/fifo && mkdir " .. ROOT .. "/slash && mkdir " .. ROOT .. "/nulgone")
  rec("rmdir fifo", try(system.rmdir, ROOT .. "/fifo"))
  rec("rmdir trailing slash dir", try(system.rmdir, ROOT .. "/slash/"))
  rec("rmdir trailing slash file", try(system.rmdir, ROOT .. "/file.txt/"))
  rec("rmdir NUL truncates", try(system.rmdir, ROOT .. "/nulgone\0junk"))
  rec("NUL target gone", tostring(exists(ROOT .. "/nulgone")))

  sh("chmod -R u+rwx " .. ROOT .. "; rm -rf " .. ROOT)
end
