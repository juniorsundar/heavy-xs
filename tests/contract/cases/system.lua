-- The pure / filesystem / environment half of `system` (src/system.rs).
--
-- Machine-specific values (mtime of something we did not create, the filesystem a
-- temp dir lives on, the pid) are reduced to a type or to a comparison, so the
-- baseline stays valid for the next person.

local ROOT = "/tmp/lxl_system_contract"

local function sh(cmd) assert(os.execute(cmd)) end

-- Lua nil vs "no value at all" is part of the contract, so keep the arity.
local function show(...)
  local n = select("#", ...)
  if n == 0 then return "<none>" end
  local out = {}
  for i = 1, n do
    local v = select(i, ...)
    out[i] = type(v) == "number" and (math.type(v) .. ":" .. tostring(v)) or tostring(v)
  end
  return n .. ":" .. table.concat(out, ",")
end

local function try(fn, ...)
  local r = table.pack(pcall(fn, ...))
  if not r[1] then return "ERR" end
  return show(table.unpack(r, 2, r.n))
end

local function write(path, text)
  local f = assert(io.open(path, "wb"))
  f:write(text)
  f:close()
end

local function read(path)
  local f = io.open(path, "rb")
  if not f then return nil end
  local s = f:read("a")
  f:close()
  return s
end

local function sorted(t)
  local copy = {}
  for i, v in ipairs(t) do copy[i] = v end
  table.sort(copy)
  return table.concat(copy, "|")
end

return function(rec)
  sh("rm -rf " .. ROOT .. " && mkdir -p " .. ROOT)
  local start_cwd = system.absolute_path(".")

  -- Surface: every function the C table exports must still exist.
  local names = {}
  for k, v in pairs(system) do names[#names + 1] = k .. ":" .. type(v) end
  table.sort(names)
  rec("exports", #names .. " " .. table.concat(names, " "))

  -- get_file_info ------------------------------------------------------------
  write(ROOT .. "/file.txt", "hello")
  sh("mkdir " .. ROOT .. "/dir")
  sh("ln -s " .. ROOT .. "/dir " .. ROOT .. "/dirlink")
  sh("ln -s " .. ROOT .. "/file.txt " .. ROOT .. "/filelink")
  sh("ln -s " .. ROOT .. "/nope " .. ROOT .. "/dangling")
  sh("mkfifo " .. ROOT .. "/fifo")
  sh("touch -d '2020-01-02 03:04:05.678901234' " .. ROOT .. "/file.txt")

  local function info(path)
    local t, msg = system.get_file_info(path)
    if not t then return "nil|" .. tostring(msg) end
    local keys = {}
    for k, v in pairs(t) do keys[#keys + 1] = k .. "=" .. type(v) .. ":" .. tostring(v) end
    table.sort(keys)
    return table.concat(keys, ";")
  end
  rec("info file", info(ROOT .. "/file.txt"))
  rec("info modified %.6f", string.format("%.6f", system.get_file_info(ROOT .. "/file.txt").modified))
  rec("info dir keys", (info(ROOT .. "/dir"):gsub("modified=number:[%d.]+", "modified=N")))
  rec("info dirlink keys", (info(ROOT .. "/dirlink"):gsub("modified=number:[%d.]+", "modified=N")))
  rec("info filelink keys", (info(ROOT .. "/filelink"):gsub("modified=number:[%d.]+", "modified=N")))
  rec("info dangling", info(ROOT .. "/dangling"))
  rec("info fifo keys", (info(ROOT .. "/fifo"):gsub("modified=number:[%d.]+", "modified=N")))
  rec("info missing", info(ROOT .. "/missing"))
  rec("info empty path", info(""))
  rec("info too long", info(string.rep("a", 5000)))
  rec("info not a dir", info(ROOT .. "/file.txt/x"))
  rec("info number arg", try(system.get_file_info, 42))
  rec("info no arg", try(system.get_file_info))
  rec("info table arg", try(system.get_file_info, {}))
  rec("info /dev/null type", tostring(system.get_file_info("/dev/null").type))

  -- absolute_path ------------------------------------------------------------
  rec("abs file", try(system.absolute_path, ROOT .. "/file.txt"))
  rec("abs dotdot", try(system.absolute_path, ROOT .. "/dir/../file.txt"))
  rec("abs symlink", try(system.absolute_path, ROOT .. "/dirlink"))
  rec("abs trailing slash", try(system.absolute_path, ROOT .. "/dir/"))
  rec("abs missing", try(system.absolute_path, ROOT .. "/missing"))
  rec("abs empty", try(system.absolute_path, ""))
  rec("abs dangling", try(system.absolute_path, ROOT .. "/dangling"))
  rec("abs root", try(system.absolute_path, "/"))
  rec("abs no arg", try(system.absolute_path))
  rec("abs number", try(system.absolute_path, 7))

  -- chdir --------------------------------------------------------------------
  rec("chdir ok", try(system.chdir, ROOT .. "/dir"))
  rec("cwd after chdir", try(system.absolute_path, "."))
  rec("rel path after chdir", try(system.absolute_path, "../file.txt"))
  rec("chdir missing", try(system.chdir, ROOT .. "/missing"))
  rec("chdir file", try(system.chdir, ROOT .. "/file.txt"))
  rec("chdir no arg", try(system.chdir))
  rec("cwd unchanged by failure", try(system.absolute_path, "."))
  system.chdir(start_cwd)

  -- mkdir / rmdir ------------------------------------------------------------
  rec("mkdir new", try(system.mkdir, ROOT .. "/made"))
  rec("mkdir exists", try(system.mkdir, ROOT .. "/made"))
  rec("mkdir nested missing", try(system.mkdir, ROOT .. "/a/b/c"))
  rec("mkdir over file", try(system.mkdir, ROOT .. "/file.txt"))
  rec("mkdir empty", try(system.mkdir, ""))
  rec("mkdir no arg", try(system.mkdir))
  rec("mkdir is dir", system.get_file_info(ROOT .. "/made").type)
  rec("mkdir mode", (io.popen("stat -c %a " .. ROOT .. "/made"):read("l") or "?"):gsub("[0-7]$", "x"))
  sh("touch " .. ROOT .. "/made/child")
  local ok, msg = system.rmdir(ROOT .. "/made")
  rec("rmdir non-empty", tostring(ok) .. "|" .. tostring(msg ~= nil))
  os.remove(ROOT .. "/made/child")
  rec("rmdir empty", try(system.rmdir, ROOT .. "/made"))
  rec("rmdir gone", tostring(system.get_file_info(ROOT .. "/made")))
  local ok2, msg2 = system.rmdir(ROOT .. "/made")
  rec("rmdir missing", tostring(ok2) .. "|" .. tostring(msg2 ~= nil))
  rec("rmdir file", (select(1, system.rmdir(ROOT .. "/file.txt"))))
  rec("rmdir no arg", try(system.rmdir))

  -- list_dir -----------------------------------------------------------------
  sh("mkdir " .. ROOT .. "/ls && touch " .. ROOT .. "/ls/b " .. ROOT .. "/ls/a " .. ROOT .. "/ls/.hidden && mkdir " .. ROOT .. "/ls/sub")
  local list = system.list_dir(ROOT .. "/ls")
  rec("list_dir", sorted(list))
  rec("list_dir count", tostring(#list))
  rec("list_dir empty", select("#", system.list_dir(ROOT .. "/dir")) .. ":" .. #system.list_dir(ROOT .. "/dir"))
  rec("list_dir type", type(system.list_dir(ROOT .. "/dir")))
  local lm1, lm2 = system.list_dir(ROOT .. "/missing")
  rec("list_dir missing", tostring(lm1) .. "|" .. type(lm2))
  local lf1, lf2 = system.list_dir(ROOT .. "/file.txt")
  rec("list_dir on file", tostring(lf1) .. "|" .. type(lf2))
  rec("list_dir no arg", try(system.list_dir))
  rec("list_dir symlink", sorted(system.list_dir(ROOT .. "/dirlink")))

  -- get_fs_type --------------------------------------------------------------
  rec("fs_type /proc", try(system.get_fs_type, "/proc"))
  rec("fs_type /sys", try(system.get_fs_type, "/sys"))
  rec("fs_type root type", type(system.get_fs_type("/")))
  rec("fs_type tmp type", type(system.get_fs_type(ROOT)))
  rec("fs_type missing", try(system.get_fs_type, ROOT .. "/missing"))
  rec("fs_type no arg", try(system.get_fs_type))

  -- ftruncate ----------------------------------------------------------------
  local tpath = ROOT .. "/trunc.txt"
  write(tpath, "0123456789")
  local f = assert(io.open(tpath, "r+b"))
  rec("ftruncate 4", show(system.ftruncate(f, 4)))
  f:close()
  rec("ftruncate content", tostring(read(tpath)))
  f = assert(io.open(tpath, "r+b"))
  rec("ftruncate grow", show(system.ftruncate(f, 8)))
  f:close()
  rec("ftruncate grow content", (tostring(read(tpath)):gsub("%z", "0")))
  f = assert(io.open(tpath, "r+b"))
  rec("ftruncate default", show(system.ftruncate(f)))
  f:close()
  rec("ftruncate default size", tostring(system.get_file_info(tpath).size))
  f = assert(io.open(tpath, "rb"))
  rec("ftruncate read-only", show(system.ftruncate(f, 1)))
  f:close()
  f = assert(io.open(tpath, "r+b"))
  rec("ftruncate negative", show(system.ftruncate(f, -1)))
  f:close()
  rec("ftruncate non-file", try(system.ftruncate, {}))
  rec("ftruncate nil", try(system.ftruncate, nil))
  rec("ftruncate string", try(system.ftruncate, "x"))
  -- Never ftruncate io.stdout here: the harness redirects it to the baseline file.

  -- process / time -----------------------------------------------------------
  local stat = read("/proc/self/stat") or ""
  rec("pid matches", tostring(system.get_process_id() == tonumber(stat:match("^(%d+)"))))
  rec("pid type", math.type(system.get_process_id()))
  rec("time type", math.type(system.get_time()))
  local t0 = system.get_time()
  system.sleep(0.05)
  local dt = system.get_time() - t0
  rec("sleep 0.05 waits", tostring(dt >= 0.045 and dt < 0.5))
  rec("sleep returns", show(system.sleep(0)))
  rec("sleep negative", show(system.sleep(-5)))
  rec("sleep string", try(system.sleep, "0"))
  rec("sleep bad", try(system.sleep, "x"))
  rec("sleep none", try(system.sleep))
  rec("time monotonic", tostring(system.get_time() >= t0))

  -- exec ---------------------------------------------------------------------
  os.remove(ROOT .. "/exec.out")
  rec("exec returns", show(system.exec("echo ran > " .. ROOT .. "/exec.out")))
  local waited = system.get_time()
  while (read(ROOT .. "/exec.out") or "") == "" and system.get_time() - waited < 3 do end
  rec("exec ran in background", (tostring(read(ROOT .. "/exec.out")):gsub("\n", "\\n")))
  rec("exec failing cmd", show(system.exec("exit 3")))
  rec("exec no arg", try(system.exec))
  os.remove(ROOT .. "/exec.out")
  local started = system.get_time()
  system.exec("sleep 1")
  rec("exec does not block", tostring(system.get_time() - started < 0.5))

  -- setenv / sandbox ---------------------------------------------------------
  rec("setenv", show(system.setenv("LXL_CONTRACT_A", "one")))
  rec("getenv sees it", tostring(os.getenv("LXL_CONTRACT_A")))
  rec("setenv overwrite", show(system.setenv("LXL_CONTRACT_A", "two")))
  rec("getenv overwritten", tostring(os.getenv("LXL_CONTRACT_A")))
  rec("setenv empty value", show(system.setenv("LXL_CONTRACT_A", "")))
  rec("getenv empty", "[" .. tostring(os.getenv("LXL_CONTRACT_A")) .. "]")
  rec("setenv eq in key", show(system.setenv("A=B", "x")))
  rec("setenv empty key", show(system.setenv("", "x")))
  rec("setenv number", show(system.setenv("LXL_CONTRACT_B", 5)))
  rec("getenv number", tostring(os.getenv("LXL_CONTRACT_B")))
  rec("setenv missing value", try(system.setenv, "LXL_CONTRACT_C"))
  rec("setenv no args", try(system.setenv))
  rec("sandbox", try(system.get_sandbox))
  rec("sandbox type", type(system.get_sandbox()))

  -- clipboard ----------------------------------------------------------------
  rec("set_clipboard returns", show(system.set_clipboard("hello clip")))
  rec("get_clipboard", show(system.get_clipboard()))
  rec("set_clipboard empty", show(system.set_clipboard("")))
  rec("get_clipboard empty", try(system.get_clipboard))
  system.set_clipboard("multi\nline")
  rec("clipboard newline", (tostring(system.get_clipboard()):gsub("\n", "\\n")))
  rec("set_clipboard no arg", try(system.set_clipboard))
  rec("set_clipboard number", show(system.set_clipboard(12)))
  rec("get_clipboard number", show(system.get_clipboard()))
  rec("set_primary returns", show(system.set_primary_selection("prim")))
  rec("get_primary", show(system.get_primary_selection()))
  rec("set_primary no arg", try(system.set_primary_selection))

  -- fuzzy_match --------------------------------------------------------------
  local fz = {
    { "renderer", "rend" }, { "src/renderer.c", "renderer" }, { "lib/font_render/build.sh", "renderer" },
    { "abc", "abc" }, { "ABC", "abc" }, { "abc", "ABC" }, { "abc", "" }, { "", "" }, { "", "a" },
    { "a b c", "abc" }, { "abc", "a b c" }, { "abc", "cba" }, { "abcabc", "abc" }, { "xaxbxc", "abc" },
    { "hello world", "hw" }, { "hello world", "wh" }, { "  ", "  " }, { "a", "  " }, { "  a", "a" },
    { "foo/bar.lua", "bar" }, { "foo/bar.lua", "lua" }, { "foo/bar.lua", "fbl" },
    { "aaa", "aaaa" }, { "aaaa", "aaa" }, { "a\0b", "ab" }, { "ab", "a\0b" },
    { "Straße", "str" }, { "日本語", "日" }, { "日本語", "語" },
  }
  for i, c in ipairs(fz) do
    rec("fuzzy " .. i .. " fwd", try(system.fuzzy_match, c[1], c[2]))
    rec("fuzzy " .. i .. " files", try(system.fuzzy_match, c[1], c[2], true))
  end
  rec("fuzzy files=false", try(system.fuzzy_match, "src/renderer.c", "renderer", false))
  rec("fuzzy files=1", try(system.fuzzy_match, "src/renderer.c", "renderer", 1))
  rec("fuzzy files='x'", try(system.fuzzy_match, "src/renderer.c", "renderer", "x"))
  rec("fuzzy number args", try(system.fuzzy_match, 123, 2))
  rec("fuzzy one arg", try(system.fuzzy_match, "a"))
  rec("fuzzy no arg", try(system.fuzzy_match))
  rec("fuzzy long", try(system.fuzzy_match, string.rep("ab", 5000), "aabb"))

  -- path_compare -------------------------------------------------------------
  local entries = {
    { "a", "file" }, { "B", "file" }, { "b", "file" }, { "a", "dir" }, { "A", "dir" },
    { "file1", "file" }, { "file2", "file" }, { "file10", "file" }, { "file01", "file" },
    { "dir/x", "file" }, { "dir/y", "file" }, { "dir", "dir" }, { "dir2", "dir" },
    { "dir/sub/z", "file" }, { "dir/sub", "dir" }, { "", "file" }, { "a b", "file" },
    { "10", "file" }, { "9", "file" }, { "a.lua", "file" }, { "a_lua", "file" },
    { "/abs/x", "file" }, { "/abs", "dir" },
  }
  local rows = {}
  for i, p in ipairs(entries) do
    local row = {}
    for j, q in ipairs(entries) do
      local ok, r = pcall(system.path_compare, p[1], p[2], q[1], q[2])
      row[j] = ok and (r and "1" or "0") or "E"
    end
    rows[i] = table.concat(row)
  end
  for i, row in ipairs(rows) do rec(("path_compare row %02d"):format(i), row) end
  rec("path_compare returns", show(system.path_compare("a", "file", "b", "file")))
  rec("path_compare same", show(system.path_compare("a", "file", "a", "file")))
  rec("path_compare big numbers", try(system.path_compare, "f99999999999999999999", "file", "f100000000000000000000", "file"))
  rec("path_compare numbers eq", try(system.path_compare, "f007x", "file", "f7y", "file"))
  rec("path_compare bad type", try(system.path_compare, "a", "weird", "b", "dir"))
  rec("path_compare 3 args", try(system.path_compare, "a", "file", "b"))
  rec("path_compare no args", try(system.path_compare))
  rec("path_compare number args", try(system.path_compare, 1, "file", 2, "file"))
  rec("path_compare nul", try(system.path_compare, "a\0b", "file", "a\0c", "file"))

  system.chdir(start_cwd)
  sh("rm -rf " .. ROOT)
end
