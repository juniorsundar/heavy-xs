-- core.common.fuzzy_match over lists. Its tie-break used to compare the items with `<`, which
-- raises for table items (autocomplete's `{ text = ..., info = ... }` objects with a __tostring),
-- so completion crashed whenever two suggestions scored equally. This is Lua only; the scoring
-- itself (system.fuzzy_match) is pinned by the `system` suite.
local common = require "core.common"

local mt = { __tostring = function(t) return t.text end }
local function obj(text) return setmetatable({ text = text, info = "normal" }, mt) end
local function objs(...) local r = {} for i, t in ipairs({ ... }) do r[i] = obj(t) end return r end

local function names(list)
  local out = {}
  for i, v in ipairs(list) do out[i] = type(v) == "table" and v.text or v end
  return table.concat(out, ",")
end

local function try(fn, ...)
  local ok, res = pcall(fn, ...)
  return ok and names(res) or "ERR"
end

return function(rec)
  -- String items: this path must never change.
  rec("strings, equal scores", try(common.fuzzy_match, { "abe", "abc", "abd" }, "ab"))
  rec("strings, distinct scores", try(common.fuzzy_match, { "axbxc", "abc", "a_b_c" }, "abc"))
  rec("strings, no match", try(common.fuzzy_match, { "xyz", "qrs" }, "ab"))
  rec("strings, empty needle", try(common.fuzzy_match, { "b", "a", "c" }, ""))
  rec("strings, files mode", try(common.fuzzy_match, { "src/main.c", "main.c", "docs/main.md" }, "main", true))
  rec("strings, empty list", try(common.fuzzy_match, {}, "a"))
  rec("strings, duplicates", try(common.fuzzy_match, { "ab", "ab", "ab" }, "ab"))

  -- Table items with __tostring: used to raise whenever two scored the same.
  rec("objects, equal scores", try(common.fuzzy_match, objs("abe", "abc", "abd"), "ab"))
  rec("objects, distinct scores", try(common.fuzzy_match, objs("axbxc", "abc", "a_b_c"), "abc"))
  rec("objects, no match", try(common.fuzzy_match, objs("xyz", "qrs"), "ab"))
  rec("objects, empty needle", try(common.fuzzy_match, objs("b", "a", "c"), ""))
  rec("objects, duplicate text", try(common.fuzzy_match, objs("ab", "ab", "ab"), "ab"))
  rec("objects, many equal", try(common.fuzzy_match, objs("v9", "v1", "v5", "v3", "v7", "v2"), "v"))

  -- The result holds the caller's own objects, not copies (autocomplete reads .info from them).
  local items = objs("abe", "abc")
  local res = common.fuzzy_match(items, "ab")
  rec("objects keep identity", tostring(rawequal(res[1], items[2]) and rawequal(res[2], items[1])))
  rec("objects keep fields", tostring(res[1].info))

  -- Mixed strings and objects sort together by their text.
  rec("mixed", try(common.fuzzy_match, { "abe", obj("abc"), "abd" }, "ab"))

  -- A larger set with many ties, as completion produces.
  local many = {}
  for i = 1, 200 do many[i] = obj(string.format("sym_%03d", (i * 37) % 200)) end
  local sorted = common.fuzzy_match(many, "sym")
  local ordered = true
  for i = 2, #sorted do if sorted[i - 1].text > sorted[i].text then ordered = false end end
  rec("200 tied objects", #sorted .. " " .. tostring(ordered) .. " " .. sorted[1].text .. " " .. sorted[#sorted].text)
end
