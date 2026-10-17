-- Observational checks for the current implementation, not replacement acceptance criteria.
-- Copy into a temporary LITE_USERDIR and run as LITE_XL_RUNTIME (see regex-contract.md).
local M = {}

function M.init()
  require "core.regex"
  local function check(name, expected, fn)
    local result = table.pack(pcall(fn))
    local parts = { result[1] and "ok" or "error" }
    for i = 2, result.n do
      local v = result[i]
      if not result[1] then
        v = tostring(v):gsub("\n.*", ""):gsub("^runtime error: ", "")
      elseif type(v) == "string" then
        v = string.format("%q", v)
      else
        v = tostring(v)
      end
      parts[#parts + 1] = v
    end
    local actual = table.concat(parts, " | ")
    io.stdout:write(name, ": ", actual, "\n")
    io.stdout:flush()
    if expected then assert(actual == expected, name .. ": unexpected observation") end
  end

  check("UTF byte offsets", "ok | 1 | 3", function()
    return regex.cmatch(regex.compile("."), "é")
  end)
  check("UTF inclusive ends", "ok | 1 | 2", function()
    return regex.find_offsets(".", "é")
  end)
  check("UTF word class without UCP", "ok", function()
    return regex.cmatch(regex.compile([[\w]]), "é")
  end)
  check("invalid UTF subject", nil, function()
    return regex.cmatch(regex.compile("."), "\255")
  end)
  check("dotall flag", "ok | 1 | 2", function()
    return regex.cmatch(regex.compile(".", "s"), "\n")
  end)
  check("unknown compile flag ignored", "ok | 1 | 2", function()
    return regex.cmatch(regex.compile("a", "z"), "a")
  end)
  check("explicit nil compile flag", nil, function()
    return regex.compile("a", nil)
  end)
  check("cmatch sees suffix as subject", "ok | 2 | 3", function()
    return regex.cmatch(regex.compile("^a"), "ba", 2)
  end)
  check("gmatch retains original subject", "ok", function()
    return regex.gmatch(regex.compile("^a"), "ba", 2)()
  end)
  check("cmatch lookbehind loses prefix", "ok", function()
    return regex.cmatch(regex.compile("(?<=b)a"), "ba", 2)
  end)
  check("gmatch lookbehind retains prefix", 'ok | "a"', function()
    return regex.gmatch("(?<=b)a", "ba", 2)()
  end)
  check("empty gmatch repeats position", "ok | 2 | 2", function()
    local iterator = regex.gmatch("(?=a)", "ba")
    -- Bounded calls: a generic for-loop here would not make progress.
    return iterator(), iterator()
  end)
  check("limited empty substitution", 'ok | "--abc" | 2', function()
    return regex.gsub("x*", "abc", "-", 2)
  end)
  check("negative limit means global", 'ok | "xx" | 2', function()
    return regex.gsub("a", "aa", "x", -1)
  end)
  check("named replacement", 'ok | "a" | 1', function()
    return regex.gsub("(?<letter>a)", "a", "${letter}")
  end)
  check("dollar and capture replacement", 'ok | "$a" | 1', function()
    return regex.gsub("(a)", "a", "$$$1")
  end)
  check("Lua replacement syntax is literal", 'ok | "%1" | 1', function()
    return regex.gsub("(a)", "a", "%1")
  end)
  check("replacement function rejected", nil, function()
    return regex.gsub("a", "a", function() return "b" end)
  end)
  check("atomic and possessive constructs", 'ok | "aa"', function()
    return regex.match("(?>a++)", "aa")
  end)
  check("unmatched trailing group omitted", "ok | 1 | 2 | 1 | 2", function()
    return regex.cmatch(regex.compile("(a)(b)?"), "a")
  end)
  print("OK: 20 bounded observations (17 asserted; 3 error messages observational)")
  os.exit(0)
end

function M.run() end
return M
