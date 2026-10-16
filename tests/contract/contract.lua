-- Headless contract runtime. Selected with LITE_XL_RUNTIME=contract (see docs/rust-port/PLAN.md §7).
-- Prints deterministic observations; scripts/rust-port/verify.sh diffs them against a recorded baseline.
local M = {}

function M.init()
  -- Flush per line so a crash still shows how far the suite got.
  local function emit(line)
    io.stdout:write(line, "\n")
    io.stdout:flush()
  end

  local function rec(key, ...)
    local n, parts = select("#", ...), {}
    for i = 1, n do parts[i] = tostring((select(i, ...))) end
    emit(("%-44s %s"):format(key, table.concat(parts, " ")))
  end

  for name in (os.getenv("CONTRACT_SUITES") or "smoke"):gmatch("[^,]+") do
    emit("## " .. name)
    local ok, err = pcall(function() require("contract_cases." .. name)(rec) end)
    if not ok then emit("SUITE ERROR: " .. tostring(err)) end
  end
  os.exit(0)
end

function M.run() end
function M.on_error(err)
  io.stdout:write("HARNESS ERROR: ", tostring(err), "\n")
  io.stdout:flush()
  os.exit(1)
end

return M
