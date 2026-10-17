-- So that in addition to regex.gsub(pattern, string), we can also do
-- pattern:gsub(string).
regex.__index = function(table, key) return regex[key]; end

---Looks for the first match of `pattern` in the string `str`.
---If it finds a match, it returns the indices of `str` where this occurrence
---starts and ends; otherwise, it returns `nil`.
---If the pattern has captures, the captured start and end indexes are returned,
---after the two initial ones.
---
---@param pattern string|table The regex pattern to use, either as a simple string or precompiled.
---@param str string The string to search for valid matches.
---@param offset? integer The position on the subject to start searching.
---@param options? integer 0 or regex.ANCHORED (ENDANCHORED is an alias).
---
---@return integer? start Offset where the first match was found; `nil` if no match.
---@return integer? end Offset where the first match ends; `nil` if no match.
---@return integer? ... #Captured matches offsets.
regex.find_offsets = function(pattern, str, offset, options)
  if type(pattern) ~= "table" then
    local err
    pattern, err = regex.compile(pattern)
    if not pattern then error(err, 2) end
  end
  local res = table.pack(regex.cmatch(pattern, str, offset or 1, options or 0))
  for i = 2,res.n,2 do
    if res[i] then res[i] = res[i] - 1 end
  end
  return table.unpack(res, 1, res.n)
end

---Behaves like `string.match`.
---Looks for the first match of `pattern` in the string `str`.
---If it finds a match, it returns the matched string; otherwise, it returns `nil`.
---If the pattern has captures, only the captured strings are returned.
---If a capture is empty, its offset is returned instead.
---
---@param pattern string|table The regex pattern to use, either as a simple string or precompiled.
---@param str string The string to search for valid matches.
---@param offset? integer The position on the subject to start searching.
---@param options? integer 0 or regex.ANCHORED (ENDANCHORED is an alias).
---
---@return (string|integer)? ... #List of captured matches; the entire match if no matches were specified; if the match is empty, its offset is returned instead.
regex.match = function(pattern, str, offset, options)
  local res = table.pack(regex.find(pattern, str, offset, options))
  if res.n == 0 then return end
  if res.n > 2 then return table.unpack(res, 3, res.n) end
  return string.sub(str, res[1], res[2])
end

---Behaves like `string.find`.
---Looks for the first match of `pattern` in the string `str`.
---If it finds a match, it returns the indices of `str` where this occurrence
---starts and ends; otherwise, it returns `nil`.
---If the pattern has captures, the captured strings are returned,
---after the two indexes ones.
---If a capture is empty, its offset is returned instead.
---
---@param pattern string|table The regex pattern to use, either as a simple string or precompiled.
---@param str string The string to search for valid matches.
---@param offset? integer The position on the subject to start searching.
---@param options? integer 0 or regex.ANCHORED (ENDANCHORED is an alias).
---
---@return integer? start Offset where the first match was found; `nil` if no match.
---@return integer? end Offset where the first match ends; `nil` if no match.
---@return (string|integer)? ... #List of captured matches; if the match is empty, its offset is returned instead.
regex.find = function(pattern, str, offset, options)
  local res = table.pack(regex.find_offsets(pattern, str, offset, options))
  if res.n == 0 then return end
  local out = { n = 2, res[1], res[2] }
  for i = 3,res.n,2 do
    out.n = out.n + 1
    if res[i] ~= nil then
      out[out.n] = res[i] > res[i+1] and res[i] or string.sub(str, res[i], res[i+1])
    end
  end
  return table.unpack(out, 1, out.n)
end
