-- Headless contract runtime. Selected with LITE_XL_RUNTIME=contract (see docs/rust-port/PLAN.md §7).
-- Prints deterministic observations; scripts/rust-port/verify.sh diffs them against a recorded baseline.
local M = {}

local suites = (os.getenv("CONTRACT_SUITES") or "smoke"):gmatch("[^,]+")

function M.init()
  local out = {}
  local function rec(key, ...)
    local n, parts = select("#", ...), {}
    for i = 1, n do parts[i] = tostring((select(i, ...))) end
    out[#out + 1] = ("%-44s %s"):format(key, table.concat(parts, " "))
  end
  for name in suites do
    out[#out + 1] = "## " .. name
    local ok, err = pcall(function() require("contract_cases." .. name)(rec) end)
    if not ok then out[#out + 1] = "SUITE ERROR: " .. tostring(err) end
  end
  io.stdout:write(table.concat(out, "\n"), "\n")
  io.stdout:flush()
  os.exit(0)
end

function M.run() end
function M.on_error(err)
  io.stdout:write("HARNESS ERROR: ", tostring(err), "\n")
  os.exit(1)
end

return M
