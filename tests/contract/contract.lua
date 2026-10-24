-- Headless contract runtime. Selected with LITE_XL_RUNTIME=contract (see docs/architecture/rust-port/PLAN.md §7).
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
    -- Keep baselines text-only: raw control bytes would make diff call them binary.
    local value = table.concat(parts, " ")
    value = value:gsub("[%z\1-\8\11\12\14-\31\127]", function(c)
      return ("\\x%02X"):format(c:byte())
    end)
    emit(("%-44s %s"):format(key, value))
  end

  local failed = false
  for name in (os.getenv("CONTRACT_SUITES") or "smoke"):gmatch("[^,]+") do
    emit("## " .. name)
    local ok, err = pcall(function() require("contract_cases." .. name)(rec) end)
    if not ok then
      emit("SUITE ERROR: " .. tostring(err))
      failed = true
    end
  end
  -- A suite that aborts must never be recordable as a baseline.
  os.exit(failed and 2 or 0)
end

function M.run() end
function M.on_error(err)
  io.stdout:write("HARNESS ERROR: ", tostring(err), "\n")
  io.stdout:flush()
  os.exit(1)
end

return M
