-- The real syntax/tokenizer path, which is regex heavy
-- (data/core/tokenizer.lua uses compiled patterns, gmatch and check_pattern).
require("core.regex")
local syntax = require("core.syntax")
local tokenizer = require "core.tokenizer"
require("plugins.language_lua")

return function(rec)
  local syn = syntax.get("test.lua", "")
  rec("syntax found", syn and syn.name or "nil")

  -- tokenizer.each_token yields (index, type, text).
  local function tokens(text, state)
    local t, newstate = tokenizer.tokenize(syn, text, state or "")
    local out = {}
    for _, ttype, ttext in tokenizer.each_token(t) do
      out[#out + 1] = ttype .. ":" .. ttext:gsub("\n", "\\n")
    end
    return table.concat(out, " "), newstate
  end

  rec("tokens: local", tokens("local x = 1"))
  rec("tokens: string+comment", tokens('local s = "hi" -- c'))
  rec("tokens: comment", tokens("-- just a comment"))
  rec("tokens: block comment", tokens("--[[ block ]]"))
  rec("tokens: function", tokens("function f(a) return a end"))
  rec("tokens: number", tokens("x = 0x1f + 2.5e3"))
  rec("tokens: table", tokens("local t = { [1] = 'a' }"))
  rec("tokens: long string", tokens("s = [[abc]]"))
  rec("tokens: bad number", tokens("x = 1e"))
  rec("tokens: unterminated", tokens('s = "abc'))

  -- Thread the returned state across lines, like Highlighter does.
  local st = ""
  local all = {}
  for _, line in ipairs({ "local a = 1", "--[[ start", "still inside ]]", "local b = 2" }) do
    local out
    out, st = tokens(line, st)
    all[#all + 1] = out
  end
  rec("tokens: multiline state", table.concat(all, " / "))
end
