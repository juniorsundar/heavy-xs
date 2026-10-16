-- Proves the harness runs and that every API module is loadable. Rust-only probe is optional.
return function(rec)
  for _, name in ipairs { "system", "renderer", "renwindow", "regex", "process", "dirmonitor", "utf8extra" } do
    rec("type(" .. name .. ")", type(_G[name]))
  end
  rec("PLATFORM", PLATFORM)
  rec("type(ARGS)", type(ARGS))
  rec("rust_probe.ping()", rust_probe and rust_probe.ping() or "absent")
end
