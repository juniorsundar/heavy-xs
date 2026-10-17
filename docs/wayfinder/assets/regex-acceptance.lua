-- Bounded acceptance-evidence runtime; see regex-acceptance.md for reproduction.
-- Reference mode records retained behavior and bounded old iterator observations.
-- Candidate mode instead checks the explicitly approved iterator corrections.
local M = {}

function M.init()
  local mode = os.getenv("REGEX_CORPUS_MODE") or "reference"
  assert(mode == "reference" or mode == "candidate", "unknown corpus mode")
  local total, failures, seen = 0, 0, {}
  local function render(values)
    local parts = {}
    for i = 1, values.n do
      local value = values[i]
      parts[i] = type(value) == "string"
        and string.format("%q", value):gsub("\\\n", "\\n") or tostring(value)
    end
    return table.concat(parts, " | ")
  end
  local function check(id, expected, fn)
    assert(not seen[id], "duplicate case: " .. id)
    seen[id], total = true, total + 1
    local result = table.pack(pcall(fn))
    local actual = { n = result.n - 1 }
    table.move(result, 2, result.n, 1, actual)
    local pass = result[1] and actual.n == expected.n
    for i = 1, expected.n do
      pass = pass and type(actual[i]) == type(expected[i]) and actual[i] == expected[i]
    end
    if not pass then failures = failures + 1 end
    print((pass and "PASS " or "FAIL ") .. id .. ":" .. (actual.n > 0 and " " .. render(actual) or ""))
    if not pass then print("  expected:" .. (expected.n > 0 and " " .. render(expected) or "")) end
  end

  require("regex-acceptance-cases")(check)
  require("regex-replacement-cases")(check)

  if mode == "reference" then
    check("OBS_iterator_last_capture_overlap", table.pack("a", "a", "a", 0), function()
      local iter = regex.gmatch("(a).*", "aaa")
      local a, b, c = iter(), iter(), iter()
      return a, b, c, select("#", iter())
    end)
    check("OBS_iterator_zero_width_repeats", table.pack(2, 2), function()
      local iter = regex.gmatch("(?=a)", "ba")
      return iter(), iter()
    end)
  else
    check("FIX_iterator_whole_match_end", table.pack("a", 0), function()
      local iter = regex.gmatch("(a).*", "aaa")
      local value = iter()
      return value, select("#", iter())
    end)
    check("FIX_iterator_zero_width_once", table.pack(2, 0), function()
      local iter = regex.gmatch("(?=a)", "ba")
      local value = iter()
      return value, select("#", iter())
    end)
    check("FIX_iterator_multibyte_zero_width_once", table.pack(3, 0), function()
      local iter = regex.gmatch("(?=a)", "éa")
      local value = iter()
      return value, select("#", iter())
    end)
    check("FIX_iterator_eof_zero_width_once", table.pack(3, 0), function()
      local iter = regex.gmatch("(?=$)", "é")
      local value = iter()
      return value, select("#", iter())
    end)
    check("FIX_iterator_utf_progress_properties", table.pack(true), function()
      local iter = regex.gmatch("()", "éa")
      local boundaries, previous, yielded = { [1] = true, [3] = true, [4] = true }, 0, false
      -- This is a finite property check, not a chosen empty-match retry algorithm.
      for _ = 1, 5 do
        local result = table.pack(iter())
        if result.n == 0 then return yielded end
        local position = result[1]
        if result.n ~= 1 or not boundaries[position] or position <= previous then return false end
        previous, yielded = position, true
      end
      return false
    end)
  end
  print(string.format("SUMMARY mode=%s cases=%d failures=%d", mode, total, failures))
  os.exit(failures == 0 and 0 or 1)
end

function M.run() end
return M
